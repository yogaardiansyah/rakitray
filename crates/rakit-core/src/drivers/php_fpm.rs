use super::{DriverError, RuntimePaths, ServiceDriver};
use crate::models::{
    PhpIniDirective, PhpIniLimits, PhpProfileMode, ProjectSite, ServiceCategory, ServiceInstance,
};
use async_trait::async_trait;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedIni {
    pub ini: String,
    pub warnings: Vec<String>,
}

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
        let conf_file = conf_dir.join(format!("{}-generated.ini", instance.version));

        if !conf_file.exists() {
            let profile = if instance.profile == "prod" {
                PhpProfileMode::Prod
            } else {
                PhpProfileMode::Dev
            };

            let default_limits = default_limits_for_profile(profile);
            let rendered = render_ini(&instance.version, profile, &default_limits, &[], None)?;

            std::fs::write(&conf_file, rendered.ini)?;
        }

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
        let port = instance.port.unwrap_or(9000);
        let bind_addr = format!("127.0.0.1:{}", port);

        cmd.arg("-b").arg(&bind_addr).arg("-c").arg(config_path);

        cmd.env("PHP_FCGI_CHILDREN", "4");
        cmd.env("PHP_FCGI_MAX_REQUESTS", "1000");
        cmd.env("PHP_INI_SCAN_DIR", paths.config.join("php"));

        Ok(cmd)
    }
}

pub fn default_limits_for_profile(profile: PhpProfileMode) -> PhpIniLimits {
    match profile {
        PhpProfileMode::Dev => PhpIniLimits {
            memory_limit: "512M".to_string(),
            upload_max_filesize: "64M".to_string(),
            post_max_size: "64M".to_string(),
            max_execution_time: 120,
        },
        PhpProfileMode::Prod => PhpIniLimits {
            memory_limit: "1024M".to_string(),
            upload_max_filesize: "64M".to_string(),
            post_max_size: "64M".to_string(),
            max_execution_time: 60,
        },
    }
}

pub fn default_directives_for_profile(
    profile: PhpProfileMode,
    limits: &PhpIniLimits,
) -> Vec<PhpIniDirective> {
    let is_dev = profile == PhpProfileMode::Dev;
    vec![
        // Limits & Uploads
        PhpIniDirective {
            key: "memory_limit".to_string(),
            value: limits.memory_limit.clone(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "max_execution_time".to_string(),
            value: limits.max_execution_time.to_string(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "max_input_time".to_string(),
            value: "60".to_string(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "max_input_vars".to_string(),
            value: "1000".to_string(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "output_buffering".to_string(),
            value: "4096".to_string(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "file_uploads".to_string(),
            value: "On".to_string(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "upload_max_filesize".to_string(),
            value: limits.upload_max_filesize.clone(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "post_max_size".to_string(),
            value: limits.post_max_size.clone(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "max_file_uploads".to_string(),
            value: "20".to_string(),
            enabled: true,
            section: "Limits & Uploads".to_string(),
            custom: false,
        },
        // Error & Logging
        PhpIniDirective {
            key: "error_reporting".to_string(),
            value: if is_dev {
                "E_ALL".to_string()
            } else {
                "E_ALL & ~E_DEPRECATED & ~E_STRICT".to_string()
            },
            enabled: true,
            section: "Error & Logging".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "display_errors".to_string(),
            value: if is_dev {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            enabled: true,
            section: "Error & Logging".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "display_startup_errors".to_string(),
            value: if is_dev {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            enabled: true,
            section: "Error & Logging".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "log_errors".to_string(),
            value: "On".to_string(),
            enabled: true,
            section: "Error & Logging".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "ignore_repeated_errors".to_string(),
            value: "Off".to_string(),
            enabled: true,
            section: "Error & Logging".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "report_memleaks".to_string(),
            value: "On".to_string(),
            enabled: true,
            section: "Error & Logging".to_string(),
            custom: false,
        },
        // PHP Core
        PhpIniDirective {
            key: "engine".to_string(),
            value: "On".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "short_open_tag".to_string(),
            value: "Off".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "precision".to_string(),
            value: "14".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "zlib.output_compression".to_string(),
            value: "Off".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "implicit_flush".to_string(),
            value: "Off".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "zend.enable_gc".to_string(),
            value: "On".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "expose_php".to_string(),
            value: if is_dev {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "variables_order".to_string(),
            value: "\"GPCS\"".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "request_order".to_string(),
            value: "\"GP\"".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "register_argc_argv".to_string(),
            value: "Off".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "auto_globals_jit".to_string(),
            value: "On".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "default_mimetype".to_string(),
            value: "\"text/html\"".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "default_charset".to_string(),
            value: "\"UTF-8\"".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "date.timezone".to_string(),
            value: "UTC".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "enable_dl".to_string(),
            value: "Off".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "cgi.fix_pathinfo".to_string(),
            value: "1".to_string(),
            enabled: true,
            section: "PHP Core".to_string(),
            custom: false,
        },
        // Session & Security
        PhpIniDirective {
            key: "allow_url_fopen".to_string(),
            value: "On".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "allow_url_include".to_string(),
            value: "Off".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.save_handler".to_string(),
            value: "files".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.use_strict_mode".to_string(),
            value: "1".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.use_cookies".to_string(),
            value: "1".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.use_only_cookies".to_string(),
            value: "1".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.name".to_string(),
            value: "PHPSESSID".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.cookie_httponly".to_string(),
            value: "1".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.cookie_samesite".to_string(),
            value: "Lax".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "session.gc_maxlifetime".to_string(),
            value: "1440".to_string(),
            enabled: true,
            section: "Session & Security".to_string(),
            custom: false,
        },
        // OPcache
        PhpIniDirective {
            key: "opcache.enable".to_string(),
            value: "1".to_string(),
            enabled: true,
            section: "opcache".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "opcache.enable_cli".to_string(),
            value: "0".to_string(),
            enabled: true,
            section: "opcache".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "opcache.memory_consumption".to_string(),
            value: "128".to_string(),
            enabled: true,
            section: "opcache".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "opcache.interned_strings_buffer".to_string(),
            value: "8".to_string(),
            enabled: true,
            section: "opcache".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "opcache.max_accelerated_files".to_string(),
            value: "10000".to_string(),
            enabled: true,
            section: "opcache".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "opcache.revalidate_freq".to_string(),
            value: if is_dev {
                "0".to_string()
            } else {
                "60".to_string()
            },
            enabled: true,
            section: "opcache".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "opcache.validate_timestamps".to_string(),
            value: if is_dev {
                "1".to_string()
            } else {
                "0".to_string()
            },
            enabled: true,
            section: "opcache".to_string(),
            custom: false,
        },
        // Xdebug
        PhpIniDirective {
            key: "xdebug.mode".to_string(),
            value: if is_dev {
                "debug,develop".to_string()
            } else {
                "off".to_string()
            },
            enabled: is_dev,
            section: "xdebug".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "xdebug.client_port".to_string(),
            value: "9003".to_string(),
            enabled: is_dev,
            section: "xdebug".to_string(),
            custom: false,
        },
        PhpIniDirective {
            key: "xdebug.start_with_request".to_string(),
            value: "trigger".to_string(),
            enabled: is_dev,
            section: "xdebug".to_string(),
            custom: false,
        },
    ]
}

pub fn validate_directive_key(key: &str) -> Result<String, DriverError> {
    let s = key.trim();
    if s.is_empty() {
        return Err(DriverError::Config(
            "Directive key cannot be empty".to_string(),
        ));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
    {
        return Err(DriverError::Config(format!(
            "Directive key '{s}' contains invalid characters"
        )));
    }
    Ok(s.to_string())
}

pub fn validate_directive_value(val: &str) -> Result<String, DriverError> {
    let s = val.trim();
    if s.chars().any(|c| c.is_control() || c == '\n' || c == '\r') {
        return Err(DriverError::Config(
            "Directive value contains illegal control characters".to_string(),
        ));
    }
    Ok(s.to_string())
}

fn validate_size_limit(key: &str, val: &str) -> Result<String, DriverError> {
    let s = val.trim();
    if s.is_empty() {
        return Err(DriverError::Config(format!(
            "Configuration limit '{key}' cannot be empty"
        )));
    }

    if s.chars().any(|c| c.is_control() || c == '\n' || c == '\r') {
        return Err(DriverError::Config(format!(
            "Configuration limit '{key}' contains illegal control characters"
        )));
    }

    if s == "-1" {
        return Ok("-1".to_string());
    }

    let (digits, suffix) = match s.chars().last() {
        Some(c @ ('K' | 'M' | 'G' | 'k' | 'm' | 'g')) => {
            (&s[..s.len() - 1], Some(c.to_ascii_uppercase()))
        }
        _ => (s, None),
    };

    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(DriverError::Config(format!(
            "Configuration limit '{key}' has invalid format '{val}' (expected ^(-1|\\d+[KMG]?)$)"
        )));
    }

    let formatted = match suffix {
        Some(c) => format!("{digits}{c}"),
        None => digits.to_string(),
    };
    Ok(formatted)
}

fn parse_size_bytes(val: &str) -> Option<i64> {
    let s = val.trim();
    if s == "-1" {
        return Some(-1);
    }
    let (digits, mult) = match s.chars().last() {
        Some('K' | 'k') => (&s[..s.len() - 1], 1024i64),
        Some('M' | 'm') => (&s[..s.len() - 1], 1024 * 1024i64),
        Some('G' | 'g') => (&s[..s.len() - 1], 1024 * 1024 * 1024i64),
        _ => (s, 1i64),
    };
    digits.parse::<i64>().ok().map(|d| d * mult)
}

fn validate_ext_name(name: &str) -> Result<String, DriverError> {
    let s = name.trim();
    if s.is_empty() {
        return Err(DriverError::Config(
            "Extension name cannot be empty".to_string(),
        ));
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(DriverError::Config(format!(
            "Extension name '{s}' contains invalid characters (allowed: [a-zA-Z0-9_])"
        )));
    }
    Ok(s.to_lowercase())
}

fn is_zend_extension(name: &str) -> bool {
    matches!(name, "opcache" | "zend opcache" | "xdebug" | "pcov")
}

fn extension_tier(name: &str) -> u8 {
    match name {
        "openssl" | "pcre" | "zlib" | "hash" | "session" | "ctype" | "tokenizer" | "filter" => 0,
        "mbstring" | "xml" | "dom" | "simplexml" | "xmlreader" | "xmlwriter" | "pdo"
        | "mysqlnd" | "sockets" | "curl" | "fileinfo" | "bcmath" | "intl" | "gd" | "zip"
        | "iconv" | "sodium" => 1,
        "exif" | "pdo_mysql" | "pdo_pgsql" | "pdo_sqlite" | "mysqli" | "pgsql" | "sqlite3"
        | "redis" | "memcached" | "imagick" => 2,
        _ => 3,
    }
}

pub fn render_ini_with_directives(
    version: &str,
    profile: PhpProfileMode,
    limits: &PhpIniLimits,
    enabled_exts: &[String],
    ext_dir: Option<&Path>,
    custom_directives: &[PhpIniDirective],
) -> Result<RenderedIni, DriverError> {
    if version
        .chars()
        .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err(DriverError::Config(
            "PHP version string contains illegal control characters".to_string(),
        ));
    }

    let defaults = default_limits_for_profile(profile);

    let mut eff_mem_str = if limits.memory_limit.trim().is_empty() {
        defaults.memory_limit.clone()
    } else {
        limits.memory_limit.clone()
    };
    let mut eff_upload_str = if limits.upload_max_filesize.trim().is_empty() {
        defaults.upload_max_filesize.clone()
    } else {
        limits.upload_max_filesize.clone()
    };
    let mut eff_post_str = if limits.post_max_size.trim().is_empty() {
        defaults.post_max_size.clone()
    } else {
        limits.post_max_size.clone()
    };
    let mut eff_max_exec = if limits.max_execution_time > 0 {
        limits.max_execution_time
    } else {
        defaults.max_execution_time
    };

    for d in custom_directives {
        if d.key == "memory_limit" && !d.value.trim().is_empty() {
            eff_mem_str = d.value.clone();
        } else if d.key == "upload_max_filesize" && !d.value.trim().is_empty() {
            eff_upload_str = d.value.clone();
        } else if d.key == "post_max_size" && !d.value.trim().is_empty() {
            eff_post_str = d.value.clone();
        } else if d.key == "max_execution_time" {
            if let Ok(v) = d.value.trim().parse::<u32>() {
                eff_max_exec = v;
            }
        }
    }

    let eff_mem = validate_size_limit("memory_limit", &eff_mem_str)?;
    let eff_upload = validate_size_limit("upload_max_filesize", &eff_upload_str)?;
    let eff_post = validate_size_limit("post_max_size", &eff_post_str)?;

    let mut warnings = Vec::new();
    let upload_bytes = parse_size_bytes(&eff_upload);
    let post_bytes = parse_size_bytes(&eff_post);
    if let (Some(u), Some(p)) = (upload_bytes, post_bytes) {
        if p != -1 && u > p {
            warnings.push(format!(
                "upload_max_filesize ({eff_upload}) exceeds post_max_size ({eff_post})"
            ));
        }
    }

    let mut valid_exts = BTreeSet::new();
    for ext in enabled_exts {
        let norm = validate_ext_name(ext)?;
        valid_exts.insert(norm);
    }

    let is_dev = profile == PhpProfileMode::Dev;

    let mut normal_exts: Vec<String> = valid_exts
        .iter()
        .filter(|e| !is_zend_extension(e))
        .cloned()
        .collect();
    normal_exts.sort_by(|a, b| {
        let tier_a = extension_tier(a);
        let tier_b = extension_tier(b);
        tier_a.cmp(&tier_b).then_with(|| a.cmp(b))
    });

    let mut zend_exts: Vec<String> = valid_exts
        .iter()
        .filter(|e| is_zend_extension(e))
        .cloned()
        .collect();

    if !is_dev {
        zend_exts.retain(|e| e != "xdebug");
    }

    zend_exts.sort_by_key(|e| match e.as_str() {
        "opcache" => 0,
        "pcov" => 1,
        "xdebug" => 2,
        _ => 3,
    });

    let has_custom = !custom_directives.is_empty();
    let mut out = String::with_capacity(4096);
    let profile_str = if is_dev { "dev" } else { "prod" };

    if !has_custom {
        out.push_str(&format!(
            r#"[PHP]
; Rakitray generated configuration for PHP {version} ({profile_str} profile)
engine = On
short_open_tag = Off
precision = 14
output_buffering = 4096
zlib.output_compression = Off
implicit_flush = Off
zend.enable_gc = On

max_execution_time = {eff_max_exec}
max_input_time = 60
memory_limit = {eff_mem}

error_reporting = {error_reporting}
display_errors = {display_errors}
display_startup_errors = {display_startup_errors}
log_errors = On

variables_order = "GPCS"
request_order = "GP"
register_argc_argv = Off
auto_globals_jit = On

post_max_size = {eff_post}
file_uploads = On
upload_max_filesize = {eff_upload}
max_file_uploads = 20

default_mimetype = "text/html"
default_charset = "UTF-8"

enable_dl = Off
"#,
            version = version,
            profile_str = profile_str,
            eff_max_exec = eff_max_exec,
            eff_mem = eff_mem,
            error_reporting = if is_dev {
                "E_ALL"
            } else {
                "E_ALL & ~E_DEPRECATED & ~E_STRICT"
            },
            display_errors = if is_dev { "On" } else { "Off" },
            display_startup_errors = if is_dev { "On" } else { "Off" },
            eff_post = eff_post,
            eff_upload = eff_upload,
        ));
    } else {
        out.push_str(&format!(
            r#"[PHP]
; Rakitray generated configuration for PHP {version} ({profile_str} profile)
"#
        ));
        for d in custom_directives {
            if d.section == "opcache" || d.section == "xdebug" {
                continue;
            }
            validate_directive_key(&d.key)?;
            validate_directive_value(&d.value)?;
            if d.enabled {
                out.push_str(&format!("{} = {}\n", d.key, d.value));
            } else {
                out.push_str(&format!("; {} = {}\n", d.key, d.value));
            }
        }
    }

    if let Some(dir) = ext_dir {
        let dir_str = dir.display().to_string();
        if dir_str
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
        {
            return Err(DriverError::Config(
                "extension_dir contains illegal control characters".to_string(),
            ));
        }
        out.push_str(&format!("extension_dir = \"{}\"\n", dir_str));
    }

    if !normal_exts.is_empty() {
        out.push_str("\n; Dynamic Extensions\n");
        for ext in &normal_exts {
            out.push_str(&format!("extension={ext}\n"));
        }
    }

    if !zend_exts.is_empty() {
        out.push_str("\n; Zend Extensions\n");
        for zend in &zend_exts {
            out.push_str(&format!("zend_extension={zend}\n"));
        }
    }

    if !has_custom {
        out.push_str(&format!(
            r#"
[opcache]
opcache.enable = 1
opcache.enable_cli = 0
opcache.memory_consumption = 128
opcache.interned_strings_buffer = 8
opcache.max_accelerated_files = 10000
opcache.revalidate_freq = {revalidate_freq}
opcache.validate_timestamps = {validate_timestamps}
"#,
            revalidate_freq = if is_dev { "0" } else { "60" },
            validate_timestamps = if is_dev { "1" } else { "0" }
        ));

        if is_dev && valid_exts.contains("xdebug") {
            out.push_str(
                r#"
[xdebug]
xdebug.mode = debug,develop
xdebug.client_port = 9003
"#,
            );
        }
    } else {
        let opcache_dirs: Vec<&PhpIniDirective> = custom_directives
            .iter()
            .filter(|d| d.section == "opcache")
            .collect();
        if !opcache_dirs.is_empty() {
            out.push_str("\n[opcache]\n");
            for d in opcache_dirs {
                validate_directive_key(&d.key)?;
                validate_directive_value(&d.value)?;
                if d.enabled {
                    out.push_str(&format!("{} = {}\n", d.key, d.value));
                } else {
                    out.push_str(&format!("; {} = {}\n", d.key, d.value));
                }
            }
        }

        let xdebug_dirs: Vec<&PhpIniDirective> = custom_directives
            .iter()
            .filter(|d| d.section == "xdebug")
            .collect();
        if !xdebug_dirs.is_empty() && is_dev && valid_exts.contains("xdebug") {
            out.push_str("\n[xdebug]\n");
            for d in xdebug_dirs {
                validate_directive_key(&d.key)?;
                validate_directive_value(&d.value)?;
                if d.enabled {
                    out.push_str(&format!("{} = {}\n", d.key, d.value));
                } else {
                    out.push_str(&format!("; {} = {}\n", d.key, d.value));
                }
            }
        }
    }

    Ok(RenderedIni { ini: out, warnings })
}

pub fn render_ini(
    version: &str,
    profile: PhpProfileMode,
    limits: &PhpIniLimits,
    enabled_exts: &[String],
    ext_dir: Option<&Path>,
) -> Result<RenderedIni, DriverError> {
    render_ini_with_directives(version, profile, limits, enabled_exts, ext_dir, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_ini_dev_snapshot() {
        let limits = PhpIniLimits {
            memory_limit: "512M".to_string(),
            upload_max_filesize: "64M".to_string(),
            post_max_size: "64M".to_string(),
            max_execution_time: 120,
        };
        let exts = vec![
            "mbstring".to_string(),
            "curl".to_string(),
            "opcache".to_string(),
            "xdebug".to_string(),
        ];
        let res = render_ini("8.3.14", PhpProfileMode::Dev, &limits, &exts, None).unwrap();

        assert!(res.ini.contains("display_errors = On"));
        assert!(res.ini.contains("display_startup_errors = On"));
        assert!(res.ini.contains("error_reporting = E_ALL"));
        assert!(res.ini.contains("memory_limit = 512M"));
        assert!(res.ini.contains("opcache.revalidate_freq = 0"));
        assert!(res.ini.contains("opcache.validate_timestamps = 1"));
        assert!(res.ini.contains("opcache.enable_cli = 0"));
        assert!(res.ini.contains("extension=curl\n"));
        assert!(res.ini.contains("extension=mbstring\n"));
        assert!(res.ini.contains("zend_extension=opcache\n"));
        assert!(res.ini.contains("zend_extension=xdebug\n"));
        assert!(res
            .ini
            .contains("[xdebug]\nxdebug.mode = debug,develop\nxdebug.client_port = 9003"));
        assert!(res.warnings.is_empty());
    }

    #[test]
    fn render_ini_prod_snapshot_excludes_xdebug() {
        let limits = PhpIniLimits {
            memory_limit: "1024M".to_string(),
            upload_max_filesize: "".to_string(),
            post_max_size: "".to_string(),
            max_execution_time: 0,
        };
        let exts = vec![
            "mbstring".to_string(),
            "xdebug".to_string(),
            "opcache".to_string(),
        ];
        let res = render_ini("8.3.14", PhpProfileMode::Prod, &limits, &exts, None).unwrap();

        assert!(res.ini.contains("display_errors = Off"));
        assert!(res.ini.contains("display_startup_errors = Off"));
        assert!(res.ini.contains("memory_limit = 1024M"));
        assert!(res.ini.contains("opcache.revalidate_freq = 60"));
        assert!(res.ini.contains("opcache.validate_timestamps = 0"));
        assert!(!res.ini.contains("xdebug"));
        assert!(res.warnings.is_empty());
    }

    #[test]
    fn render_ini_topological_ordering() {
        let limits = default_limits_for_profile(PhpProfileMode::Dev);
        let exts = vec![
            "exif".to_string(),
            "pdo_mysql".to_string(),
            "mbstring".to_string(),
            "pdo".to_string(),
            "curl".to_string(),
            "opcache".to_string(),
        ];
        let res = render_ini("8.3.14", PhpProfileMode::Dev, &limits, &exts, None).unwrap();

        let idx_mbstring = res.ini.find("extension=mbstring").unwrap();
        let idx_exif = res.ini.find("extension=exif").unwrap();
        let idx_pdo = res.ini.find("extension=pdo\n").unwrap();
        let idx_pdo_mysql = res.ini.find("extension=pdo_mysql").unwrap();

        assert!(idx_mbstring < idx_exif);
        assert!(idx_pdo < idx_pdo_mysql);
        assert!(res.ini.contains("zend_extension=opcache"));
    }

    #[test]
    fn render_ini_rejects_injection_and_warns_limits() {
        let bad_limits = PhpIniLimits {
            memory_limit: "512M\nauto_prepend_file=/tmp/pwn.php".to_string(),
            upload_max_filesize: "64M".to_string(),
            post_max_size: "64M".to_string(),
            max_execution_time: 120,
        };
        let err = render_ini("8.3.14", PhpProfileMode::Dev, &bad_limits, &[], None);
        assert!(err.is_err());

        let bad_ext = vec!["../evil".to_string()];
        let limits = default_limits_for_profile(PhpProfileMode::Dev);
        let err2 = render_ini("8.3.14", PhpProfileMode::Dev, &limits, &bad_ext, None);
        assert!(err2.is_err());

        let warn_limits = PhpIniLimits {
            memory_limit: "512M".to_string(),
            upload_max_filesize: "128M".to_string(),
            post_max_size: "64M".to_string(),
            max_execution_time: 120,
        };
        let res = render_ini("8.3.14", PhpProfileMode::Dev, &warn_limits, &[], None).unwrap();
        assert_eq!(res.warnings.len(), 1);
        assert!(res.warnings[0].contains("upload_max_filesize (128M) exceeds post_max_size (64M)"));
    }
}
