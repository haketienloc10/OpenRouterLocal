use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub providers: HashMap<String, ProviderConfig>,
    pub models: HashMap<String, ModelConfig>,
    #[serde(default)]
    pub fallback_models: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub kind: String,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub pricing: Pricing,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pricing {
    pub input_per_1m: f64,
    pub output_per_1m: f64,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let path = std::env::var("ROUTER_CONFIG").unwrap_or_else(|_| "./config/router.yaml".to_string());
        let raw = fs::read_to_string(&path)?;
        let cfg: Self = serde_yaml::from_str(&raw)?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn parses_config() {
        let yaml = r#"
server:
  bind: "127.0.0.1"
  port: 18790
providers:
  openai:
    kind: "openai_http"
    base_url: "https://api.openai.com/v1"
    api_key_env: "OPENAI_API_KEY"
models:
  gpt-4o:
    provider: "openai"
    pricing:
      input_per_1m: 5.0
      output_per_1m: 15.0
fallback_models: ["gpt-4o"]
"#;
        let cfg: AppConfig = serde_yaml::from_str(yaml).expect("should parse");
        assert_eq!(cfg.server.port, 18790);
        assert!(cfg.providers.contains_key("openai"));
        assert_eq!(cfg.fallback_models.len(), 1);
    }
}
