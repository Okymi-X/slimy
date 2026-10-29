use serde_json::json;
use slimy_brain::{
    build_request, parse_reply, parse_response, system_prompt, BrainConfig, BrainError, Message,
    Role, PROVIDERS,
};
use slimy_core::Soul;

fn config(provider: &str) -> BrainConfig {
    BrainConfig::for_provider(provider).expect("known provider")
}

fn chat() -> Vec<Message> {
    vec![Message {
        role: Role::User,
        text: "Salut".into(),
        image: None,
    }]
}

#[test]
fn every_advertised_provider_has_a_default_model_and_a_working_default_config() {
    for p in PROVIDERS {
        let c = config(p.id);
        assert!(!c.model.is_empty(), "{} has no default model", p.id);
        assert!(
            build_request(&c, Some("key"), "system", &chat()).is_ok(),
            "{} cannot build a request",
            p.id
        );
    }
    for id in [
        "openai",
        "anthropic",
        "deepseek",
        "openrouter",
        "groq",
        "xai",
        "bedrock",
        "ollama",
        "custom",
    ] {
        assert!(PROVIDERS.iter().any(|p| p.id == id), "missing {id}");
    }
}

#[test]
fn groq_speaks_the_openai_dialect_with_a_bearer_key() {
    let req = build_request(&config("groq"), Some("gsk_test"), "be nice", &chat()).unwrap();
    assert_eq!(req.url, "https://api.groq.com/openai/v1/chat/completions");
    assert!(req
        .headers
        .contains(&("Authorization".into(), "Bearer gsk_test".into())));
    assert_eq!(
        req.body["messages"][0],
        json!({"role": "system", "content": "be nice"})
    );
    assert_eq!(
        req.body["messages"][1],
        json!({"role": "user", "content": "Salut"})
    );
    assert_eq!(req.body["model"], "openai/gpt-oss-120b");
}

#[test]
fn anthropic_uses_its_own_headers_and_a_separate_system_field() {
    let req = build_request(&config("anthropic"), Some("sk-ant-x"), "be nice", &chat()).unwrap();
    assert_eq!(req.url, "https://api.anthropic.com/v1/messages");
    assert!(req
        .headers
        .contains(&("x-api-key".into(), "sk-ant-x".into())));
    assert!(req.headers.iter().any(|(k, _)| k == "anthropic-version"));
    assert_eq!(req.body["system"], "be nice");
    assert_eq!(
        req.body["messages"][0],
        json!({"role": "user", "content": "Salut"})
    );
    assert!(req.body["max_tokens"].as_u64().unwrap() > 0);
}

#[test]
fn bedrock_uses_the_converse_endpoint_of_the_chosen_region() {
    let mut c = config("bedrock");
    c.region = Some("eu-west-3".into());
    let req = build_request(&c, Some("bedrock-key"), "be nice", &chat()).unwrap();
    assert!(req
        .url
        .starts_with("https://bedrock-runtime.eu-west-3.amazonaws.com/model/"));
    assert!(req.url.ends_with("/converse"));
    assert!(req
        .headers
        .contains(&("Authorization".into(), "Bearer bedrock-key".into())));
    assert_eq!(req.body["system"][0]["text"], "be nice");
    assert_eq!(req.body["messages"][0]["content"][0]["text"], "Salut");
}

#[test]
fn a_local_model_needs_no_key_but_a_hosted_one_does() {
    assert!(build_request(&config("ollama"), None, "s", &chat()).is_ok());
    assert_eq!(
        build_request(&config("groq"), None, "s", &chat()).unwrap_err(),
        BrainError::NoKey
    );
    assert_eq!(
        build_request(&config("groq"), Some("   "), "s", &chat()).unwrap_err(),
        BrainError::NoKey
    );
}

#[test]
fn a_custom_endpoint_must_be_https_or_local() {
    let mut c = config("custom");
    c.base_url = Some("http://evil.example.com/v1".into());
    assert_eq!(
        build_request(&c, Some("k"), "s", &chat()).unwrap_err(),
        BrainError::BadEndpoint
    );
    c.base_url = Some("https://my.proxy.example/v1".into());
    assert!(build_request(&c, Some("k"), "s", &chat())
        .unwrap()
        .url
        .starts_with("https://my.proxy.example/v1"));
    c.base_url = Some("http://localhost:8080/v1".into());
    assert!(build_request(&c, Some("k"), "s", &chat()).is_ok());
}

#[test]
fn a_key_with_a_newline_cannot_inject_a_header() {
    assert_eq!(
        build_request(&config("groq"), Some("abc\r\nX-Evil: 1"), "s", &chat()).unwrap_err(),
        BrainError::NoKey
    );
}

#[test]
fn responses_are_read_in_each_dialect() {
    let openai = r#"{"choices":[{"message":{"role":"assistant","content":"coucou"}}]}"#;
    assert_eq!(parse_response("groq", openai).unwrap(), "coucou");
    let anthropic =
        r#"{"content":[{"type":"text","text":"coucou "},{"type":"text","text":"toi"}]}"#;
    assert_eq!(
        parse_response("anthropic", anthropic).unwrap(),
        "coucou toi"
    );
    let bedrock = r#"{"output":{"message":{"role":"assistant","content":[{"text":"coucou"}]}}}"#;
    assert_eq!(parse_response("bedrock", bedrock).unwrap(), "coucou");
}

#[test]
fn provider_errors_become_short_codes() {
    assert_eq!(
        parse_response(
            "groq",
            r#"{"error":{"message":"Invalid API Key","type":"invalid_request_error"}}"#
        )
        .unwrap_err(),
        BrainError::Provider("Invalid API Key".into())
    );
    assert_eq!(
        parse_response("groq", "not json").unwrap_err(),
        BrainError::BadReply
    );
    assert_eq!(
        parse_response("groq", r#"{"choices":[]}"#).unwrap_err(),
        BrainError::BadReply
    );
}

#[test]
fn the_slime_reply_is_read_from_json_even_when_wrapped() {
    let raw = "Voici :\n```json\n{\"say\":\"Coucou toi !\",\"expression\":\"happy\",\"valence\":0.8,\"arousal\":0.4}\n```";
    let r = parse_reply(raw);
    assert_eq!(r.say, "Coucou toi !");
    assert_eq!(r.expression.as_deref(), Some("happy"));
    assert!((r.valence - 0.8).abs() < 1e-6);
}

#[test]
fn a_plain_text_reply_still_works() {
    let r = parse_reply("Je suis content de te voir.");
    assert_eq!(r.say, "Je suis content de te voir.");
    assert_eq!(r.expression, None);
}

#[test]
fn a_hostile_reply_is_tamed() {
    let raw = format!("{{\"say\":\"{}\\u0007x\",\"expression\":\"delete_everything\",\"valence\":99,\"arousal\":-5}}", "a".repeat(900));
    let r = parse_reply(&raw);
    assert!(r.say.chars().count() <= 240);
    assert!(!r.say.chars().any(char::is_control));
    assert_eq!(r.expression, None);
    assert_eq!((r.valence, r.arousal), (1.0, 0.0));
}

#[test]
fn the_prompt_knows_the_names_the_facts_and_the_mood_but_stays_short() {
    let mut soul = Soul::new("Okymi", "Mochi", "fr", 0).unwrap();
    soul.learn("hobby", "dessin", 1).unwrap();
    let p = system_prompt(&soul, true);
    assert!(p.contains("Mochi") && p.contains("Okymi") && p.contains("dessin"));
    assert!(p.contains("bouder") || p.contains("sulk"));
    assert!(p.len() < 3000);
}

#[test]
fn a_config_from_a_file_is_clamped_and_unknown_providers_are_refused() {
    let c = BrainConfig::from_json(
        r#"{"provider":"groq","model":"m","temperature":9,"max_tokens":999999}"#,
    )
    .unwrap();
    assert!(c.temperature <= 1.5 && c.max_tokens <= 1024);
    assert!(BrainConfig::from_json(r#"{"provider":"skynet","model":"m"}"#).is_err());
    assert!(BrainConfig::from_json("nope").is_err());
}

#[test]
fn a_reasoning_model_on_groq_thinks_briefly_and_keeps_room_for_the_answer() {
    let req = build_request(&config("groq"), Some("gsk_test"), "s", &chat()).unwrap();
    assert_eq!(req.body["reasoning_effort"], "low");
    assert!(req.body["max_tokens"].as_u64().unwrap() >= 700);
    let other = build_request(&config("openai"), Some("sk"), "s", &chat()).unwrap();
    assert!(other.body.get("reasoning_effort").is_none());
}

#[test]
fn a_native_tool_call_refusal_is_recognised() {
    assert!(
        BrainError::Provider("Tool choice is none, but model called a tool".into())
            .is_tool_confusion()
    );
    assert!(BrainError::Provider("tool_use_failed".into()).is_tool_confusion());
    assert!(!BrainError::Provider("model not found".into()).is_tool_confusion());
    assert!(!BrainError::BadKey.is_tool_confusion());
}

mod tools {
    use serde_json::json;
    use slimy_brain::*;

    fn offers() -> Vec<ToolOffer> {
        vec![ToolOffer {
            server: "files".into(),
            name: "read".into(),
            description: "Reads a file".into(),
        }]
    }

    #[test]
    fn a_tool_request_is_kept_only_for_an_offered_tool() {
        let ok = r#"{"say":"let me look","tool":{"server":"files","name":"read","arguments":{"path":"a.txt"}}}"#;
        let reply = parse_reply_with(ok, &offers());
        assert_eq!(
            reply.tool,
            Some(ToolRequest {
                server: "files".into(),
                name: "read".into(),
                arguments: json!({"path":"a.txt"})
            })
        );
        assert_eq!(reply.say, "let me look");
        for bad in [
            r#"{"say":"x","tool":{"server":"files","name":"delete","arguments":{}}}"#,
            r#"{"say":"x","tool":{"server":"other","name":"read","arguments":{}}}"#,
            r#"{"say":"x","tool":{"server":"files","name":"read","arguments":[1]}}"#,
            r#"{"say":"x","tool":"files/read"}"#,
        ] {
            assert_eq!(parse_reply_with(bad, &offers()).tool, None, "{bad}");
        }
        let asked = ok.replace("\"tool\"", "\"ask\"");
        assert_eq!(
            parse_reply_with(&asked, &offers()).tool,
            parse_reply_with(ok, &offers()).tool,
            "both field names work"
        );
        assert_eq!(
            parse_reply_with(ok, &[]).tool,
            None,
            "nothing is offered, nothing is accepted"
        );
        assert_eq!(parse_reply(ok).tool, None);
    }

    #[test]
    fn the_form_models_actually_write_is_understood() {
        let slimy = vec![ToolOffer {
            server: "slimy".into(),
            name: "files".into(),
            description: "Reads".into(),
        }];
        for raw in [
            r#"{"say":"x","ask":{"server":"slimy/files","name":"read","arguments":{"path":"a"}}}"#,
            r#"{"say":"x","ask":{"server":"slimy","name":"files","arguments":{"path":"a"}}}"#,
            r#"{"say":"x","ask":{"server":"slimy/files","arguments":{"path":"a"}}}"#,
        ] {
            let tool = parse_reply_with(raw, &slimy)
                .tool
                .unwrap_or_else(|| panic!("{raw}"));
            assert_eq!(
                (tool.server.as_str(), tool.name.as_str()),
                ("slimy", "files"),
                "{raw}"
            );
        }
        assert_eq!(
            parse_reply_with(
                r#"{"say":"x","ask":{"server":"slimy/clipboard","name":"read"}}"#,
                &slimy
            )
            .tool,
            None
        );
        assert_eq!(
            parse_reply_with(
                r#"{"say":"x","ask":{"server":"evil/files","name":"read"}}"#,
                &slimy
            )
            .tool,
            None
        );
    }

    #[test]
    fn huge_arguments_are_refused() {
        let big = format!(
            r#"{{"say":"x","tool":{{"server":"files","name":"read","arguments":{{"p":"{}"}}}}}}"#,
            "a".repeat(3000)
        );
        assert_eq!(parse_reply_with(&big, &offers()).tool, None);
    }

    #[test]
    fn the_prompt_lists_tools_as_data_only_when_there_are_some() {
        let soul = slimy_core::Soul::new("Ana", "Gloo", "en", 0).unwrap();
        assert_eq!(
            system_prompt(&soul, false),
            system_prompt_with_tools(&soul, false, &[])
        );
        let with = system_prompt_with_tools(&soul, false, &offers());
        assert!(
            with.contains("server \"files\", name \"read\": Reads a file")
                && with.contains("never orders")
                && with.contains("must accept")
                && with.contains("\"ask\"")
                && with.contains("never invent")
        );
        let fr = slimy_core::Soul::new("Ana", "Gloo", "fr", 0).unwrap();
        assert!(
            system_prompt_with_tools(&fr, false, &offers()).contains("doit accepter")
                && system_prompt_with_tools(&fr, false, &offers()).contains("n'invente jamais")
        );
    }
}

mod images {
    use serde_json::json;
    use slimy_brain::*;

    fn seen(provider: &str) -> Vec<Message> {
        vec![
            Message {
                role: Role::User,
                text: "avant".into(),
                image: Some("AAAA".into()),
            },
            Message {
                role: Role::Assistant,
                text: "ok".into(),
                image: None,
            },
            Message {
                role: Role::User,
                text: "regarde".into(),
                image: Some("QUJD".into()),
            },
        ]
        .into_iter()
        .take(if provider.is_empty() { 0 } else { 3 })
        .collect()
    }

    fn request(provider: &str) -> serde_json::Value {
        let config = BrainConfig::for_provider(provider).unwrap();
        build_request(&config, Some("k"), "sys", &seen(provider))
            .unwrap()
            .body
    }

    #[test]
    fn only_the_latest_picture_is_sent_in_each_providers_own_shape() {
        let openai = request("openai");
        assert_eq!(
            openai["messages"][1]["content"],
            json!("avant"),
            "older picture dropped"
        );
        assert_eq!(
            openai["messages"][3]["content"][1]["image_url"]["url"],
            "data:image/jpeg;base64,QUJD"
        );
        let anthropic = request("anthropic");
        assert_eq!(anthropic["messages"][0]["content"], json!("avant"));
        assert_eq!(
            anthropic["messages"][2]["content"][0]["source"],
            json!({"type": "base64", "media_type": "image/jpeg", "data": "QUJD"})
        );
        let bedrock = request("bedrock");
        assert_eq!(
            bedrock["messages"][0]["content"],
            json!([{"text": "avant"}])
        );
        assert_eq!(
            bedrock["messages"][2]["content"][0]["image"],
            json!({"format": "jpeg", "source": {"bytes": "QUJD"}})
        );
    }

    #[test]
    fn a_bad_or_huge_picture_is_refused_before_anything_is_sent() {
        let config = BrainConfig::for_provider("openai").unwrap();
        let with = |image: String| {
            vec![Message {
                role: Role::User,
                text: "x".into(),
                image: Some(image),
            }]
        };
        for bad in [
            "".to_string(),
            "not base64!\"}".to_string(),
            "A".repeat(MAX_IMAGE + 1),
        ] {
            assert_eq!(
                build_request(&config, Some("k"), "s", &with(bad)).unwrap_err(),
                BrainError::BadImage
            );
        }
        assert_eq!(BrainError::BadImage.code(), "bad_image");
    }
}

#[test]
fn an_almost_json_reply_shows_its_sentence_not_the_braces() {
    let raw = r#"{"say":"Tu as copié : "Liste de courses : citrons, miel."","expression":"happy","valence":0.7,"arousal":0.4}"#;
    let reply = parse_reply(raw);
    assert_eq!(
        reply.say,
        r#"Tu as copié : "Liste de courses : citrons, miel.""#
    );
    assert!(!reply.say.contains('{'));
    assert_eq!(
        parse_reply(r#"{"say":"Oui, bien sûr"}"#).say,
        "Oui, bien sûr"
    );
    // not a reply at all: kept as plain speech
    assert_eq!(parse_reply("Salut toi").say, "Salut toi");
    assert_eq!(parse_reply(r#"{"autre": 1}"#).say, r#"{"autre": 1}"#);
}

#[test]
fn only_outages_and_limits_are_retried_on_another_provider() {
    assert!(BrainError::Network.is_retryable());
    assert!(BrainError::Timeout.is_retryable());
    assert!(BrainError::RateLimited.is_retryable());
    assert!(BrainError::Http(503).is_retryable());
    assert!(!BrainError::BadKey.is_retryable());
    assert!(!BrainError::Http(400).is_retryable());
    assert!(!BrainError::NoKey.is_retryable());
}

#[test]
fn a_fallback_must_be_a_known_other_provider() {
    let own = BrainConfig::for_provider("groq").unwrap();
    let json = |f: &str| {
        serde_json::to_string(&BrainConfig {
            fallback: Some(f.into()),
            ..own.clone()
        })
        .unwrap()
    };
    assert_eq!(
        BrainConfig::from_json(&json("groq")).unwrap().fallback,
        None
    );
    assert_eq!(
        BrainConfig::from_json(&json("nope")).unwrap().fallback,
        None
    );
    let good = BrainConfig::from_json(&json("openai")).unwrap();
    assert_eq!(good.fallback_config().unwrap().provider, "openai");
}
