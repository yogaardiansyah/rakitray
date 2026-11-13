use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub const ALLOWED_TEMPLATE_VARS: &[&str] = &[
    "instance_id",
    "version",
    "bin_dir",
    "data_dir",
    "config_file",
    "port",
    "socket_path",
    "pid_path",
    "log_file",
    "bind_address",
];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BlueprintError {
    #[error("Unknown template variable '{{{{{0}}}}}' in blueprint. Allowed: {1}")]
    UnknownVariable(String, String),
    #[error("Missing variable value for key '{0}' during rendering")]
    MissingVariableValue(String),
    #[error("Invalid blueprint TOML: {0}")]
    ParseError(String),
}

/// Validates that all `{{var}}` occurrences belong to the contract whitelist.
pub fn validate_template_variables(text: &str) -> Result<(), BlueprintError> {
    let allowed: HashSet<&str> = ALLOWED_TEMPLATE_VARS.iter().copied().collect();
    let mut cursor = 0;

    while let Some(start) = text[cursor..].find("{{") {
        let abs_start = cursor + start;
        let inner_start = abs_start + 2;
        if let Some(end) = text[inner_start..].find("}}") {
            let abs_end = inner_start + end;
            let var_name = text[inner_start..abs_end].trim();
            if !allowed.contains(var_name) {
                return Err(BlueprintError::UnknownVariable(
                    var_name.to_string(),
                    ALLOWED_TEMPLATE_VARS.join(", "),
                ));
            }
            cursor = abs_end + 2;
        } else {
            break;
        }
    }
    Ok(())
}

/// Renders a template string by replacing `{{key}}` with values from `vars`.
pub fn render_template(
    template: &str,
    vars: &HashMap<String, String>,
) -> Result<String, BlueprintError> {
    validate_template_variables(template)?;

    let mut result = template.to_string();
    for (k, v) in vars {
        let pattern = format!("{{{{{}}}}}", k);
        result = result.replace(&pattern, v);
    }

    // Check if any `{{...}}` tags remain unreplaced
    if let Some(start) = result.find("{{") {
        let inner_start = start + 2;
        if let Some(end) = result[inner_start..].find("}}") {
            let var_name = &result[inner_start..inner_start + end];
            return Err(BlueprintError::MissingVariableValue(
                var_name.trim().to_string(),
            ));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_variables() {
        let tmpl = "--port={{port}} --datadir={{data_dir}} --bin={{bin_dir}}";
        assert!(validate_template_variables(tmpl).is_ok());
    }

    #[test]
    fn test_invalid_variable_rejected() {
        let tmpl = "--port={{port}} --custom={{dangerous_eval}}";
        let err = validate_template_variables(tmpl).unwrap_err();
        match err {
            BlueprintError::UnknownVariable(var, _) => assert_eq!(var, "dangerous_eval"),
            _ => panic!("Expected UnknownVariable error"),
        }
    }

    #[test]
    fn test_render_template() {
        let mut vars = HashMap::new();
        vars.insert("port".to_string(), "3306".to_string());
        vars.insert(
            "data_dir".to_string(),
            "/runtime/data/mariadb_1".to_string(),
        );

        let res = render_template("--port={{port}} --datadir={{data_dir}}", &vars).unwrap();
        assert_eq!(res, "--port=3306 --datadir=/runtime/data/mariadb_1");
    }

    #[test]
    fn test_missing_var_render_fails() {
        let vars = HashMap::new();
        let res = render_template("--port={{port}}", &vars);
        assert!(matches!(res, Err(BlueprintError::MissingVariableValue(_))));
    }
}
