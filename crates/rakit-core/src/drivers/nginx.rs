use super::{DriverError, RuntimePaths, ServiceDriver};
use crate::models::{ProjectSite, ServiceCategory, ServiceInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct NginxDriver;

#[async_trait]
impl ServiceDriver for NginxDriver {
    fn category(&self) -> ServiceCategory {
        ServiceCategory::WebServer
    }

    fn engine(&self) -> &'static str {
        "nginx"
    }

    async fn prepare_config(
        &self,
        instance: &ServiceInstance,
        paths: &RuntimePaths,
        sites: &[ProjectSite],
    ) -> Result<PathBuf, DriverError> {
        let conf_dir = paths.config.join("nginx");
        std::fs::create_dir_all(&conf_dir)?;
        let conf_file = conf_dir.join(format!("{}.conf", instance.instance_id));

        let port = instance.port.unwrap_or(8088);
        let pid_file = paths.tmp.join(format!("{}.pid", instance.instance_id));
        let error_log = paths
            .logs
            .join(format!("{}_error.log", instance.instance_id));
        let access_log = paths
            .logs
            .join(format!("{}_access.log", instance.instance_id));

        let mut site_blocks = String::new();
        if sites.is_empty() {
            let default_www = paths.www.join("default");
            std::fs::create_dir_all(&default_www)?;
            let index_file = default_www.join("index.php");
            if !index_file.exists() {
                let _ = std::fs::write(&index_file, "<?php phpinfo(); ?>");
            }

            site_blocks.push_str(&format!(
                r#"
    server {{
        listen {};
        server_name localhost;
        root {};
        index index.php index.html;

        location / {{
            try_files $uri $uri/ /index.php?$query_string;
        }}

        location ~ \.php$ {{
            fastcgi_pass 127.0.0.1:9000;
            fastcgi_index index.php;
            include fastcgi_params;
            fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        }}
    }}
"#,
                port,
                default_www.display()
            ));
        } else {
            for site in sites {
                site_blocks.push_str(&format!(
                    r#"
    server {{
        listen {};
        server_name {};
        root {};
        index index.php index.html;

        location / {{
            try_files $uri $uri/ /index.php?$query_string;
        }}

        location ~ \.php$ {{
            fastcgi_pass 127.0.0.1:{};
            fastcgi_index index.php;
            include fastcgi_params;
            fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        }}
    }}
"#,
                    site.port,
                    site.domain,
                    site.root_path,
                    if site.runtime_id.contains("8.2") {
                        9002
                    } else {
                        9000
                    }
                ));
            }
        }

        let nginx_conf = format!(
            r#"worker_processes 1;
pid {};
error_log {};

events {{
    worker_connections 1024;
}}

http {{
    include mime.types;
    default_type application/octet-stream;
    access_log {};
    sendfile on;
    keepalive_timeout 65;

{}
}}
"#,
            pid_file.display(),
            error_log.display(),
            access_log.display(),
            site_blocks
        );

        std::fs::write(&conf_file, nginx_conf)?;
        Ok(conf_file)
    }

    fn compile_command(
        &self,
        instance: &ServiceInstance,
        binary_path: &Path,
        config_path: &Path,
        paths: &RuntimePaths,
    ) -> Result<Command, DriverError> {
        let mut cmd = Command::new(binary_path);
        let port = instance.port.unwrap_or(8088);

        cmd.current_dir(&paths.root)
            .arg("-c")
            .arg(config_path)
            .arg("-p")
            .arg(port.to_string())
            .arg("-g")
            .arg("daemon off;");

        Ok(cmd)
    }
}
