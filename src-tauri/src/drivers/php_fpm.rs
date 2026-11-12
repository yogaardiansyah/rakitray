use super::{DriverError, RuntimePaths, ServiceDriver};
use crate::models::{ProjectSite, ServiceCategory, ServiceInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct PhpFpmDriver;

#[async_trait]
impl ServiceDriver for PhpFpmDriver {
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Runtime
    }

    fn engine(&self) -> &'static str {
        "php-cgi"
    }

    async fn prepare_config(
        &self,
        instance: &ServiceInstance,
        paths: &RuntimePaths,
        _sites: &[ProjectSite],
    ) -> Result<PathBuf, DriverError> {
        let conf_dir = paths.config.join("php");
        std::fs::create_dir_all(&conf_dir)?;
        let conf_file = conf_dir.join(format!("{}.ini", instance.instance_id));

        let is_dev = instance.profile != "prod";
        let php_ini = format!(
            r#"[PHP]
engine = On
short_open_tag = Off
precision = 14
output_buffering = 4096
zlib.output_compression = Off
implicit_flush = Off
zend.enable_gc = On

max_execution_time = 120
max_input_time = 60
memory_limit = 512M

error_reporting = E_ALL
display_errors = {}
display_startup_errors = {}
log_errors = On
error_log = {}

variables_order = "GPCS"
request_order = "GP"
register_argc_argv = Off
auto_globals_jit = On

post_max_size = 64M
file_uploads = On
upload_max_filesize = 64M
max_file_uploads = 20

default_mimetype = "text/html"
default_charset = "UTF-8"

enable_dl = Off

[opcache]
opcache.enable = 1
opcache.memory_consumption = 128
opcache.interned_strings_buffer = 8
opcache.max_accelerated_files = 10000
opcache.revalidate_freq = {}
"#,
            if is_dev { "On" } else { "Off" },
            if is_dev { "On" } else { "Off" },
            paths
                .logs
                .join(format!("{}.log", instance.instance_id))
                .display(),
            if is_dev { "0" } else { "60" }
        );

        std::fs::write(&conf_file, php_ini)?;
        Ok(conf_file)
    }

    fn compile_command(
        &self,
        instance: &ServiceInstance,
        binary_path: &Path,
        config_path: &Path,
        _paths: &RuntimePaths,
    ) -> Result<Command, DriverError> {
        let mut cmd = Command::new(binary_path);
        let port = instance.port.unwrap_or(9000);
        let bind_addr = format!("127.0.0.1:{}", port);

        cmd.arg("-b").arg(&bind_addr).arg("-c").arg(config_path);

        cmd.env("PHP_FCGI_CHILDREN", "4");
        cmd.env("PHP_FCGI_MAX_REQUESTS", "1000");

        Ok(cmd)
    }
}
