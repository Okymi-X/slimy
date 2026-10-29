use slimy_core::Soul;

#[test]
fn a_remembered_fact_can_be_forgotten_and_only_that_one() {
    let mut soul = Soul::new("Ana", "Gloo", "fr", 0).unwrap();
    soul.learn("favorite_color", "blue", 1).unwrap();
    soul.learn("pet", "cat", 2).unwrap();
    assert!(soul.forget("favorite_color"));
    assert!(!soul.forget("favorite_color"), "already gone");
    assert!(!soul.forget("nothing"));
    assert_eq!(
        soul.facts
            .iter()
            .map(|f| f.key.as_str())
            .collect::<Vec<_>>(),
        ["pet"]
    );
}
