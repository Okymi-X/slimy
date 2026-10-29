use serde_json::{json, Value};

use crate::provider::{info, BrainConfig, Protocol};

const MAX_MESSAGES: usize = 12;
const MAX_TEXT: usize = 1000;
const MAX_SYSTEM: usize = 4000;
/// Longest base64 image accepted, in bytes of text: about 1.1 MB of JPEG.
pub const MAX_IMAGE: usize = 1_500_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrainError {
    NoKey,
    BadKey,
    RateLimited,
    Network,
    Timeout,
    BadEndpoint,
    BadReply,
    /// The attached image is not valid base64 or is too large.
    BadImage,
    UnknownProvider,
    Http(u16),
    Provider(String),
}

impl BrainError {
    /// The provider refused because the model tried a native function call: it took the prompt's list of powers for tools.
    pub fn is_tool_confusion(&self) -> bool {
        matches!(self, BrainError::Provider(m) if { let m = m.to_lowercase(); m.contains("called a tool") || m.contains("tool_use_failed") || m.contains("tool choice") })
    }

    /// A failure that another provider could avoid: the network, a timeout, a rate limit or a server error. A bad key or a bad request would fail again.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            BrainError::RateLimited
                | BrainError::Network
                | BrainError::Timeout
                | BrainError::Http(500..=599)
        )
    }

    /// A short code the page can translate.
    pub fn code(&self) -> &'static str {
        match self {
            BrainError::NoKey => "no_key",
            BrainError::BadKey => "bad_key",
            BrainError::RateLimited => "rate_limited",
            BrainError::Network => "network",
            BrainError::Timeout => "timeout",
            BrainError::BadEndpoint => "bad_endpoint",
            BrainError::BadReply => "bad_reply",
            BrainError::BadImage => "bad_image",
            BrainError::UnknownProvider => "unknown_provider",
            BrainError::Http(_) | BrainError::Provider(_) => "provider",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub text: String,
    /// A JPEG picture as base64, shown to the model with this turn. Only the last one in a conversation is sent.
    pub image: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

fn role(r: Role) -> &'static str {
    match r {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn clean_key(key: Option<&str>) -> Option<String> {
    let key = key?.trim();
    (!key.is_empty() && key.chars().all(|c| c.is_ascii_graphic())).then(|| key.to_owned())
}

fn is_local(url: &str) -> bool {
    ["http://localhost", "http://127.0.0.1", "http://[::1]"]
        .iter()
        .any(|p| url.starts_with(p) && url[p.len()..].starts_with([':', '/']) || url == *p)
}

fn endpoint(config: &BrainConfig) -> Result<String, BrainError> {
    let p = info(&config.provider).ok_or(BrainError::UnknownProvider)?;
    if p.protocol == Protocol::Bedrock {
        let region = config.region.as_deref().unwrap_or("us-east-1");
        if region.is_empty()
            || !region
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(BrainError::BadEndpoint);
        }
        let model: String = config
            .model
            .bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
        return Ok(format!(
            "https://bedrock-runtime.{region}.amazonaws.com/model/{model}/converse"
        ));
    }
    let base = config
        .base_url
        .as_deref()
        .unwrap_or(p.base_url)
        .trim_end_matches('/');
    if base.chars().any(|c| c.is_control() || c.is_whitespace())
        || !(base.starts_with("https://") || is_local(base))
    {
        return Err(BrainError::BadEndpoint);
    }
    Ok(match p.protocol {
        Protocol::Anthropic => format!("{base}/messages"),
        _ => format!("{base}/chat/completions"),
    })
}

pub fn build_request(
    config: &BrainConfig,
    key: Option<&str>,
    system: &str,
    messages: &[Message],
) -> Result<HttpRequest, BrainError> {
    let p = info(&config.provider).ok_or(BrainError::UnknownProvider)?;
    let key = clean_key(key);
    if p.needs_key && key.is_none() {
        return Err(BrainError::NoKey);
    }
    let url = endpoint(config)?;
    let system = cut(system, MAX_SYSTEM);
    let start = messages.len().saturating_sub(MAX_MESSAGES);
    let window = &messages[start..];
    // One picture at most, the latest: older ones would only make every request heavier.
    let with_image = window.iter().rposition(|m| m.image.is_some());
    if let Some(i) = with_image {
        let image = window[i].image.as_deref().unwrap_or_default();
        if image.is_empty()
            || image.len() > MAX_IMAGE
            || !image
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
        {
            return Err(BrainError::BadImage);
        }
    }
    let image_of =
        |i: usize| (Some(i) == with_image).then(|| window[i].image.clone().unwrap_or_default());
    let turns: Vec<(&str, String, Option<String>)> = window
        .iter()
        .enumerate()
        .map(|(i, m)| (role(m.role), cut(&m.text, MAX_TEXT), image_of(i)))
        .collect();
    let mut headers = vec![("Content-Type".to_owned(), "application/json".to_owned())];

    let body = match p.protocol {
        Protocol::OpenAi => {
            if let Some(k) = &key {
                headers.push(("Authorization".into(), format!("Bearer {k}")));
            }
            if config.provider == "openrouter" {
                headers.push(("X-Title".into(), "Slimy".into()));
            }
            let mut msgs = vec![json!({"role": "system", "content": system})];
            msgs.extend(turns.iter().map(|(r, t, img)| match img {
                Some(data) => json!({"role": r, "content": [{"type": "text", "text": t}, {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{data}")}}]}),
                None => json!({"role": r, "content": t}),
            }));
            let mut body = json!({"model": config.model, "messages": msgs, "temperature": config.temperature, "max_tokens": config.max_tokens});
            // Reasoning models spend part of the budget thinking: keep it short and leave room for the answer.
            if config.provider == "groq" && config.model.contains("gpt-oss") {
                body["reasoning_effort"] = json!("low");
                body["max_tokens"] = json!(config.max_tokens.max(700));
            }
            body
        }
        Protocol::Anthropic => {
            headers.push(("x-api-key".into(), key.clone().unwrap_or_default()));
            headers.push(("anthropic-version".into(), "2023-06-01".into()));
            let msgs: Vec<Value> = turns.iter().map(|(r, t, img)| match img {
                Some(data) => json!({"role": r, "content": [{"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": data}}, {"type": "text", "text": t}]}),
                None => json!({"role": r, "content": t}),
            }).collect();
            json!({"model": config.model, "system": system, "messages": msgs, "temperature": config.temperature.min(1.0), "max_tokens": config.max_tokens})
        }
        Protocol::Bedrock => {
            headers.push((
                "Authorization".into(),
                format!("Bearer {}", key.clone().unwrap_or_default()),
            ));
            let msgs: Vec<Value> = turns.iter().map(|(r, t, img)| match img {
                Some(data) => json!({"role": r, "content": [{"image": {"format": "jpeg", "source": {"bytes": data}}}, {"text": t}]}),
                None => json!({"role": r, "content": [{"text": t}]}),
            }).collect();
            json!({"system": [{"text": system}], "messages": msgs, "inferenceConfig": {"maxTokens": config.max_tokens, "temperature": config.temperature.min(1.0)}})
        }
    };
    Ok(HttpRequest { url, headers, body })
}

fn provider_message(v: &Value) -> Option<String> {
    let text = v
        .pointer("/error/message")
        .or_else(|| v.get("message"))
        .and_then(Value::as_str)?;
    Some(cut(text, 200))
}

pub fn parse_response(provider: &str, body: &str) -> Result<String, BrainError> {
    let p = info(provider).ok_or(BrainError::UnknownProvider)?;
    let v: Value = serde_json::from_str(body).map_err(|_| BrainError::BadReply)?;
    if let Some(message) = v.get("error").and_then(|_| provider_message(&v)) {
        return Err(BrainError::Provider(message));
    }
    let join = |items: Option<&Vec<Value>>, field: &str| -> String {
        items
            .map(|a| {
                a.iter()
                    .filter_map(|b| b.get(field).and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .concat()
            })
            .unwrap_or_default()
    };
    let text = match p.protocol {
        Protocol::OpenAi => v
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        Protocol::Anthropic => join(v.get("content").and_then(Value::as_array), "text"),
        Protocol::Bedrock => join(
            v.pointer("/output/message/content")
                .and_then(Value::as_array),
            "text",
        ),
    };
    if text.trim().is_empty() {
        return match provider_message(&v) {
            Some(m) => Err(BrainError::Provider(m)),
            None => Err(BrainError::BadReply),
        };
    }
    Ok(text)
}
