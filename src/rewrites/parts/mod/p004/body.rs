
#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    items.iter().map(|item| required_string(&item, label)).collect()
}

fn parse_ref_sequence_value(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    items.iter().map(|item| required_ref(&item, label)).collect()
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn local_ref(kind: &str, refs: &[String]) -> Result<String> {
    canonical_hash(&record("rewrite-derived-ref", vec![string(kind), refs_sequence(&sorted_unique_refs(refs))]))
}

fn merge_refs(left: &[String], right: &[String]) -> Vec<String> {
    left.iter().chain(right.iter()).cloned().collect()
}

fn sorted_unique_refs(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<OrderedSet<_>>().into_iter().collect()
}

fn sorted_unique_strings(values: &[String]) -> Vec<String> {
    values.iter().cloned().collect::<OrderedSet<_>>().into_iter().collect()
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} count {total} exceeds bound {maximum}")));
    }
    values.push_item(value);
    Ok(())
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type PathBuf = std::path::PathBuf;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    #[test]
    fn find_matches_schema_shapes_and_visibility_filter_hides_refs() {
        let root = temp_dir("rewrite-find");
        let schema_payload =
            parse_text(r#"<schema <shape "record" [<field "name" <shape "string">>]>>"#).expect("parse schema payload");
        let installed = install_fixture(&root, "schema", schema_payload, &[]);
        let visible =
            find(&root, &query(RewritePattern::SchemaShapeKind("record".to_string()))).expect("find schema shape");
        assert_eq!(visible.matches.len(), 1);
        assert_eq!(visible.matches[0].artifact_ref, installed.artifact_ref);
        let hidden = find(&root, &RewriteQueryInput {
            hidden_refs: vec![installed.artifact_ref.clone()],
            ..query(RewritePattern::SchemaShapeKind("record".to_string()))
        })
        .expect("hidden find");
        assert!(hidden.matches.is_empty());
    }

    #[test]
    fn preview_and_apply_create_new_artifact_without_mutating_old_payload() {
        let root = temp_dir("rewrite-apply");
        let payload = parse_text(r#"<doc "old" ["old" "keep"]>"#).expect("parse payload");
        let installed = install_fixture(&root, "doc", payload.clone(), &[]);
        let input = plan_input(RewritePattern::StringEquals("old".to_string()), "old", "new");
        let previewed = preview(&root, &input).expect("preview rewrite");
        assert_eq!(previewed.diffs.len(), 1);
        assert!(previewed.diffs[0].paths.as_slice().iter().any(|path| path.contains("doc")));
        let applied = apply(&root, &input).expect("apply rewrite");
        assert_eq!(applied.installed.len(), 1);
        let new_ref = &applied.installed[0].new_artifact_ref;
        assert_ne!(&installed.artifact_ref, new_ref);
        assert_eq!(crate::artifacts::read_payload(&root, &installed.artifact_ref).expect("old payload"), payload);
        let new_payload = crate::artifacts::read_payload(&root, new_ref).expect("new payload");
        assert!(to_text(&new_payload).expect("render new").contains("new"));
        assert!(!to_text(&new_payload).expect("render new again").contains("old"));
    }

    #[test]
    fn apply_receipt_builds_upgrade_plan_hook() {
        let root = temp_dir("rewrite-upgrade-hook");
        let payload = parse_text(r#"<doc "old">"#).expect("parse payload");
        install_fixture(&root, "doc", payload, &[]);
        let input = plan_input(RewritePattern::StringEquals("old".to_string()), "old", "new");
        let applied = apply(&root, &input).expect("apply rewrite");
        let plan = upgrade_plan_from_apply(
            &applied,
            "rewrite-session",
            &test_ref("initiator"),
            &[test_ref("upgrade-capability")],
            &[test_ref("upgrade-policy")],
        )
        .expect("upgrade plan");
        let parsed = crate::upgrades::parse_upgrade_plan(&plan).expect("parse upgrade plan");
        assert_eq!(parsed.tasks[0].kind, "install-artifact");
        assert!(parsed.checks.as_slice().iter().any(|check| check == "no-ucm-clone"));
    }

    #[test]
    fn unauthorized_or_empty_policy_is_denied_before_apply() {
        let root = temp_dir("rewrite-deny");
        let payload = parse_text(r#"<doc "old">"#).expect("parse payload");
        install_fixture(&root, "doc", payload, &[]);
        let mut input = plan_input(RewritePattern::StringEquals("old".to_string()), "old", "new");
        input.capability_refs.clear();
        let error = preview(&root, &input).expect_err("missing capability denied");
        assert!(error.to_string().contains("capability"), "{error}");
    }

    #[hegel::test(test_cases = 12)]
    fn hegel_preview_apply_consistency_and_path_stability(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("rewrite-hegel");
        let needle = format!("old-{salt}");
        let replacement = format!("new-{salt}");
        let payload = record("doc", vec![string(&needle), sequence(vec![string(&needle), string("stable")])]);
        let installed = install_fixture(&root, "doc", payload.clone(), &[]);
        let input = plan_input(RewritePattern::StringEquals(needle.clone()), &needle, &replacement);
        let first = preview(&root, &input).expect("first preview");
        let second = preview(&root, &input).expect("second preview");
        assert_eq!(first.diffs[0].paths, second.diffs[0].paths);
        assert_eq!(first.diffs[0].new_payload_ref, second.diffs[0].new_payload_ref);
        let applied = apply(&root, &input).expect("apply");
        assert_ne!(installed.artifact_ref, applied.installed[0].new_artifact_ref);
        assert_eq!(crate::artifacts::read_payload(&root, &installed.artifact_ref).expect("old payload"), payload);
    }

    fn query(pattern: RewritePattern) -> RewriteQueryInput {
        RewriteQueryInput {
            artifact_kinds: Vec::new(),
            root_refs: Vec::new(),
            include_dependencies: true,
            pattern,
            policy_refs: vec![test_ref("query-policy")],
            capability_refs: vec![test_ref("query-capability")],
            hidden_refs: Vec::new(),
        }
    }

    fn plan_input(pattern: RewritePattern, from: &str, to: &str) -> RewritePlanInput {
        RewritePlanInput {
            query: query(pattern),
            replacement: RewriteReplacement::StringValue {
                from: from.to_string(),
                to: to.to_string(),
            },
            planner_ref: test_ref("planner"),
            policy_refs: vec![test_ref("plan-policy")],
            capability_refs: vec![test_ref("plan-capability")],
            transcript_refs: vec![test_ref("transcript")],
            schema_migration_recipe_refs: vec![test_ref("migration-recipe")],
        }
    }

    fn install_fixture(
        root: &Path,
        kind: &str,
        payload: IoValue,
        dependency_refs: &[String],
    ) -> crate::artifacts::ArtifactInstall {
        crate::artifacts::install_artifact(root, &crate::artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: vec![test_ref("schema")],
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("artifact-policy")],
            evidence_refs: vec![test_ref("artifact-evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("install-capability")],
        })
        .expect("install fixture")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("rewrite-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
