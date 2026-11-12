use super::{DriverError, RuntimePaths, ServiceDriver};
use crate::models::{ProjectSite, ServiceCategory, ServiceInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct GenericDriver {
    pub engine_name: &'static str,
    pub category: ServiceCategory,
}

#[async_trait]
impl ServiceDriver for GenericDriver {
    fn category(&self) -> ServiceCategory {
        self.category
    }

    fn engine(&self) -> &'static str {
        self.engine_name
    }

    async fn prepare_config(
        &self,
        instance: &ServiceInstance,
        paths: &RuntimePaths,
        _sites: &[ProjectSite],
    ) -> Result<PathBuf, DriverError> {
        let conf_dir = paths.config.join(self.engine_name);
        std::fs::create_dir_all(&conf_dir)?;
        let conf_file = conf_dir.join(format!("{}.conf", instance.instance_id));
        if !conf_file.exists() {
            std::fs::write(&conf_file, "# Generic service configuration\n")?;
        }
        Ok(conf_file)
    }

    fn compile_command(
        &self,
        instance: &ServiceInstance,
        binary_path: &Path,
        _config_path: &Path,
        _paths: &RuntimePaths,
    ) -> Result<Command, DriverError> {
        let mut cmd = Command::new(binary_path);
        if let Some(port) = instance.port {
            if self.engine_name == "redis" {
                cmd.arg("--port").arg(port.to_string());
            }
        }
        Ok(cmd)
    }
}
