use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub permissions: PluginPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    pub read_command_metadata: bool,
    pub read_raw_output: bool,
    pub network: bool,
    pub filesystem: bool,
}
