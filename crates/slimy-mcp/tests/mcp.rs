use serde_json::json;
use slimy_mcp::*;

#[test]
fn the_server_list_is_validated() {
    let ok = r#"{"servers":[{"name":"files","command":"mcp-files","args":["--root","/tmp"]},{"name":"time","command":"mcp-time"}]}"#;
    let servers = parse_servers(ok).unwrap();
    assert_eq!(servers.len(), 2);
    assert!(servers[1].args.is_empty());
    assert_eq!(parse_servers("nope"), Err(McpError::BadConfig));
    assert_eq!(
        parse_servers(r#"{"servers":[{"name":"a b","command":"x"}]}"#),
        Err(McpError::BadName)
    );
    assert_eq!(
        parse_servers(r#"{"servers":[{"name":"a","command":"x"},{"name":"a","command":"y"}]}"#),
        Err(McpError::BadName)
    );
    assert_eq!(
        parse_servers(r#"{"servers":[{"name":"a","command":"  "}]}"#),
        Err(McpError::BadConfig)
    );
    assert_eq!(
        parse_servers(r#"{"servers":[{"name":"slimy","command":"x"}]}"#),
        Err(McpError::BadName)
    );
    let many = format!(
        r#"{{"servers":[{}]}}"#,
        (0..9)
            .map(|i| format!(r#"{{"name":"s{i}","command":"x"}}"#))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(parse_servers(&many), Err(McpError::BadConfig));
}

#[test]
fn messages_are_one_json_line_each() {
    for text in [
        initialize(1),
        initialized(),
        list_tools(2),
        call_tool(3, "read", json!({"path":"a"})).unwrap(),
    ] {
        assert!(text.ends_with('\n') && text.matches('\n').count() == 1);
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
    }
    assert_eq!(call_tool(1, "../x", json!({})), Err(McpError::BadName));
    assert_eq!(call_tool(1, "read", json!([1])), Err(McpError::BadName));
}

#[test]
fn only_the_reply_with_the_right_id_is_taken() {
    assert_eq!(
        read_line(
            r#"{"jsonrpc":"2.0","method":"notifications/message","params":{}}"#,
            1
        ),
        Incoming::Other
    );
    assert_eq!(read_line("garbage", 1), Incoming::Other);
    assert_eq!(
        read_line(r#"{"jsonrpc":"2.0","id":2,"result":{}}"#, 1),
        Incoming::Other
    );
    assert_eq!(
        read_line(r#"{"jsonrpc":"2.0","id":1,"result":{"a":1}}"#, 1),
        Incoming::Reply(Ok(json!({"a":1})))
    );
    assert_eq!(
        read_line(
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"boom"}}"#,
            1
        ),
        Incoming::Reply(Err(McpError::Server("boom".into())))
    );
    assert_eq!(
        read_line(r#"{"jsonrpc":"2.0","id":1}"#, 1),
        Incoming::Reply(Err(McpError::BadReply))
    );
    // a server asking something of us (a request with an id) is never taken for our reply
    assert_eq!(
        read_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"sampling/createMessage"}"#,
            1
        ),
        Incoming::Other
    );
}

#[test]
fn tools_and_answers_are_cleaned_and_bounded() {
    let result = json!({"tools":[{"name":"read","description":"Reads"},{"name":"bad name","description":"x"},{"name":"long","description":"d".repeat(1000)}]});
    let tools = tools_of(&result).unwrap();
    assert_eq!(
        tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        ["read", "long"]
    );
    assert_eq!(tools[1].description.chars().count(), MAX_DESCRIPTION);
    assert_eq!(tools_of(&json!({})), Err(McpError::BadReply));

    let answer = json!({"content":[{"type":"text","text":"one"},{"type":"image","data":"AAAA"},{"type":"text","text":"two"}],"isError":true});
    assert_eq!(answer_of(&answer).unwrap(), ("one\ntwo".to_string(), true));
    let big = json!({"content":[{"type":"text","text":"x".repeat(10_000)}]});
    let (text, failed) = answer_of(&big).unwrap();
    assert_eq!(text.chars().count(), MAX_ANSWER);
    assert!(!failed);
    assert_eq!(answer_of(&json!({"content":"no"})), Err(McpError::BadReply));
}
