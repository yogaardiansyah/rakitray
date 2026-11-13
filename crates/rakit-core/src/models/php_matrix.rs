use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PhpProfileMode {
    Dev,
    Prod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionKind {
    Extension,
    ZendExtension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionSource {
    SharedLib,
    BuiltIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionCategory {
    LaravelCore,
    DatabaseCache,
    PerformanceDebug,
    MediaFormat,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhpExtension {
    pub name: String,
    pub kind: ExtensionKind,
    pub source: ExtensionSource,
    pub category: ExtensionCategory,
    pub enabled: bool,
    pub toggleable: bool,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhpIniLimits {
    pub memory_limit: String,
    pub upload_max_filesize: String,
    pub post_max_size: String,
    pub max_execution_time: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhpIniDirective {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub section: String,
    #[serde(default)]
    pub custom: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhpMatrix {
    pub instance_id: String,
    pub version: String,
    pub profile: PhpProfileMode,
    pub limits: PhpIniLimits,
    pub extensions: Vec<PhpExtension>,
    #[serde(default)]
    pub directives: Vec<PhpIniDirective>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PhpMatrixEvent {
    #[serde(rename_all = "camelCase")]
    Applying {
        instance_id: String,
        version: String,
    },
    #[serde(rename_all = "camelCase")]
    Reloaded {
        instance_id: String,
        version: String,
    },
    #[serde(rename_all = "camelCase")]
    RolledBack {
        instance_id: String,
        version: String,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn php_matrix_serde_roundtrip_and_snapshot() {
        let matrix = PhpMatrix {
            instance_id: "php_83".to_string(),
            version: "8.3.14".to_string(),
            profile: PhpProfileMode::Dev,
            limits: PhpIniLimits {
                memory_limit: "512M".to_string(),
                upload_max_filesize: "64M".to_string(),
                post_max_size: "64M".to_string(),
                max_execution_time: 120,
            },
            extensions: vec![
                PhpExtension {
                    name: "mbstring".to_string(),
                    kind: ExtensionKind::Extension,
                    source: ExtensionSource::SharedLib,
                    category: ExtensionCategory::LaravelCore,
                    enabled: true,
                    toggleable: true,
                    requires: vec![],
                },
                PhpExtension {
                    name: "exif".to_string(),
                    kind: ExtensionKind::Extension,
                    source: ExtensionSource::SharedLib,
                    category: ExtensionCategory::MediaFormat,
                    enabled: false,
                    toggleable: true,
                    requires: vec!["mbstring".to_string()],
                },
                PhpExtension {
                    name: "opcache".to_string(),
                    kind: ExtensionKind::ZendExtension,
                    source: ExtensionSource::BuiltIn,
                    category: ExtensionCategory::PerformanceDebug,
                    enabled: true,
                    toggleable: false,
                    requires: vec![],
                },
                PhpExtension {
                    name: "redis".to_string(),
                    kind: ExtensionKind::Extension,
                    source: ExtensionSource::SharedLib,
                    category: ExtensionCategory::DatabaseCache,
                    enabled: true,
                    toggleable: true,
                    requires: vec![],
                },
                PhpExtension {
                    name: "gettext".to_string(),
                    kind: ExtensionKind::Extension,
                    source: ExtensionSource::SharedLib,
                    category: ExtensionCategory::Other,
                    enabled: false,
                    toggleable: true,
                    requires: vec![],
                },
            ],
            directives: vec![],
            warnings: vec!["xdebug not found in ext/".to_string()],
        };

        let val = serde_json::to_value(&matrix).expect("serialize PhpMatrix");
        let expected = json!({
            "instanceId": "php_83",
            "version": "8.3.14",
            "profile": "dev",
            "limits": {
                "memoryLimit": "512M",
                "uploadMaxFilesize": "64M",
                "postMaxSize": "64M",
                "maxExecutionTime": 120
            },
            "extensions": [
                {
                    "name": "mbstring",
                    "kind": "extension",
                    "source": "sharedLib",
                    "category": "laravelCore",
                    "enabled": true,
                    "toggleable": true,
                    "requires": []
                },
                {
                    "name": "exif",
                    "kind": "extension",
                    "source": "sharedLib",
                    "category": "mediaFormat",
                    "enabled": false,
                    "toggleable": true,
                    "requires": ["mbstring"]
                },
                {
                    "name": "opcache",
                    "kind": "zendExtension",
                    "source": "builtIn",
                    "category": "performanceDebug",
                    "enabled": true,
                    "toggleable": false,
                    "requires": []
                },
                {
                    "name": "redis",
                    "kind": "extension",
                    "source": "sharedLib",
                    "category": "databaseCache",
                    "enabled": true,
                    "toggleable": true,
                    "requires": []
                },
                {
                    "name": "gettext",
                    "kind": "extension",
                    "source": "sharedLib",
                    "category": "other",
                    "enabled": false,
                    "toggleable": true,
                    "requires": []
                }
            ],
            "directives": [],
            "warnings": ["xdebug not found in ext/"]
        });

        assert_eq!(val, expected);

        let decoded: PhpMatrix = serde_json::from_value(val).expect("deserialize PhpMatrix");
        assert_eq!(decoded, matrix);
    }

    #[test]
    fn php_matrix_event_serde_roundtrip() {
        let events = vec![
            (
                PhpMatrixEvent::Applying {
                    instance_id: "php_83".to_string(),
                    version: "8.3.14".to_string(),
                },
                json!({
                    "status": "applying",
                    "instanceId": "php_83",
                    "version": "8.3.14"
                }),
            ),
            (
                PhpMatrixEvent::Reloaded {
                    instance_id: "php_83".to_string(),
                    version: "8.3.14".to_string(),
                },
                json!({
                    "status": "reloaded",
                    "instanceId": "php_83",
                    "version": "8.3.14"
                }),
            ),
            (
                PhpMatrixEvent::RolledBack {
                    instance_id: "php_83".to_string(),
                    version: "8.3.14".to_string(),
                    reason: "Unable to load dynamic library 'bad.so'".to_string(),
                },
                json!({
                    "status": "rolledBack",
                    "instanceId": "php_83",
                    "version": "8.3.14",
                    "reason": "Unable to load dynamic library 'bad.so'"
                }),
            ),
        ];

        for (event, expected_json) in events {
            let serialized = serde_json::to_value(&event).expect("serialize PhpMatrixEvent");
            assert_eq!(serialized, expected_json);
            let roundtrip: PhpMatrixEvent =
                serde_json::from_value(serialized).expect("deserialize PhpMatrixEvent");
            assert_eq!(roundtrip, event);
        }
    }
}
