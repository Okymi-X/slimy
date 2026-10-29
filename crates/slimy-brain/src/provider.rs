use serde::{Deserialize, Serialize};

use crate::request::BrainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// The chat completions dialect spoken by OpenAI and most others.
    OpenAi,
    Anthropic,
    /// Amazon Bedrock, Converse API, with a Bedrock API key.
    Bedrock,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub protocol: Protocol,
    pub base_url: &'static str,
    pub default_model: &'static str,
    pub needs_key: bool,
    pub note: &'static str,
}

pub const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: "groq",
        name: "Groq",
        protocol: Protocol::OpenAi,
        base_url: "https://api.groq.com/openai/v1",
        default_model: "openai/gpt-oss-120b",
        needs_key: true,
        note: "Very fast. Key starts with gsk_.",
    },
    ProviderInfo {
        id: "openai",
        name: "OpenAI",
        protocol: Protocol::OpenAi,
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-4o-mini",
        needs_key: true,
        note: "",
    },
    ProviderInfo {
        id: "anthropic",
        name: "Anthropic",
        protocol: Protocol::Anthropic,
        base_url: "https://api.anthropic.com/v1",
        default_model: "claude-haiku-5-5",
        needs_key: true,
        note: "",
    },
    ProviderInfo {
        id: "deepseek",
        name: "DeepSeek",
        protocol: Protocol::OpenAi,
        base_url: "https://api.deepseek.com",
        default_model: "deepseek-chat",
        needs_key: true,
        note: "",
    },
    ProviderInfo {
        id: "openrouter",
        name: "OpenRouter",
        protocol: Protocol::OpenAi,
        base_url: "https://openrouter.ai/api/v1",
        default_model: "meta-llama/llama-3.3-70b-instruct",
        needs_key: true,
        note: "One key, many models.",
    },
    ProviderInfo {
        id: "xai",
        name: "xAI (Grok)",
        protocol: Protocol::OpenAi,
        base_url: "https://api.x.ai/v1",
        default_model: "grok-3-mini",
        needs_key: true,
        note: "",
    },
    ProviderInfo {
        id: "bedrock",
        name: "Amazon Bedrock",
        protocol: Protocol::Bedrock,
        base_url: "",
        default_model: "amazon.nova-lite-v1:0",
        needs_key: true,
        note: "Needs a Bedrock API key and a region.",
    },
    ProviderInfo {
        id: "ollama",
        name: "Ollama (local)",
        protocol: Protocol::OpenAi,
        base_url: "http://localhost:11434/v1",
        default_model: "llama3.2",
        needs_key: false,
        note: "Runs on this machine, no key.",
    },
    ProviderInfo {
        id: "custom",
        name: "Custom (OpenAI compatible)",
        protocol: Protocol::OpenAi,
        base_url: "http://localhost:1234/v1",
        default_model: "default",
        needs_key: false,
        note: "Any server speaking the OpenAI dialect.",
    },
];

pub fn info(id: &str) -> Option<&'static ProviderInfo> {
    PROVIDERS.iter().find(|p| p.id == id)
}

const MAX_TOKENS: u32 = 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainConfig {
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_tokens")]
    pub max_tokens: u32,
    /// A second provider tried once when this one is unreachable, rate limited or failing (its own key is used).
    #[serde(default)]
    pub fallback: Option<String>,
}

fn default_temperature() -> f32 {
    0.8
}

fn default_tokens() -> u32 {
    220
}

impl BrainConfig {
    pub fn for_provider(id: &str) -> Option<Self> {
        let p = info(id)?;
        Some(Self {
            provider: p.id.to_owned(),
            model: p.default_model.to_owned(),
            base_url: None,
            region: (p.protocol == Protocol::Bedrock).then(|| "us-east-1".to_owned()),
            temperature: default_temperature(),
            max_tokens: default_tokens(),
            fallback: None,
        })
    }

    /// The configuration of the fallback provider, with its own default model.
    pub fn fallback_config(&self) -> Option<Self> {
        Self::for_provider(self.fallback.as_deref()?)
    }

    /// Parses a stored config: the provider must be known, the numbers are clamped, gaps are filled.
    pub fn from_json(json: &str) -> Result<Self, BrainError> {
        let mut c: BrainConfig = serde_json::from_str(json).map_err(|_| BrainError::BadReply)?;
        let p = info(&c.provider).ok_or(BrainError::UnknownProvider)?;
        c.model = c.model.trim().to_owned();
        if c.model.is_empty() {
            c.model = p.default_model.to_owned();
        }
        c.temperature = if c.temperature.is_nan() {
            default_temperature()
        } else {
            c.temperature.clamp(0.0, 1.5)
        };
        c.max_tokens = c.max_tokens.clamp(16, MAX_TOKENS);
        c.base_url = c
            .base_url
            .map(|u| u.trim().to_owned())
            .filter(|u| !u.is_empty());
        c.region = c
            .region
            .map(|r| r.trim().to_owned())
            .filter(|r| !r.is_empty());
        c.fallback = c
            .fallback
            .map(|f| f.trim().to_owned())
            .filter(|f| f != &c.provider && info(f).is_some());
        if p.protocol == Protocol::Bedrock && c.region.is_none() {
            c.region = Some("us-east-1".to_owned());
        }
        Ok(c)
    }
}
