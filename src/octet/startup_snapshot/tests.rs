use super::*;
use molten_core::node_startup::{BUNDLE_SCHEMA, Cohort, Descriptor, Member, POLICY_SCHEMA, ROLES, TrustedCohort};

// In-memory test data only. No fixture exporter, startup token, or runtime activation path.
pub(crate) fn fixture() -> (TrustedCohort, Descriptor, Vec<Vec<u8>>) {
    let manifest = b"[workspace.metadata.octet]\ndefault_scope = [\"-p\", \"molten\", \"-p\", \"molten-node-host\"]\ncargo_check_args = [\"--all-targets\"]\n";
    let dylint = b"[octet]\ndisabled_lints = []\n";
    let expected = explicit_metadata(std::str::from_utf8(manifest).unwrap(), dylint, DEFAULT_GATE_COMMAND).unwrap();
    let mut members = vec![
        manifest.to_vec(),
        dylint.to_vec(),
        b"locked".to_vec(),
        b"{}".to_vec(),
        b"[toolchain]\nchannel = \"nightly-2026-05-26\"\n".to_vec(),
    ];
    let paths = [
        "src/job/dag.rs",
        "src/main.rs",
        "src/node/daemon.rs",
        "src/node/runtime.rs",
        "src/octet/gate.rs",
        "src/upgrades/mod.rs",
        "src/node/content.rs",
        "src/node/startup_evidence.rs",
        "src/octet/startup_snapshot.rs",
        "crates/molten-core/src/node_startup.rs",
        "crates/molten-core/src/content_store_adapter/node_service.rs",
        "src/node/parts/daemon/p018/body.rs",
        "src/node/parts/daemon/p019/body.rs",
    ];
    let mut sources: Vec<SourceFile> = members
        .iter()
        .zip(&ROLES)
        .map(|(bytes, role)| SourceFile {
            name: role.filename().into(),
            blake3: blake3::hash(bytes).to_hex().to_string(),
            bytes: bytes.len() as u64,
        })
        .collect();
    for path in paths {
        sources.push(SourceFile {
            name: path.into(),
            blake3: "c".repeat(64),
            bytes: 1,
        });
    }
    sources.sort_by(|a, b| a.name.cmp(&b.name));
    members.push(serde_json::to_vec(&sources).unwrap());
    members.push(DEFAULT_GATE_COMMAND.as_bytes().to_vec());
    members.push(serde_json::to_vec(&serde_json::json!({"status":"clean", "exit_code":0,
        "metadata":{"tool_name":"cargo-octet","tool_version":"0.1.0","rustc_version":"rustc model data",
        "toolchain":"nightly-2026-03-21-x86_64-unknown-linux-gnu", "config_hash":expected.config_hash,"profile_hash":expected.profile_hash},
        "total_findings":0,"warning_findings":0,"error_findings":0,"autofixable_findings":0})).unwrap());
    members.push(b"Findings: 0\n\nBy lint:\n\nIndex:\n".to_vec());
    members.push(serde_json::to_vec(&serde_json::json!({"schema":"octet.function-object-corpus-receipt.v1", "schema_version":1,
        "object_count":13, "source_paths":paths, "object_set_hash":format!("b3:{}", "a".repeat(64)), "pure_cache_blocked_count":0})).unwrap());
    let cohort = Cohort {
        source_revision: "a".repeat(40),
        source_inventory_blake3: blake3::hash(&members[5]).to_hex().to_string(),
        executable_blake3: "b".repeat(64),
        build_rustc_blake3: "c".repeat(64),
        build_toolchain: "nightly-2026-05-26".into(),
        octet_revision: "c9b06bcf565c51d4a77d210e61b69ae51db9df25".into(),
        octet_cli_blake3: "e".repeat(64),
        octet_driver_blake3: "f".repeat(64),
        octet_lints_blake3: "1".repeat(64),
        octet_rustc_blake3: "2".repeat(64),
        octet_toolchain: "nightly-2026-03-21-x86_64-unknown-linux-gnu".into(),
    };
    let mut descriptor = Descriptor {
        schema: BUNDLE_SCHEMA.into(),
        cohort: cohort.clone(),
        members: vec![],
    };
    let mut policy = TrustedCohort {
        schema: POLICY_SCHEMA.into(),
        descriptor_blake3: "0".repeat(64),
        cohort,
    };
    repin_test_data(&mut policy, &mut descriptor, &members);
    (policy, descriptor, members)
}

// Test-only changes re-pin identities to exercise semantic rejection, rather than just hash failure.
pub(crate) fn repin_test_data(policy: &mut TrustedCohort, descriptor: &mut Descriptor, members: &[Vec<u8>]) {
    descriptor.members = members
        .iter()
        .zip(ROLES)
        .map(|(bytes, role)| Member {
            role,
            blake3: blake3::hash(bytes).to_hex().to_string(),
            bytes: bytes.len() as u64,
        })
        .collect();
    descriptor.cohort.source_inventory_blake3 = blake3::hash(&members[5]).to_hex().to_string();
    policy.cohort = descriptor.cohort.clone();
    policy.descriptor_blake3 = blake3::hash(&serde_json::to_vec(descriptor).unwrap()).to_hex().to_string();
}

fn evaluate_data(policy: &TrustedCohort, descriptor: Descriptor, members: &[Vec<u8>]) -> Result<OctetGateEvaluation> {
    let plan = EvidencePlan::admit(policy, descriptor, &policy.cohort.executable_blake3).unwrap();
    evaluate(Snapshot { plan: &plan, members })
}

// r[verify molten.startup_evidence.strict]
#[test]
fn explicit_snapshot_is_deterministic_and_strict() {
    let (policy, descriptor, members) = fixture();
    let result = evaluate_data(&policy, descriptor.clone(), &members).unwrap();
    assert_eq!(result.decision, "pass");
    assert_eq!(result.receipt_ref, evaluate_data(&policy, descriptor, &members).unwrap().receipt_ref);
}

#[test]
fn metadata_formula_matches_existing_owner() {
    let actual = explicit_metadata(
        include_str!("../../../Cargo.toml"),
        include_bytes!("../../../dylint.toml"),
        DEFAULT_GATE_COMMAND,
    )
    .unwrap();
    let expected = expected_metadata_for_command(DEFAULT_GATE_COMMAND).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn stale_context_is_not_repaired_by_receipt_or_cwd() {
    let (mut p, mut d, mut m) = fixture();
    let mut status: serde_json::Value = serde_json::from_slice(&m[7]).unwrap();
    status["metadata"]["config_hash"] = serde_json::json!(format!("b3:{}", "f".repeat(64)));
    m[7] = serde_json::to_vec(&status).unwrap();
    repin_test_data(&mut p, &mut d, &m);
    assert_eq!(evaluate_data(&p, d, &m).unwrap().decision, "deny");
}

#[test]
fn synthetic_toolchain_findings_and_inconsistent_summary_deny() {
    for field in [
        "total_findings",
        "warning_findings",
        "error_findings",
        "autofixable_findings",
        "exit_code",
    ] {
        let (mut p, mut d, mut m) = fixture();
        let mut status: serde_json::Value = serde_json::from_slice(&m[7]).unwrap();
        status[field] = serde_json::json!(1);
        m[7] = serde_json::to_vec(&status).unwrap();
        repin_test_data(&mut p, &mut d, &m);
        assert!(evaluate_data(&p, d, &m).unwrap_err().to_string().contains("not-strict-clean"));
    }
    let (mut p, mut d, mut m) = fixture();
    let mut status: serde_json::Value = serde_json::from_slice(&m[7]).unwrap();
    status["metadata"]["toolchain"] = serde_json::json!("nightly-test-toolchain");
    m[7] = serde_json::to_vec(&status).unwrap();
    repin_test_data(&mut p, &mut d, &m);
    assert!(evaluate_data(&p, d, &m).is_err());
    for summary in [
        "Findings: 01\n",
        "Findings: 0\nFindings: 0\n",
        "Findings: 0\nBy lint:\nnot_critical 1\n",
    ] {
        let (mut p, mut d, mut m) = fixture();
        m[8] = summary.as_bytes().to_vec();
        repin_test_data(&mut p, &mut d, &m);
        assert!(evaluate_data(&p, d, &m).unwrap_err().to_string().contains("summary-count"));
    }
}

#[test]
fn replay_command_is_not_source_coverage() {
    let (mut p, mut d, mut m) = fixture();
    let mut corpus: serde_json::Value = serde_json::from_slice(&m[9]).unwrap();
    corpus["source_paths"] = serde_json::json!(["src/main.rs"]);
    corpus["replay"] = serde_json::json!({"command": SOURCE_GATE_SOURCE_SCOPE_PATHS.join(" ")});
    m[9] = serde_json::to_vec(&corpus).unwrap();
    repin_test_data(&mut p, &mut d, &m);
    assert!(evaluate_data(&p, d, &m).unwrap_err().to_string().contains("source-coverage"));
}

#[test]
fn narrowed_scope_baselines_and_suppression_are_rejected() {
    let manifest = include_str!("../../../Cargo.toml");
    for config in [
        "[octet]\ndisabled_lints = [\"no_panic\"]\n",
        "[octet]\ndisabled_lints = []\nwarning_budget = 1\n",
    ] {
        assert!(explicit_metadata(manifest, config.as_bytes(), DEFAULT_GATE_COMMAND).is_err());
    }
    let (mut p, mut d, mut m) = fixture();
    m[6] = b"cargo octet check -p molten --artifact-dir target/octet --baseline baseline.json".to_vec();
    repin_test_data(&mut p, &mut d, &m);
    assert!(evaluate_data(&p, d, &m).unwrap_err().to_string().contains("command-profile"));
}

#[test]
fn missing_source_and_changed_member_deny() {
    let (mut p, mut d, mut m) = fixture();
    m[5] = b"[]".to_vec();
    repin_test_data(&mut p, &mut d, &m);
    assert!(evaluate_data(&p, d, &m).unwrap_err().to_string().contains("source-context"));
    let (p, d, mut m) = fixture();
    m[0].push(b'!');
    assert!(evaluate_data(&p, d, &m).unwrap_err().to_string().contains("member-identity"));
}
