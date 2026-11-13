use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio::process::Command;

use crate::blueprint::schema::{ReadyCheck, ServiceBlueprint};
use crate::blueprint::{parse_blueprint, render_template, resolve_blueprint_args};
use crate::drivers::DriverError;

pub struct BlueprintDriver;

impl BlueprintDriver {
    /// Loads a blueprint from the runtime directory.
    pub fn load_blueprint(runtime_dir: &Path) -> Result<ServiceBlueprint, DriverError> {
        let manifest_path = runtime_dir.join("rakit-manifest.toml");
        if !manifest_path.exists() {
            return Err(DriverError::Config(format!(
                "rakit-manifest.toml missing in {}",
                runtime_dir.display()
            )));
        }

        let toml_str = fs::read_to_string(&manifest_path).map_err(DriverError::Io)?;
        parse_blueprint(&toml_str).map_err(|e| DriverError::Config(e.to_string()))
    }

    /// Checks if bootstrap is needed and executes the bootstrap command.
    pub async fn run_bootstrap_if_needed(
        blueprint: &ServiceBlueprint,
        vars: &HashMap<String, String>,
    ) -> Result<(), DriverError> {
        if let Some(check_dir_tmpl) = &blueprint.lifecycle.bootstrap_check_dir {
            let rendered_check_dir = render_template(check_dir_tmpl, vars)
                .map_err(|e| DriverError::Config(e.to_string()))?;
            let check_path = PathBuf::from(rendered_check_dir);

            if !check_path.exists() {
                if let Some(bootstrap_args) = &blueprint.lifecycle.bootstrap_cmd {
                    let resolved = resolve_blueprint_args(bootstrap_args, vars)
                        .map_err(|e| DriverError::Config(e.to_string()))?;

                    if resolved.is_empty() {
                        return Ok(());
                    }

                    let program = &resolved[0];
                    let args = &resolved[1..];

                    let mut cmd = Command::new(program);
                    cmd.args(args);

                    let status = cmd.status().await.map_err(DriverError::Io)?;
                    if !status.success() {
                        return Err(DriverError::Config(format!(
                            "Bootstrap command failed with status: {:?}",
                            status.code()
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Compiles a `tokio::process::Command` from blueprint binaries and start args.
    pub fn compile_start_command(
        blueprint: &ServiceBlueprint,
        bin_dir: &Path,
        vars: &HashMap<String, String>,
    ) -> Result<Command, DriverError> {
        let daemon_rel = &blueprint.binaries.daemon;
        let daemon_path = bin_dir.join(daemon_rel);

        let resolved_args = resolve_blueprint_args(&blueprint.lifecycle.start_args, vars)
            .map_err(|e| DriverError::Config(e.to_string()))?;

        let mut cmd = Command::new(&daemon_path);
        cmd.args(&resolved_args);

        #[cfg(unix)]
        {
            cmd.process_group(0);
        }

        Ok(cmd)
    }

    /// Polls ready check until timeout or success.
    pub async fn wait_for_ready(
        ready_check: &ReadyCheck,
        vars: &HashMap<String, String>,
    ) -> Result<(), DriverError> {
        let timeout_secs = ready_check.timeout_secs();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
        let poll_interval = Duration::from_millis(200);

        match ready_check {
            ReadyCheck::Tcp { .. } => {
                let port_str = vars.get("port").ok_or_else(|| {
                    DriverError::Config("Missing port variable for TCP ready check".to_string())
                })?;
                let bind_addr = vars
                    .get("bind_address")
                    .cloned()
                    .unwrap_or_else(|| "127.0.0.1".to_string());
                let addr = format!("{}:{}", bind_addr, port_str);

                while tokio::time::Instant::now() < deadline {
                    if let Ok(_stream) = TcpStream::connect(&addr).await {
                        return Ok(());
                    }
                    tokio::time::sleep(poll_interval).await;
                }
                Err(DriverError::Config(format!(
                    "Timed out waiting for TCP port {} to become ready after {}s",
                    addr, timeout_secs
                )))
            }
            ReadyCheck::UnixSocket { .. } => {
                let socket_str = vars.get("socket_path").ok_or_else(|| {
                    DriverError::Config(
                        "Missing socket_path variable for UnixSocket ready check".to_string(),
                    )
                })?;
                let sock_path = PathBuf::from(socket_str);

                #[cfg(unix)]
                {
                    while tokio::time::Instant::now() < deadline {
                        if sock_path.exists() {
                            if let Ok(_stream) = UnixStream::connect(&sock_path).await {
                                return Ok(());
                            }
                        }
                        tokio::time::sleep(poll_interval).await;
                    }
                    Err(DriverError::Config(format!(
                        "Timed out waiting for Unix socket {} to become ready after {}s",
                        sock_path.display(),
                        timeout_secs
                    )))
                }
                #[cfg(not(unix))]
                {
                    // Fallback on non-unix if socket is requested
                    let _ = sock_path;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    Ok(())
                }
            }
            ReadyCheck::LogRegex { .. } => {
                // Short grace interval for log regex (supervised via stdout reader)
                tokio::time::sleep(Duration::from_millis(500)).await;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_start_command() {
        let toml_str = r#"
[engine]
name = "nginx"
category = "web_server"
version = "1.26.2"

[binaries]
daemon = "sbin/nginx"

[lifecycle]
start_args = ["-c", "{{config_file}}", "-g", "daemon off;"]
ready_check = { kind = "tcp", timeout_secs = 5 }
"#;
        let blueprint = parse_blueprint(toml_str).unwrap();
        let bin_dir = Path::new("/runtime/bin/web_server/nginx/1.26.2");

        let mut vars = HashMap::new();
        vars.insert(
            "config_file".to_string(),
            "/runtime/config/nginx/nginx.conf".to_string(),
        );

        let cmd = BlueprintDriver::compile_start_command(&blueprint, bin_dir, &vars).unwrap();
        let prog = cmd.as_std().get_program().to_string_lossy().to_string();
        assert!(prog.ends_with("sbin/nginx"));

        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert_eq!(args[0], "-c");
        assert_eq!(args[1], "/runtime/config/nginx/nginx.conf");
        assert_eq!(args[2], "-g");
        assert_eq!(args[3], "daemon off;");
    }
}
