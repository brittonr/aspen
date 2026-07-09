#[test]
fn cli_transcript_commands_work() {
    let dir = temp_dir("transcript-cli");
    let fixture = parse_transcript_fixture(&dir);
    run_transcript_fixture(&dir, &fixture);
    show_transcript_fixture(fixture);
}

struct TranscriptFixture {
    transcript_out: PathBuf,
    run_receipt: PathBuf,
}

fn parse_transcript_fixture(dir: &Path) -> TranscriptFixture {
    let markdown = dir.join("example.md");
    let transcript_out = dir.join("transcript.preserves");
    write_file(
        &markdown,
        "```preserves:hide\n<value \"cli\">\n```\n```expect\n<expect-output <value \"cli\">>\n```\n",
    )
    .expect("write transcript markdown");
    crate::cli_transcript::run(crate::cli_transcript::Command::Parse {
        markdown,
        out: transcript_out.clone(),
        dependency_refs: Vec::new(),
        dependency_closure_hash: None,
        artifact_refs: Vec::new(),
        schema_refs: Vec::new(),
        handler_profile_ref: None,
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        resource_refs: Vec::new(),
        effect_manifest_refs: Vec::new(),
        revocation_refs: Vec::new(),
        seed_ref: None,
        logical_time: None,
        expected_refs: Vec::new(),
        resolution_refs: Vec::new(),
    })
    .expect("transcript parse");
    TranscriptFixture {
        transcript_out,
        run_receipt: dir.join("transcript-run-receipt.preserves"),
    }
}

fn run_transcript_fixture(dir: &Path, fixture: &TranscriptFixture) {
    let rendered = dir.join("rendered.md");
    crate::cli_transcript::run(crate::cli_transcript::Command::Run {
        transcript: fixture.transcript_out.clone(),
        cache: Some(dir.join("transcript-cache")),
        state: "fresh".to_string(),
        save_root: None,
        out: Some(rendered.clone()),
        receipt_out: Some(fixture.run_receipt.clone()),
        failure_out: Some(dir.join("transcript.failure.preserves")),
    })
    .expect("transcript run");
    assert!(fs::read_to_string(&rendered).expect("read rendered").contains("output hidden"));
}

fn show_transcript_fixture(fixture: TranscriptFixture) {
    crate::cli_transcript::run(crate::cli_transcript::Command::Show {
        transcript: fixture.transcript_out.clone(),
    })
    .expect("transcript show");
    crate::cli_transcript::run(crate::cli_transcript::Command::Render {
        transcript: fixture.transcript_out,
        receipt: Some(fixture.run_receipt),
        out: temp_dir("transcript-render").join("rendered-again.md"),
    })
    .expect("transcript render");
}

#[test]
fn cli_eval_cache_commands_work() {
    let dir = temp_dir("cache-cli");
    let fixture = put_cache_value(&dir);
    fetch_cache_value(&dir, &fixture);
    inspect_cache_entries(&fixture);
    invalidate_cache_entry(&dir, fixture);
}

struct CacheFixture {
    cache: PathBuf,
    key: molten::eval_cache::Key,
    value_out: PathBuf,
    dependency_ref: String,
    policy_ref: String,
}

fn put_cache_value(dir: &Path) -> CacheFixture {
    let cache = dir.join("eval-cache");
    let input = dir.join("input.preserves");
    let output = dir.join("output.preserves");
    let key_out = dir.join("key.preserves");
    let value_out = dir.join("value.preserves");
    let dependency_ref = test_ref("cache-cli-dependency");
    let policy_ref = test_ref("cache-cli-policy");
    write_file(&input, "<schema-shape <record \"x\">>").expect("write cache input");
    write_file(&output, "<fingerprint \"ok\">").expect("write cache output");
    crate::cli_cache::run(crate::cli_cache::Command::Put(crate::cli_cache::command::Put {
        input,
        cache: cache.clone(),
        output: Some(output),
        operation: "schema-fingerprint".to_string(),
        version: "v1".to_string(),
        dependencies: vec![dependency_ref.clone()],
        dependency_closure_hash: None,
        handler_profile_ref: None,
        policy_refs: vec![policy_ref.clone()],
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: None,
        tool_version: "cli-test".to_string(),
        assumption_refs: Vec::new(),
        tier: molten::eval_cache::TIER_PURE.to_string(),
        status: molten::eval_cache::STATUS_PASS.to_string(),
        evidence_refs: Vec::new(),
        diagnostics: Vec::new(),
        key_out: Some(key_out.clone()),
        value_out: Some(value_out.clone()),
        receipt_out: Some(dir.join("put-receipt.preserves")),
    }))
    .expect("cache put");
    let key = molten::eval_cache::parse_key(&read_preserves_file(&key_out).expect("read key"))
        .expect("parse cache key");
    CacheFixture {
        cache,
        key,
        value_out,
        dependency_ref,
        policy_ref,
    }
}

fn fetch_cache_value(dir: &Path, fixture: &CacheFixture) {
    let hit_out = dir.join("hit.preserves");
    crate::cli_cache::run(crate::cli_cache::Command::Get(crate::cli_cache::command::Get {
        key_ref: fixture.key.key_ref.clone(),
        cache: fixture.cache.clone(),
        current_policy_refs: vec![fixture.policy_ref.clone()],
        current_capability_refs: Vec::new(),
        current_revocation_refs: Vec::new(),
        semantic_enabled: true,
        out: Some(hit_out.clone()),
        receipt_out: Some(dir.join("hit-receipt.preserves")),
    }))
    .expect("cache get");
    assert_eq!(fs::read_to_string(&hit_out).expect("read hit"), "<fingerprint \"ok\">");
}

fn inspect_cache_entries(fixture: &CacheFixture) {
    crate::cli_cache::run(crate::cli_cache::Command::Status(crate::cli_cache::command::Status {
        cache: fixture.cache.clone(),
    }))
    .expect("cache status");
    crate::cli_cache::run(crate::cli_cache::Command::List(crate::cli_cache::command::List {
        cache: fixture.cache.clone(),
        operation: Some("schema-fingerprint".to_string()),
        tier: Some(molten::eval_cache::TIER_PURE.to_string()),
        status: Some(molten::eval_cache::STATUS_PASS.to_string()),
        dependency_ref: Some(fixture.dependency_ref.clone()),
        policy_ref: Some(fixture.policy_ref.clone()),
        capability_ref: None,
        revocation_ref: None,
        evidence_ref: None,
    }))
    .expect("cache list");
    show_cache_key_and_value(fixture);
}

fn show_cache_key_and_value(fixture: &CacheFixture) {
    crate::cli_cache::run(crate::cli_cache::Command::Show(crate::cli_cache::command::Show {
        reference: fixture.key.key_ref.clone(),
        cache: fixture.cache.clone(),
    }))
    .expect("cache show key");
    crate::cli_cache::run(crate::cli_cache::Command::Show(crate::cli_cache::command::Show {
        reference: molten::eval_cache::parse_value(&read_preserves_file(&fixture.value_out).expect("read value"))
            .expect("parse cache value")
            .value_ref,
        cache: fixture.cache.clone(),
    }))
    .expect("cache show value");
}

fn invalidate_cache_entry(dir: &Path, fixture: CacheFixture) {
    let retention_object = RetentionCliObject {
        root: &fixture.cache,
        label: "cache-invalidate",
        object_ref: &fixture.key.key_ref,
        object_kind: "eval-cache-key",
        retention_class: molten::retention::CLASS_EPHEMERAL_CACHE,
        action: molten::retention::ACTION_TOMBSTONE,
    };
    let retention = retention_cli_args_for_object(retention_object);
    let apply_refs = vec![retention_apply_ref(retention_object, "eval-cache-invalidate", &retention)];
    let invalidate_receipt = dir.join("invalidate-receipt.preserves");
    crate::cli_cache::run(crate::cli_cache::Command::Invalidate(crate::cli_cache::command::Invalidate {
        cache: fixture.cache.clone(),
        key_ref: None,
        dependency_ref: Some(fixture.dependency_ref.clone()),
        policy_ref: None,
        capability_ref: None,
        revocation_ref: None,
        operation: None,
        reason: "cli-test".to_string(),
        apply_refs,
        retention,
        receipt_out: Some(invalidate_receipt.clone()),
    }))
    .expect("cache invalidate");
    let invalidate_text = fs::read_to_string(&invalidate_receipt).expect("read invalidate receipt");
    assert!(invalidate_text.contains("retention-execution"));
    assert_cache_miss(fixture);
}

fn assert_cache_miss(fixture: CacheFixture) {
    let error = crate::cli_cache::run(crate::cli_cache::Command::Get(crate::cli_cache::command::Get {
        key_ref: fixture.key.key_ref,
        cache: fixture.cache,
        current_policy_refs: Vec::new(),
        current_capability_refs: Vec::new(),
        current_revocation_refs: Vec::new(),
        semantic_enabled: true,
        out: None,
        receipt_out: None,
    }))
    .expect_err("invalidated key should miss");
    assert!(error.to_string().contains("tombstoned"), "{error}");
}
