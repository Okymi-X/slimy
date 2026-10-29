//! The Model Context Protocol over stdio, as pure data: the messages the slime sends, how replies are
//! read, and how the user's server list is validated. No process and no I/O here, so every rule is testable.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const MAX_SERVERS: usize = 8;
pub const MAX_ARGS: usize = 16;
pub const MAX_NAME: usize = 32;
pub const MAX_TOOLS: usize = 64;
/// Longest tool answer kept, in characters: the slime reads a summary, not a file dump.
/// The name of the slime's own powers, which no declared server may take.
pub const RESERVED: &str = "slimy";
pub const MAX_ANSWER: usize = 4000;
pub const MAX_DESCRIPTION: usize = 200;

#[derive(Debug, PartialEq, Eq)]
pub enum McpError {
    /// The server list is not valid JSON or breaks a limit.
    BadConfig,
    /// A name or argument is not allowed.
    BadName,
    /// The server answered with an error.
    Server(String),
    /// The answer is not what the protocol says.
    BadReply,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
}

/// Names that identify a server or a tool: short, with letters, digits, dash, underscore and dot.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Reads the user's `mcp.json`: `{ "servers": [ { "name", "command", "args" } ] }`.
pub fn parse_servers(text: &str) -> Result<Vec<Server>, McpError> {
    #[derive(Deserialize)]
    struct File {
        servers: Vec<Server>,
    }
    let file: File = serde_json::from_str(text).map_err(|_| McpError::BadConfig)?;
    if file.servers.len() > MAX_SERVERS {
        return Err(McpError::BadConfig);
    }
    for (i, s) in file.servers.iter().enumerate() {
        if !valid_name(&s.name)
            || s.name == RESERVED
            || file.servers[..i].iter().any(|o| o.name == s.name)
        {
            return Err(McpError::BadName);
        }
        if s.command.trim().is_empty()
            || s.command.contains('\0')
            || s.args.len() > MAX_ARGS
            || s.args.iter().any(|a| a.contains('\0'))
        {
            return Err(McpError::BadConfig);
        }
    }
    Ok(file.servers)
}

fn line(value: Value) -> String {
    let mut text = value.to_string();
    text.push('\n');
    text
}

pub fn initialize(id: u64) -> String {
    line(
        json!({ "jsonrpc": "2.0", "id": id, "method": "initialize", "params": {
        "protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": { "name": "slimy", "version": "0.1.0" } } }),
    )
}

pub fn initialized() -> String {
    line(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
}

pub fn list_tools(id: u64) -> String {
    line(json!({ "jsonrpc": "2.0", "id": id, "method": "tools/list" }))
}

/// A tool call. The tool name is checked; the arguments must be a JSON object.
pub fn call_tool(id: u64, name: &str, arguments: Value) -> Result<String, McpError> {
    if !valid_name(name) || !arguments.is_object() {
        return Err(McpError::BadName);
    }
    Ok(line(
        json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": arguments } }),
    ))
}

/// The result of one incoming line.
#[derive(Debug, PartialEq)]
pub enum Incoming {
    /// The reply to the request with this id.
    Reply(Result<Value, McpError>),
    /// A notification, a log line or another id: keep reading.
    Other,
}

pub fn read_line(text: &str, id: u64) -> Incoming {
    let Ok(message) = serde_json::from_str::<Value>(text.trim()) else {
        return Incoming::Other;
    };
    if message.get("id").and_then(Value::as_u64) != Some(id) || message.get("method").is_some() {
        return Incoming::Other;
    }
    if let Some(error) = message.get("error") {
        let text = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("error");
        return Incoming::Reply(Err(McpError::Server(
            text.chars().take(MAX_DESCRIPTION).collect(),
        )));
    }
    match message.get("result") {
        Some(result) => Incoming::Reply(Ok(result.clone())),
        None => Incoming::Reply(Err(McpError::BadReply)),
    }
}

/// The tools of a `tools/list` result, with names validated and descriptions cut.
pub fn tools_of(result: &Value) -> Result<Vec<Tool>, McpError> {
    let list = result
        .get("tools")
        .and_then(Value::as_array)
        .ok_or(McpError::BadReply)?;
    Ok(list
        .iter()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?;
            valid_name(name).then(|| Tool {
                name: name.to_string(),
                description: t
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .chars()
                    .take(MAX_DESCRIPTION)
                    .collect(),
            })
        })
        .take(MAX_TOOLS)
        .collect())
}

/// The text of a `tools/call` result and whether the tool reported a failure. Only text blocks are kept.
pub fn answer_of(result: &Value) -> Result<(String, bool), McpError> {
    let blocks = result
        .get("content")
        .and_then(Value::as_array)
        .ok_or(McpError::BadReply)?;
    let text: String = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    Ok((
        text.chars().take(MAX_ANSWER).collect(),
        result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    ))
}
