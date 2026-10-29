//! Sends one conversation turn to the chosen provider and returns what the slime says.

use std::path::Path;
use std::time::Duration;

use slimy_brain::{
    build_request, parse_reply_with, parse_response, system_prompt_with_tools, BrainConfig,
    BrainError, Message, Reply, ToolOffer,
};
use slimy_core::Soul;

use crate::secrets;

const FILE: &str = "brain.json";
const MAX_BODY: usize = 1_000_000;
const DEFAULT_PROVIDER: &str = "groq";

pub fn load_config(dir: &Path) -> BrainConfig {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|text| BrainConfig::from_json(&text).ok())
        .or_else(|| BrainConfig::for_provider(DEFAULT_PROVIDER))
        .expect("the default provider exists")
}

pub fn save_config(dir: &Path, config: &BrainConfig) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let staging = dir.join("brain.json.tmp");
    std::fs::write(
        &staging,
        serde_json::to_string_pretty(config).expect("a config serializes"),
    )?;
    std::fs::rename(staging, dir.join(FILE))
}

fn from_status(status: u16, body: &str, provider: &str) -> BrainError {
    match status {
        401 | 403 => BrainError::BadKey,
        429 => BrainError::RateLimited,
        _ => match parse_response(provider, body) {
            Err(BrainError::Provider(message)) => BrainError::Provider(message),
            _ => BrainError::Http(status),
        },
    }
}

pub async fn chat(
    config: &BrainConfig,
    soul: &Soul,
    sulking: bool,
    history: &[Message],
    offers: &[ToolOffer],
) -> Result<Reply, BrainError> {
    let key = secrets::load(&config.provider).await;
    let request = build_request(
        config,
        key.as_deref(),
        &system_prompt_with_tools(soul, sulking, offers),
        history,
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| BrainError::Network)?;
    let mut call = client.post(&request.url).json(&request.body);
    for (name, value) in &request.headers {
        call = call.header(name, value);
    }
    let response = call.send().await.map_err(|e| {
        if e.is_timeout() {
            BrainError::Timeout
        } else {
            BrainError::Network
        }
    })?;
    let status = response.status().as_u16();
    let bytes = response.bytes().await.map_err(|_| BrainError::Network)?;
    if bytes.len() > MAX_BODY {
        return Err(BrainError::BadReply);
    }
    let body = String::from_utf8_lossy(&bytes);
    if !(200..300).contains(&status) {
        return Err(from_status(status, &body, &config.provider));
    }
    parse_response(&config.provider, &body).map(|text| {
        // Development aid: shows exactly what the model wrote (never the request, so never a key).
        if std::env::var_os("SLIMY_DEBUG_RAW").is_some() {
            eprintln!("raw reply: {text}");
        }
        parse_reply_with(&text, offers)
    })
}
