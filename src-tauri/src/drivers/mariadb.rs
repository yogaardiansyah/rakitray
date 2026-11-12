use super::{DriverError, RuntimePaths, ServiceDriver};
use crate::models::{ProjectSite, ServiceCategory, ServiceInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct MariaDbDriver;

#[async_trait]
impl ServiceDriver for MariaDbDriver {
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Database
    }

    fn engine(&self) -> &'static str {
        "mariadb"
    }

    async fn prepare_config(
        &self,
        instance: &ServiceInstance,
        paths: &RuntimePaths,
        _sites: &[ProjectSite],
    ) -> Result<PathBuf, DriverError> {
        let conf_dir = paths.config.join("mariadb");
        let data_dir = paths.data.join(&instance.instance_id);
        std::fs::create_dir_all(&conf_dir)?;
        std::fs::create_dir_all(&data_dir)?;

        let conf_file = conf_dir.join(format!("{}.cnf", instance.instance_id));
        let port = instance.port.unwrap_or(3306);
        let pid_file = paths.tmp.join(format!("{}.pid", instance.instance_id));
        let socket_file = paths.tmp.join(format!("{}.sock", instance.instance_id));

        let cnf = format!(
            r#"[mysqld]
port = {}
datadir = {}
pid-file = {}
socket = {}
bind-address = 127.0.0.1
log-error = {}
default_storage_engine = InnoDB
innodb_buffer_pool_size = 128M
"#,
            port,
            data_dir.display(),
            pid_file.display(),
            socket_file.display(),
            paths
                .logs
                .join(format!("{}.log", instance.instance_id))
                .display()
        );

        std::fs::write(&conf_file, cnf)?;
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
        let data_dir = paths.data.join(&instance.instance_id);

        cmd.arg(format!("--defaults-file={}", config_path.display()))
            .arg(format!("--datadir={}", data_dir.display()))
            .arg(format!("--port={}", instance.port.unwrap_or(3306)));

        Ok(cmd)
    }
}
