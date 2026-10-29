use slimy_core::{export, import, summary, SlimeFileError, Soul, MAX_FILE_BYTES};

fn soul() -> Soul {
    let mut s = Soul::new("Okymi", "Mochi", "fr", 42).unwrap();
    s.learn("color", "green", 7).unwrap();
    s
}

fn tweak(f: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&export(&soul(), 1)).unwrap();
    f(&mut v);
    v.to_string()
}

#[test]
fn round_trip_keeps_the_soul() {
    let back = import(&export(&soul(), 1)).unwrap();
    assert_eq!(back, soul());
    assert_eq!(summary(&back).facts, 1);
}

#[test]
fn only_the_soul_fields_are_exported() {
    let v: serde_json::Value = serde_json::from_str(&export(&soul(), 1)).unwrap();
    let mut keys: Vec<_> = v["soul"].as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "born_at",
            "facts",
            "genome",
            "language",
            "relation",
            "slime_name",
            "tone",
            "user_name"
        ]
    );
}

#[test]
fn out_of_range_genes_are_refused_not_repaired() {
    let bad = tweak(|v| v["soul"]["genome"]["genes"][3] = serde_json::json!(1.5));
    assert_eq!(import(&bad), Err(SlimeFileError::OutOfRange));
    let neg = tweak(|v| v["soul"]["genome"]["genes"][0] = serde_json::json!(-0.1));
    assert_eq!(import(&neg), Err(SlimeFileError::OutOfRange));
    let short = tweak(|v| {
        v["soul"]["genome"]["genes"].as_array_mut().unwrap().pop();
    });
    assert_eq!(import(&short), Err(SlimeFileError::OutOfRange));
}

#[test]
fn version_format_and_unknown_fields_are_refused() {
    assert_eq!(
        import(&tweak(|v| v["version"] = 2.into())),
        Err(SlimeFileError::BadVersion)
    );
    assert_eq!(
        import(&tweak(|v| v["format"] = "x".into())),
        Err(SlimeFileError::BadFormat)
    );
    assert_eq!(
        import(&tweak(|v| v["extra"] = 1.into())),
        Err(SlimeFileError::Malformed)
    );
    assert_eq!(
        import(&tweak(|v| v["soul"]["api_key"] = "sk".into())),
        Err(SlimeFileError::Malformed)
    );
    assert_eq!(import("not json"), Err(SlimeFileError::Malformed));
}

#[test]
fn bad_tone_names_language_and_facts_are_refused() {
    assert!(import(&tweak(|v| v["soul"]["tone"]["energy"] = 2.into())).is_err());
    assert!(import(&tweak(|v| v["soul"]["relation"]["trust"] = (-1).into())).is_err());
    assert!(import(&tweak(|v| v["soul"]["slime_name"] = "a\nb".into())).is_err());
    assert!(import(&tweak(|v| v["soul"]["language"] = "de".into())).is_err());
    assert!(import(&tweak(
        |v| v["soul"]["facts"][0]["value"] = "x".repeat(500).into()
    ))
    .is_err());
    let dup = tweak(|v| {
        let f = v["soul"]["facts"][0].clone();
        v["soul"]["facts"].as_array_mut().unwrap().push(f);
    });
    assert!(import(&dup).is_err());
}

#[test]
fn history_deltas_must_be_in_range() {
    let bad = tweak(|v| {
        v["soul"]["genome"]["history"] = serde_json::json!([{ "night": 1, "seed": 1, "deltas": [[200, 0.1]], "mutation": null }]);
    });
    assert_eq!(import(&bad), Err(SlimeFileError::OutOfRange));
}

#[test]
fn oversized_input_is_refused() {
    assert_eq!(
        import(&" ".repeat(MAX_FILE_BYTES + 1)),
        Err(SlimeFileError::TooLarge)
    );
}
