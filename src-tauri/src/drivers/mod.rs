use crate::models::{ProjectSite, ServiceCategory, ServiceInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    Config(String),
}

pub struct RuntimePaths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
    pub logs: PathBuf,
    pub tmp: PathBuf,
    pub www: PathBuf,
}

impl RuntimePaths {
    pub fn new(root: PathBuf) -> Self {
        Self {
            config: root.join("config"),
            data: root.join("data"),
            logs: root.join("logs"),
            tmp: root.join("tmp"),
            www: root.join("www"),
            root,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config)?;
        std::fs::create_dir_all(&self.data)?;
        std::fs::create_dir_all(&self.logs)?;
        std::fs::create_dir_all(&self.tmp)?;
        std::fs::create_dir_all(&self.www)?;
        Ok(())
    }
}

#[async_trait]
pub trait ServiceDriver: Send + Sync {
    fn category(&self) -> ServiceCategory;
    fn engine(&self) -> &'static str;
    async fn prepare_config(
        &self,
        instance: &ServiceInstance,
        paths: &RuntimePaths,
        sites: &[ProjectSite],
    ) -> Result<PathBuf, DriverError>;
    fn compile_command(
        &self,
        instance: &ServiceInstance,
        binary_path: &Path,
        config_path: &Path,
        paths: &RuntimePaths,
    ) -> Result<Command, DriverError>;
}

pub mod generic;
pub mod mariadb;
pub mod nginx;
pub mod php_fpm;
