//! Deterministic normalizer controls only. These are not a maintained DB scan.
// r[verify molten.dependency_audit.controls]
// r[verify molten.dependency_audit.resolved]
use super::*;

fn require(condition: bool, case: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(format!("audit control failed: {case}")) }
}
fn has(eval: &Evaluation, kind: &str) -> bool { eval.findings.iter().any(|f| f.kind == kind) }

pub fn run() -> Result<(), String> {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/metadata.json")).map_err(|e| e.to_string())?;
    let metadata: Metadata = serde_json::from_value(fixture.clone()).map_err(|e| e.to_string())?;
    let mut policy = Policy { schema: "molten.dependency-audit-policy.v2".into(),
        allowed_licenses: vec!["MIT".into(), "Apache-2.0".into(), "AGPL-3.0-or-later".into()],
        allowed_exceptions: vec!["LLVM-exception".into()], exceptions: vec![] };
    let release = json!({"schema":"molten.release-dependency-profile.v2","dependencies":[]});
    let binding = json!({"schema":"molten.dependency-audit-binding.v2","profile":"molten-default","features":[],
        "default_features":true,"target":"x86_64-unknown-linux-gnu","scanner_version":SCANNER,
        "database_revision":"1111111111111111111111111111111111111111","database_date":"2026-11-13"});
    let scanner = json!({"database":{"advisory-count":1,"last-commit":"1111111111111111111111111111111111111111",
        "last-updated":"2026-11-13T00:00:00Z"},"settings":{"ignore":[],"severity":null,"target_arch":[],"target_os":[],
        "informational_warnings":["unmaintained","unsound","notice"]},"vulnerabilities":{"count":0,"found":false,"list":[]},"warnings":{}});
    let as_of = "2026-11-13";
    let baseline = evaluate(&metadata,&policy,&release,&scanner,&binding,as_of);
    require(!baseline.denied() && baseline.packages.len() == 2, "default transitive closure excludes dev-only package")?;
    let mut offline = scanner.clone();
    offline["database"]["last-commit"] = Value::Null;
    offline["database"]["last-updated"] = Value::Null;
    require(!evaluate(&metadata,&policy,&release,&offline,&binding,as_of).denied(),
        "offline scanner metadata is separately bound to the retained verified Git snapshot")?;
    let commit = b"tree 1111111111111111111111111111111111111111\ncommitter fixture <fixture@example.invalid> 1794528000 +0000\n\nfixture\n";
    require(commit_day(commit) == day("2026-11-13") && commit_day(b"missing committer").is_none(),
        "retained commit date derives from the actual Git committer timestamp")?;
    let mut missing_metadata = offline.clone();
    missing_metadata["database"].as_object_mut().ok_or("database fixture object")?.remove("last-commit");
    require(has(&evaluate(&metadata,&policy,&release,&missing_metadata,&binding,as_of),"stale-database"),
        "missing scanner metadata field is not an explicitly nullable offline observation")?;
    let mut incomplete = fixture.clone(); incomplete["resolve"]["nodes"] = json!([fixture["resolve"]["nodes"][0].clone()]);
    let incomplete: Metadata = serde_json::from_value(incomplete).map_err(|e| e.to_string())?;
    require(has(&evaluate(&incomplete,&policy,&release,&scanner,&binding,as_of),"resolved-closure"),
        "missing selected resolve-node metadata remains unavailable")?;
    require(license_allowed("MIT OR GPL-3.0-only", &policy)?
        && !license_allowed("MIT AND GPL-3.0-only", &policy)?
        && license_allowed("(MIT OR GPL-3.0-only) AND Apache-2.0", &policy)?
        && license_allowed("Apache-2.0 WITH LLVM-exception", &policy)?
        && license_allowed("MIT OR", &policy).is_err(), "SPDX AND/OR/WITH/parentheses and invalid syntax")?;
    let mut affected = scanner.clone();
    affected["vulnerabilities"] = json!({"count":1,"found":true,"list":[{"advisory":{"id":"RUSTSEC-2020-0071",
        "title":"Affected fixture observation"},"package":{"name":"time","version":"0.1.43","source":CRATES_IO},
        "versions":{"patched":[">=0.2.23"],"unaffected":[]}}]});
    let found = evaluate(&metadata,&policy,&release,&affected,&binding,as_of);
    require(found.denied() && found.findings.iter().any(|f| f.kind == "advisory" && f.package == "time"
        && f.dependency_path == ["root", "time"] && f.evidence["versions"]["patched"] == json!([">=0.2.23"])),
        "affected transitive package retains path and actionable scanner evidence")?;
    for source in ["git+https://github.com/OnixResearch/unreviewed#1111111111111111111111111111111111111111",
        "git+rad://unknown#1111111111111111111111111111111111111111"] {
        let mut unknown = fixture.clone(); unknown["packages"][1]["source"] = json!(source);
        let unknown: Metadata = serde_json::from_value(unknown).map_err(|e| e.to_string())?;
        let found = evaluate(&unknown,&policy,&release,&affected,&binding,as_of);
        require(found.denied() && has(&found,"source-policy") && !has(&found,"advisory"),
            "unknown Git/Radicle denies; registry advisory is not falsely assigned to different source")?;
    }
    let mut unavailable = fixture.clone();
    unavailable["packages"][1]["license"] = Value::Null;
    unavailable["packages"][1]["license_file"] = json!("LICENSE");
    let unavailable: Metadata = serde_json::from_value(unavailable).map_err(|e| e.to_string())?;
    require(has(&evaluate(&unavailable,&policy,&release,&scanner,&binding,as_of),"unavailable-license"),
        "license file without expression remains unavailable")?;
    let exception = Exception { package:"time".into(),version:"0.1.43".into(),source:CRATES_IO.into(),
        advisory:"RUSTSEC-2020-0071".into(), owner:"fixture-only".into(),rationale:"Synthetic control; not a real exception".into(),
        review_ref:"https://example.invalid/control-only".into(),reviewed_at:"2026-11-12".into(),expires:"2026-12-01".into() };
    policy.exceptions.push(exception);
    let excepted = evaluate(&metadata,&policy,&release,&affected,&binding,as_of);
    require(!excepted.denied() && excepted.findings.iter().any(|f| f.disposition == "excepted"
        && f.evidence["review"]["owner"] == "fixture-only"), "exact current review retained, not marked clean")?;
    policy.exceptions[0].version = "0.1.42".into();
    require(has(&evaluate(&metadata,&policy,&release,&affected,&binding,as_of),"stale-exception-binding"),
        "exception cannot cover another version and unused package binding is stale")?;
    policy.exceptions[0].version = "0.1.43".into();
    policy.exceptions[0].expires = "2026-11-12".into();
    require(has(&evaluate(&metadata,&policy,&release,&scanner,&binding,as_of),"expired-or-invalid-exception"),
        "expired unused exception is stale even without matching advisory")?;
    policy.exceptions.clear();
    let mut stale = binding.clone(); stale["database_date"] = json!("2026-10-01");
    require(has(&evaluate(&metadata,&policy,&release,&scanner,&stale,as_of),"stale-database"),"stale database denies")?;
    let mut mismatch = binding.clone(); mismatch["database_revision"] = json!("2222222222222222222222222222222222222222");
    require(has(&evaluate(&metadata,&policy,&release,&scanner,&mismatch,as_of),"stale-database"),"snapshot revision mismatch denies")?;
    require(has(&evaluate(&metadata,&policy,&release,&scanner,&binding,"2027-03-01"),"stale-scanner"),"old pinned scanner denies")?;
    require(evaluate(&metadata,&policy,&release,&json!({}),&binding,as_of).denied(),"empty supplied snapshot is not a scan")?;
    let mut empty = scanner.clone(); empty["database"]["advisory-count"] = json!(0);
    require(has(&evaluate(&metadata,&policy,&release,&empty,&binding,as_of),"stale-database"),"empty database denies")?;
    let mut suppressed = scanner.clone(); suppressed["settings"]["ignore"] = json!(["RUSTSEC-2020-0071"]);
    require(has(&evaluate(&metadata,&policy,&release,&suppressed,&binding,as_of),"scanner-settings"),"ambient ignore suppression denies")?;
    let mut wrong_profile = binding.clone(); wrong_profile["features"] = json!(["world-snapshot-vm-cohort"]);
    require(has(&evaluate(&metadata,&policy,&release,&scanner,&wrong_profile,as_of),"profile-binding"),"optional profile cannot masquerade as default")?;
    let mut root_features = fixture.clone(); root_features["resolve"]["nodes"][0]["features"] = json!(["spacewasm-pilot"]);
    let root_features: Metadata = serde_json::from_value(root_features).map_err(|e| e.to_string())?;
    require(has(&evaluate(&root_features,&policy,&release,&scanner,&binding,as_of),"profile-binding"),
        "metadata root optional-feature union cannot masquerade as default")?;
    let mut dev_features = fixture.clone(); dev_features["resolve"]["nodes"][0]["features"] = json!(["test-fixtures"]);
    let dev_features: Metadata = serde_json::from_value(dev_features).map_err(|e| e.to_string())?;
    require(!has(&evaluate(&dev_features,&policy,&release,&scanner,&binding,as_of),"profile-binding"),
        "the crate's own test-fixtures dev-dependency is reviewed")?;
    let mut extra_features = fixture.clone(); extra_features["resolve"]["nodes"][0]["features"] = json!(["test-fixtures","spacewasm-pilot"]);
    let extra_features: Metadata = serde_json::from_value(extra_features).map_err(|e| e.to_string())?;
    require(has(&evaluate(&extra_features,&policy,&release,&scanner,&binding,as_of),"profile-binding"),
        "test-fixtures does not admit any other root feature")?;
    let expected = reference(b"captured exact bytes");
    require(binding_matches(b"captured exact bytes", &expected) && !binding_matches(b"changed capture", &expected),
        "changed byte binding denies")?;
    require(day("2026-02-29").is_none() && day("2024-02-29").is_some() && !fresh("2026-11-14",as_of,14),
        "invalid and future dates deny")?;
    println!("dependency audit controls passed: closure/SPDX/affected/source/license/exact-review/expired/stale-database/stale-scanner/empty/suppressed/profile/stale-binding/date (fixture observations only)");
    Ok(())
}
