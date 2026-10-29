//! The slime's brain: talks to the language model the user chose. Several providers, one small
//! interface. Everything here is pure (requests are built and replies parsed without any I/O), so
//! it is tested without a network; the app does the sending.

mod provider;
mod reply;
mod request;

pub use provider::{BrainConfig, Protocol, ProviderInfo, PROVIDERS};
pub use reply::{
    parse_reply, parse_reply_with, system_prompt, system_prompt_with_tools, Reply, ToolOffer,
    ToolRequest, EXPRESSIONS,
};
pub use request::{
    build_request, parse_response, BrainError, HttpRequest, Message, Role, MAX_IMAGE,
};
