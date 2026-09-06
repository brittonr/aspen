//! Consumer compatibility regression: migration is additive, not a policy reset.
use serde_json::{Value, json};

fn baseline() -> Value {
    serde_json::from_str(include_str!("../cairn-policy/fixtures/migration-baseline.json")).unwrap()
}
fn candidate() -> Value {
    serde_json::from_str(include_str!("../cairn-policy/generated/cairn-policy.json")).unwrap()
}

fn first_difference(old: &Value, new: &Value, path: &str) -> String {
    if let (Some(a), Some(b)) = (old.as_object(), new.as_object()) {
        for key in a.keys().chain(b.keys()) {
            if a.get(key) != b.get(key) {
                return first_difference(&old[key], &new[key], &format!("{path}.{key}"));
            }
        }
    }
    if let (Some(a), Some(b)) = (old.as_array(), new.as_array()) {
        if a.len() != b.len() {
            return format!("{path} length {} -> {}", a.len(), b.len());
        }
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            if x != y {
                return first_difference(x, y, &format!("{path}[{i}]"));
            }
        }
    }
    format!(
        "{path}: {} -> {}",
        old.to_string().chars().take(128).collect::<String>(),
        new.to_string().chars().take(128).collect::<String>()
    )
}

fn preserved(old: &Value, new: &Value) -> Result<(), String> {
    for (key, value) in old.as_object().ok_or("baseline object")? {
        if ![
            "schemas",
            "traceability_policy",
            "sync_archive_policy",
            "task_marker_policy",
            "policy_schema_compatibility",
            "receipt_schemas",
        ]
        .contains(&key.as_str())
            && new.get(key) != Some(value)
        {
            return Err(format!("protected policy changed: {}", first_difference(value, &new[key], key)));
        }
    }
    let prior_receipts = old["receipt_schemas"].as_array().ok_or("prior receipts")?;
    let next_receipts = new["receipt_schemas"].as_array().ok_or("next receipts")?;
    if next_receipts.len() != prior_receipts.len() + 5 || next_receipts[..prior_receipts.len()] != prior_receipts[..] {
        return Err("prior receipt schemas changed".into());
    }
    for (row, (command, hash)) in next_receipts[prior_receipts.len()..].iter().zip([
        ("function-object", "object_hash"),
        ("name-pointer", "pointer_hash"),
        ("export-bundle", "bundle_hash"),
        ("downstream-pilot", "pilot_hash"),
        ("circuit-primitive", "graph_hash"),
    ]) {
        if row["command"] != format!("evidence {command} validate")
            || row["hash_fields"] != json!([hash])
            || row["issue_keys"] != json!(["actual", "code", "expected", "field_path"])
        {
            return Err("retained receipt contract mapping changed".into());
        }
    }
    let mut expected = old["sync_archive_policy"].clone();
    expected["default_archive_dir"] = json!(".cairn/archive");
    if new["sync_archive_policy"] != expected {
        return Err("archive policy".into());
    }
    let mut expected = old["traceability_policy"].clone();
    for profile in expected["profiles"].as_array_mut().ok_or("profiles")? {
        profile["assurance_level"] = json!("declared");
        profile["anchor_kind"] = json!("exact_marker_bytes");
    }
    if new["traceability_policy"] != expected {
        return Err("traceability policy".into());
    }
    let mut markers = new["task_marker_policy"].clone();
    let all = markers["markers"].as_array_mut().ok_or("markers")?;
    if all.len() != 5
        || all[3] != json!({"id":"task","token":"[task:<id>]","kind":"task-id-prefix"})
        || all[4] != json!({"id":"after","token":"[after:<task-id>]","kind":"task-predecessor-prefix"})
    {
        return Err("marker additions".into());
    }
    all.truncate(3);
    if markers != old["task_marker_policy"] {
        return Err("marker policy".into());
    }
    let spec = new["schemas"]
        .as_array()
        .ok_or("schemas")?
        .iter()
        .find(|s| s["id"] == "spec-driven")
        .ok_or("spec-driven")?;
    let artifacts = spec["artifacts"].as_array().ok_or("artifacts")?;
    if artifacts.len() != 4 {
        return Err("artifact count".into());
    }
    for (prior, next) in old["schemas"][0]["artifacts"].as_array().ok_or("old artifacts")?.iter().zip(artifacts) {
        for field in ["id", "generates", "requires"] {
            if prior[field] != next[field] {
                return Err(format!("artifact {field}"));
            }
        }
        if next["skipped"] != false || next["skip_satisfies_dependencies"] != false {
            return Err("skipped artifact".into());
        }
    }
    if new["workflow_profile_policy"]["allowed_profiles"] != old["workflow_profile_policy"]["allowed_profiles"]
        || new["workflow_profile_policy"]["default_profile"] != "spec-driven"
        || new["workflow_profile_policy"]["forbidden_selection_sources"]
            != old["workflow_profile_policy"]["forbidden_selection_sources"]
        || new["workflow_profile_policy"]["required_profiles_by_effect"]
            != old["workflow_profile_policy"]["required_profiles_by_effect"]
        || spec["spec_effect"] != "delta"
        || spec["task_link_class"] != "requirement"
    {
        return Err("workflow selection widened".into());
    }
    if new["authenticated_evidence_policy"]["trusted_producers"] != json!([]) {
        return Err("new trust".into());
    }
    Ok(())
}

#[test]
fn existing_gate_authority_and_artifact_graph_are_preserved() {
    preserved(&baseline(), &candidate()).unwrap();
}
#[test]
fn weaker_gate_missing_requirement_and_added_profile_fail() {
    let old = baseline();
    for pointer in [
        "/gate_policy/substance/minimum_requirement_blocks",
        "/mutation_policy/archive/require_passing_gate",
        "/receipt_hash_policy/required",
        "/schemas/0/artifacts/3/requires",
        "/workflow_profile_policy/allowed_profiles",
    ] {
        let mut changed = candidate();
        *changed.pointer_mut(pointer).unwrap() = Value::Null;
        assert!(preserved(&old, &changed).is_err(), "{pointer}");
    }
}

struct TestRoot(std::path::PathBuf);
impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "molten-policy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove owned test root");
    }
}

#[test]
#[ignore = "requires an explicitly selected compatible Cairn binary; no installation or fallback"]
fn runtime_admits_policy_and_rejects_bad_graph_and_profile() {
    let cairn = std::env::var_os("MOLTEN_POLICY_CAIRN").expect("MOLTEN_POLICY_CAIRN");
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = TestRoot::new();
    let good = candidate();
    let cases = [
        ("good", good.clone(), None),
        (
            "cycle",
            {
                let mut p = good.clone();
                p["schemas"][0]["artifacts"][0]["requires"] = json!(["tasks"]);
                p
            },
            Some("cycle"),
        ),
        (
            "escape",
            {
                let mut p = good.clone();
                p["schemas"][0]["artifacts"][0]["output_path"] = json!("../outside");
                p
            },
            Some("unsafe_path"),
        ),
        (
            "missing",
            {
                let mut p = good.clone();
                p["schemas"][0]["artifacts"][0].as_object_mut().unwrap().remove("output_path");
                p
            },
            Some("output_path"),
        ),
    ];
    for (name, value, error) in cases {
        let file = temp.path().join(format!("{name}.json"));
        std::fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        let result = std::process::Command::new(&cairn)
            .current_dir(repo)
            .args(["workflow", "status", "node-content-service", "--root", ".", "--policy"])
            .arg(file)
            .output()
            .unwrap();
        match error {
            None => assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr)),
            Some(reason) => {
                assert!(!result.status.success(), "{name}");
                assert!(
                    String::from_utf8_lossy(&result.stderr).contains(reason),
                    "{name}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }
    }
    // An undeclared profile cannot create lifecycle artifacts.
    let result = std::process::Command::new(&cairn)
        .current_dir(repo)
        .args([
            "change",
            "create",
            "not-authorized",
            "--profile",
            "not-declared",
            "--root",
        ])
        .arg(temp.path())
        .args(["--policy"])
        .arg(repo.join("cairn-policy/generated/cairn-policy.json"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!temp.path().join(".cairn/changes/not-authorized").exists());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("unknown"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
