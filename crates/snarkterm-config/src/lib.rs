use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ui: UiConfig,
    pub personality: PersonalityConfig,
    pub llm: LlmConfig,
    pub stats: StatsConfig,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub font_family: String,
    pub font_size: f32,
    pub snark_gutter: bool,
    pub gutter_width: u16,
    pub transparency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityConfig {
    pub profile: String,
    pub roast_intensity: u8,
    pub work_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub timeout_ms: u64,
    pub redact_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsConfig {
    pub enabled: bool,
    pub store_command_text: bool,
    pub store_raw_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub telemetry: bool,
    pub local_only: bool,
}
