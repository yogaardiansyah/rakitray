pub mod schema;
pub mod validator;

pub use schema::{
    BlueprintBinaries, BlueprintConfig, BlueprintEngine, BlueprintHooks, BlueprintLifecycle,
    ReadyCheck, ServiceBlueprint,
};
pub use validator::{
    render_template, validate_template_variables, BlueprintError, ALLOWED_TEMPLATE_VARS,
};

use std::collections::HashMap;

/// Loads and validates a ServiceBlueprint from a TOML string.
pub fn parse_blueprint(toml_content: &str) -> Result<ServiceBlueprint, BlueprintError> {
    // Validate all template variables inside strings before deserializing
    validate_template_variables(toml_content)?;

    toml::from_str(toml_content).map_err(|e| BlueprintError::ParseError(e.to_string()))
}

/// Resolves a blueprint's lifecycle arguments with instance context variables.
pub fn resolve_blueprint_args(
    args: &[String],
    vars: &HashMap<String, String>,
) -> Result<Vec<String>, BlueprintError> {
    args.iter().map(|arg| render_template(arg, vars)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARIADB_BLUEPRINT: &str = r#"
[engine]
name = "mariadb"
category = "database"
version = "11.4.3"
default_port = 3306

[binaries]
daemon = "bin/mariadbd"
client = "bin/mariadb"
installer = "scripts/mariadb-install-db"

[lifecycle]
bootstrap_check_dir = "{{data_dir}}/mysql"
bootstrap_cmd = [
  "{{bin_dir}}/scripts/mariadb-install-db",
  "--no-defaults",
  "--datadir={{data_dir}}",
  "--basedir={{bin_dir}}",
  "--auth-root-authentication-method=normal",
]
start_args = [
  "--defaults-file={{config_file}}",
  "--basedir={{bin_dir}}",
  "--datadir={{data_dir}}",
  "--port={{port}}",
  "--socket={{socket_path}}",
  "--pid-file={{pid_path}}",
  "--console",
]
ready_check = { kind = "unix_socket", timeout_secs = 30 }

[hooks]
post_start = "provision_mariadb"
"#;

    #[test]
    fn test_parse_mariadb_blueprint() {
        let blueprint =
            parse_blueprint(MARIADB_BLUEPRINT).expect("Failed to parse valid blueprint");
        assert_eq!(blueprint.engine.name, "mariadb");
        assert_eq!(blueprint.engine.default_port, Some(3306));
        assert_eq!(blueprint.binaries.daemon, "bin/mariadbd");
        assert_eq!(blueprint.lifecycle.start_args.len(), 7);
        assert_eq!(blueprint.lifecycle.ready_check.timeout_secs(), 30);
    }

    #[test]
    fn test_resolve_args() {
        let mut vars = HashMap::new();
        vars.insert(
            "data_dir".to_string(),
            "/runtime/data/mariadb_1".to_string(),
        );
        vars.insert(
            "bin_dir".to_string(),
            "/runtime/bin/database/mariadb/11.4.3".to_string(),
        );
        vars.insert(
            "config_file".to_string(),
            "/runtime/config/mariadb/mariadb_1.cnf".to_string(),
        );
        vars.insert("port".to_string(), "3307".to_string());
        vars.insert(
            "socket_path".to_string(),
            "/runtime/tmp/mariadb_1.sock".to_string(),
        );
        vars.insert(
            "pid_path".to_string(),
            "/runtime/tmp/mariadb_1.pid".to_string(),
        );

        let blueprint = parse_blueprint(MARIADB_BLUEPRINT).unwrap();
        let resolved = resolve_blueprint_args(&blueprint.lifecycle.start_args, &vars).unwrap();

        assert_eq!(
            resolved[0],
            "--defaults-file=/runtime/config/mariadb/mariadb_1.cnf"
        );
        assert_eq!(resolved[3], "--port=3307");
        assert_eq!(resolved[6], "--console");
    }

    #[test]
    fn test_invalid_var_in_blueprint_fails() {
        let invalid_toml = r#"
[engine]
name = "nginx"
category = "web_server"
version = "1.26"

[binaries]
daemon = "sbin/nginx"

[lifecycle]
start_args = ["-c", "{{evil_path}}"]
ready_check = { kind = "tcp", timeout_secs = 10 }
"#;
        let res = parse_blueprint(invalid_toml);
        assert!(matches!(res, Err(BlueprintError::UnknownVariable(..))));
    }
}
