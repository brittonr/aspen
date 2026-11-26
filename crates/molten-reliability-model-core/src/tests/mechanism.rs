//! Bounded profile text, collection, and closed-vocabulary regressions.

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Bounded {
    #[serde(deserialize_with = "crate::mechanism::bounded::text")]
    name: alloc::string::String,
    #[serde(deserialize_with = "crate::mechanism::bounded::texts")]
    tags: alloc::vec::Vec<alloc::string::String>,
    #[serde(deserialize_with = "crate::mechanism::bounded::optional_text")]
    note: Option<alloc::string::String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Buffered,
    MachineLoss,
}

const fn level_name(level: Level) -> &'static str {
    match level {
        Level::Buffered => "buffered",
        Level::MachineLoss => "machine-loss",
    }
}

#[test]
fn bounded_text_and_collections_accept_limits_and_reject_overflow() {
    let at_limit = "x".repeat(crate::mechanism::bounded::MAX_TEXT_BYTES);
    let decoded: Bounded = serde_json::from_value(serde_json::json!({
        "name": at_limit, "tags": ["a", "b"], "note": null
    }))
    .expect("bounded profile text");
    assert_eq!(decoded.name.len(), crate::mechanism::bounded::MAX_TEXT_BYTES);
    assert_eq!(decoded.tags, alloc::vec![alloc::string::String::from("a"), alloc::string::String::from("b")]);
    assert_eq!(decoded.note, None);
    let over = "x".repeat(crate::mechanism::bounded::MAX_TEXT_BYTES + 1);
    assert!(serde_json::from_value::<Bounded>(serde_json::json!({"name": over, "tags": [], "note": null})).is_err());
    assert!(serde_json::from_value::<Bounded>(serde_json::json!({"name": "n", "tags": [over], "note": null})).is_err());
    assert!(serde_json::from_value::<Bounded>(serde_json::json!({"name": "n", "tags": [], "note": over})).is_err());
    let many = alloc::vec!["t"; crate::mechanism::bounded::MAX_ITEMS + 1];
    assert!(serde_json::from_value::<Bounded>(serde_json::json!({"name": "n", "tags": many, "note": "ok"})).is_err());
    assert!(
        serde_json::from_value::<Bounded>(
            serde_json::json!({"name": "n", "tags": [], "note": null, "authority": true})
        )
        .is_err()
    );
}

#[test]
fn closed_vocabulary_has_no_default_member() {
    let levels = [Level::Buffered, Level::MachineLoss];
    assert_eq!(crate::mechanism::choice("machine-loss", &levels, level_name), Ok(Level::MachineLoss));
    assert_eq!(
        crate::mechanism::choice("process-loss", &levels, level_name),
        Err(crate::mechanism::VocabularyIssue {
            value: "process-loss".into()
        })
    );
    assert_eq!(
        crate::mechanism::choices(alloc::vec!["buffered".into(), "machine-loss".into()], &levels, level_name),
        Ok(alloc::vec![Level::Buffered, Level::MachineLoss])
    );
    let denied = crate::mechanism::choices(alloc::vec!["buffered".into(), "trusted".into()], &levels, level_name)
        .expect_err("unknown member");
    assert_eq!(std::string::ToString::to_string(&denied), "unsupported profile value: trusted");
}
