pub struct OllamaConfig {
    pub endpoint: String,
    pub model: String,
    pub timeout_ms: u64,
    pub max_tokens: u16,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434/api/generate".to_string(),
            model: "llama3.1".to_string(),
            timeout_ms: 750,
            max_tokens: 80,
        }
    }
}
