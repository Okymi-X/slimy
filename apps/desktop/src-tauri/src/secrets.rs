//! API keys live in the system keyring (Secret Service on Linux, Credential Manager on Windows, Keychain on macOS), never in a file of ours. The environment
//! is a read-only fallback for people who prefer it. Keys are never logged or sent to the page.

use std::collections::HashMap;

const SERVICE: &str = "slimy";

#[cfg(target_os = "linux")]
fn attributes(provider: &str) -> HashMap<&str, &str> {
    HashMap::from([("application", SERVICE), ("provider", provider)])
}

fn env_names(provider: &str) -> Vec<String> {
    let mut names = vec![format!("SLIMY_KEY_{}", provider.to_uppercase())];
    let known = match provider {
        "groq" => "GROQ_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "xai" => "XAI_API_KEY",
        "bedrock" => "AWS_BEARER_TOKEN_BEDROCK",
        _ => "",
    };
    if !known.is_empty() {
        names.push(known.to_owned());
    }
    names
}

#[cfg(target_os = "linux")]
pub async fn save(provider: &str, key: &str) -> Result<(), String> {
    let keyring = oo7::Keyring::new()
        .await
        .map_err(|_| "keyring_unavailable".to_string())?;
    keyring
        .create_item(
            &format!("Slimy {provider} key"),
            &attributes(provider),
            key.trim().as_bytes(),
            true,
        )
        .await
        .map_err(|_| "keyring_unavailable".to_string())
}

#[cfg(target_os = "linux")]
pub async fn load(provider: &str) -> Option<String> {
    if let Ok(keyring) = oo7::Keyring::new().await {
        if let Ok(items) = keyring.search_items(&attributes(provider)).await {
            if let Some(item) = items.first() {
                if let Ok(secret) = item.secret().await {
                    if let Ok(text) = String::from_utf8(secret.to_vec()) {
                        return Some(text);
                    }
                }
            }
        }
    }
    from_env(provider)
}

#[cfg(target_os = "linux")]
pub async fn remove(provider: &str) -> Result<(), String> {
    let keyring = oo7::Keyring::new()
        .await
        .map_err(|_| "keyring_unavailable".to_string())?;
    keyring
        .delete(&attributes(provider))
        .await
        .map_err(|_| "keyring_unavailable".to_string())
}

#[cfg(not(target_os = "linux"))]
fn entry(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, provider).map_err(|_| "keyring_unavailable".to_string())
}

#[cfg(not(target_os = "linux"))]
pub async fn save(provider: &str, key: &str) -> Result<(), String> {
    entry(provider)?
        .set_password(key.trim())
        .map_err(|_| "keyring_unavailable".to_string())
}

#[cfg(not(target_os = "linux"))]
pub async fn load(provider: &str) -> Option<String> {
    if let Ok(text) = entry(provider).and_then(|e| e.get_password().map_err(|_| String::new())) {
        return Some(text);
    }
    from_env(provider)
}

#[cfg(not(target_os = "linux"))]
pub async fn remove(provider: &str) -> Result<(), String> {
    entry(provider)?
        .delete_credential()
        .map_err(|_| "keyring_unavailable".to_string())
}

fn from_env(provider: &str) -> Option<String> {
    env_names(provider)
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|v| !v.trim().is_empty()))
}
