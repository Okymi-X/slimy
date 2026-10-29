//! Talks to the MCP servers the user declared in `mcp.json`, one short session per request: start the
//! server, handshake, ask, stop it. Only the user writes that file; the page can name a server but
//! never gives a command.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;
use slimy_mcp::{read_line, Incoming, McpError, Server};

const FILE: &str = "mcp.json";
/// Whole session budget: start, handshake and answer.
const TIMEOUT: Duration = Duration::from_secs(20);

pub fn load(dir: &Path) -> Result<Vec<Server>, McpError> {
    match std::fs::read_to_string(dir.join(FILE)) {
        Ok(text) => slimy_mcp::parse_servers(&text),
        Err(_) => Ok(Vec::new()),
    }
}

pub fn find(dir: &Path, name: &str) -> Result<Server, String> {
    load(dir)
        .map_err(|_| "mcp_config".to_string())?
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| "mcp_unknown".to_string())
}

struct Session {
    child: Child,
    lines: mpsc::Receiver<String>,
    deadline: Instant,
}

impl Session {
    fn start(server: &Server) -> Result<Self, String> {
        let mut child = Command::new(&server.command)
            .args(&server.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "mcp_start".to_string())?;
        let out = child.stdout.take().ok_or("mcp_start")?;
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Session {
            child,
            lines,
            deadline: Instant::now() + TIMEOUT,
        })
    }

    fn send(&mut self, text: &str) -> Result<(), String> {
        let stdin = self.child.stdin.as_mut().ok_or("mcp_io")?;
        stdin
            .write_all(text.as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|_| "mcp_io".to_string())
    }

    fn ask(&mut self, id: u64, request: &str) -> Result<Value, String> {
        self.send(request)?;
        loop {
            let left = self.deadline.saturating_duration_since(Instant::now());
            let line = self
                .lines
                .recv_timeout(left)
                .map_err(|_| "mcp_timeout".to_string())?;
            if let Incoming::Reply(reply) = read_line(&line, id) {
                return reply.map_err(|e| match e {
                    McpError::Server(m) => format!("mcp_server:{m}"),
                    _ => "mcp_reply".to_string(),
                });
            }
        }
    }

    fn open(server: &Server) -> Result<Self, String> {
        let mut session = Self::start(server)?;
        session.ask(1, &slimy_mcp::initialize(1))?;
        session.send(&slimy_mcp::initialized())?;
        Ok(session)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn tools(server: &Server) -> Result<Vec<slimy_mcp::Tool>, String> {
    let mut session = Session::open(server)?;
    let result = session.ask(2, &slimy_mcp::list_tools(2))?;
    slimy_mcp::tools_of(&result).map_err(|_| "mcp_reply".to_string())
}

/// The tool's text answer and whether the tool reported a failure.
pub fn call(server: &Server, tool: &str, arguments: Value) -> Result<(String, bool), String> {
    let request = slimy_mcp::call_tool(2, tool, arguments).map_err(|_| "mcp_name".to_string())?;
    let mut session = Session::open(server)?;
    let result = session.ask(2, &request)?;
    slimy_mcp::answer_of(&result).map_err(|_| "mcp_reply".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE: &str = r#"while read l; do case "$l" in
      *'"initialize"'*) echo '{"jsonrpc":"2.0","method":"notifications/message","params":{}}'; echo '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"f","version":"1"}}}';;
      *'tools/list'*) echo '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"Echo"}]}}';;
      *'tools/call'*) echo '{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}],"isError":false}}';;
    esac; done"#;

    fn fake() -> Server {
        Server {
            name: "fake".into(),
            command: "sh".into(),
            args: vec!["-c".into(), FAKE.into()],
        }
    }

    #[test]
    fn a_server_lists_its_tools_and_answers_a_call() {
        let tools = tools(&fake()).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        assert_eq!(
            call(&fake(), "echo", serde_json::json!({})).unwrap(),
            ("pong".to_string(), false)
        );
    }

    #[test]
    fn a_missing_command_or_a_silent_server_fails_with_a_short_code() {
        let missing = Server {
            name: "x".into(),
            command: "definitely-not-a-command".into(),
            args: vec![],
        };
        assert_eq!(tools(&missing).unwrap_err(), "mcp_start");
        assert_eq!(
            call(&fake(), "../bad", serde_json::json!({})).unwrap_err(),
            "mcp_name"
        );
    }

    #[test]
    fn no_file_means_no_servers() {
        assert!(load(Path::new("/nonexistent-slimy-dir"))
            .unwrap()
            .is_empty());
        assert_eq!(
            find(Path::new("/nonexistent-slimy-dir"), "a").unwrap_err(),
            "mcp_unknown"
        );
    }
}
