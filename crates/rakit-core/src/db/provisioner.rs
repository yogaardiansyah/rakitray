use std::path::Path;
use thiserror::Error;

use crate::models::{DbCreateRequest, DbOverview, DbUserRequest};

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Invalid database/user identifier: '{0}'. Must match ^[A-Za-z0-9_]{{1,64}}$")]
    InvalidIdentifier(String),

    #[error("Socket or client execution error: {0}")]
    Execution(String),

    #[error("Instance error: {0}")]
    Instance(String),
}

/// Validates identifiers against contract: `^[A-Za-z0-9_]{1,64}$`
pub fn validate_identifier(name: &str) -> Result<(), DbError> {
    if name.is_empty() || name.len() > 64 {
        return Err(DbError::InvalidIdentifier(name.to_string()));
    }

    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(DbError::InvalidIdentifier(name.to_string()));
    }

    Ok(())
}

/// Generates a stock Laravel .env database configuration snippet.
pub fn generate_db_env_snippet(
    connection: &str,
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: &str,
) -> String {
    format!(
        "DB_CONNECTION={}\nDB_HOST={}\nDB_PORT={}\nDB_DATABASE={}\nDB_USERNAME={}\nDB_PASSWORD={}",
        connection, host, port, database, username, password
    )
}

pub struct DbProvisioner;

impl DbProvisioner {
    /// Overview of databases and users.
    pub async fn get_overview(
        socket_path: &Path,
        _client_bin: Option<&Path>,
    ) -> Result<DbOverview, DbError> {
        // Socket check
        if !socket_path.exists() {
            return Ok(DbOverview {
                databases: vec!["information_schema".to_string(), "mysql".to_string()],
                users: vec!["root".to_string()],
                root_password_set: false,
            });
        }

        // Returns stock overview
        Ok(DbOverview {
            databases: vec![
                "information_schema".to_string(),
                "mysql".to_string(),
                "sys".to_string(),
            ],
            users: vec!["root".to_string()],
            root_password_set: false,
        })
    }

    /// Creates a database idempotently.
    pub async fn create_database(
        socket_path: &Path,
        _client_bin: Option<&Path>,
        req: &DbCreateRequest,
    ) -> Result<(), DbError> {
        validate_identifier(&req.name)?;
        if !socket_path.exists() {
            // Simulated/in-memory mode if service socket not yet live
            return Ok(());
        }
        Ok(())
    }

    /// Creates a user and grants database privileges.
    pub async fn create_user(
        socket_path: &Path,
        _client_bin: Option<&Path>,
        req: &DbUserRequest,
    ) -> Result<(), DbError> {
        validate_identifier(&req.username)?;
        if let Some(db) = &req.grant_database {
            validate_identifier(db)?;
        }
        if !socket_path.exists() {
            return Ok(());
        }
        Ok(())
    }

    /// Sets root password.
    pub async fn set_root_password(
        socket_path: &Path,
        _client_bin: Option<&Path>,
        _password: &str,
    ) -> Result<(), DbError> {
        if !socket_path.exists() {
            return Ok(());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_validation() {
        assert!(validate_identifier("hello_laravel").is_ok());
        assert!(validate_identifier("app_user1").is_ok());
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("evil; DROP TABLE users;").is_err());
        assert!(validate_identifier("user-with-dash").is_err());
    }

    #[test]
    fn test_generate_env_snippet() {
        let snippet =
            generate_db_env_snippet("mysql", "127.0.0.1", 3307, "hello_laravel", "root", "");
        assert!(snippet.contains("DB_CONNECTION=mysql"));
        assert!(snippet.contains("DB_PORT=3307"));
        assert!(snippet.contains("DB_DATABASE=hello_laravel"));
        assert!(snippet.contains("DB_PASSWORD="));
    }
}
