#!/usr/bin/env -S cargo -q -Zscript
---
[dependencies]
blake3 = "=1.8.5"
serde = { version = "=1.0.228", features = ["derive"] }
serde_json = "=1.0.149"
spdx = "=0.13.4"
---
//! Read-only normalizer for the pinned offline RustSec capture. JSON is observation
//! evidence, never an admission receipt. The shell owns processes and file capture.
// r[impl molten.dependency_audit.resolved]
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[path = "../config/dependency-audit/controls.rs"]
mod control_tests;

const LIMIT: u64 = 32 * 1024 * 1024;
const SCANNER: &str = "cargo-audit 0.23.0";
const SCANNER_DATE: &str = "2026-11-10";
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[derive(Default, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Option<Resolve>,
    workspace_members: Vec<String>,
    workspace_root: String,
}
#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    id: String,
    source: Option<String>,
    license: Option<String>,
    license_file: Option<String>,
    manifest_path: String,
}
#[derive(Deserialize)]
struct Resolve { nodes: Vec<Node> }
#[derive(Deserialize)]
struct Node {
    id: String,
    #[serde(default)]
    deps: Vec<NodeDependency>,
    #[serde(default)]
    features: Vec<String>,
}
#[derive(Deserialize)]
struct NodeDependency { pkg: String, dep_kinds: Vec<DependencyKind> }
#[derive(Deserialize)]
struct DependencyKind { kind: Option<String> }
#[derive(Default, Deserialize)]
struct Policy {
    schema: String,
    allowed_licenses: Vec<String>,
    allowed_exceptions: Vec<String>,
    exceptions: Vec<Exception>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Exception {
    package: String,
    version: String,
    source: String,
    advisory: String,
    owner: String,
    rationale: String,
    review_ref: String,
    reviewed_at: String,
    expires: String,
}
#[derive(Serialize)]
struct Finding {
    kind: String,
    package: String,
    version: String,
    source: String,
    dependency_path: Vec<String>,
    detail: String,
    disposition: String,
    evidence: Value,
}
#[derive(Default, Serialize)]
struct Evaluation {
    findings: Vec<Finding>,
    packages: Vec<Value>,
}
impl Evaluation {
    fn unavailable(&mut self, kind: &str, detail: impl Into<String>) {
        self.findings.push(Finding { kind: kind.into(), package: String::new(), version: String::new(),
            source: String::new(), dependency_path: vec![], detail: detail.into(),
            disposition: "unavailable".into(), evidence: Value::Null });
    }
    fn package_finding(&mut self, kind: &str, package: &Package, path: &[String], detail: impl Into<String>, evidence: Value) {
        self.findings.push(Finding { kind: kind.into(), package: package.name.clone(), version: package.version.clone(),
            source: source(package).into(), dependency_path: path.to_vec(), detail: detail.into(),
            disposition: "actionable".into(), evidence });
    }
    fn denied(&self) -> bool { self.findings.iter().any(|f| f.disposition != "excepted") }
}
fn source(package: &Package) -> &str { package.source.as_deref().unwrap_or("path") }
fn reference(bytes: &[u8]) -> String { format!("blake3:{}", blake3::hash(bytes).to_hex()) }
fn binding_matches(bytes: &[u8], expected: &str) -> bool { reference(bytes) == expected }
fn file_reference(path: &Path) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > 512 * 1024 * 1024 {
        return Err("scanner executable exceeds 512 MiB".into());
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file).map_err(|e| e.to_string())?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}
fn bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > LIMIT { return Err(format!("{}: not a bounded regular file", path.display())); }
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}
fn source_reference(path: &Path) -> Result<String, String> {
    if fs::symlink_metadata(path).map_err(|e| e.to_string())?.file_type().is_symlink() {
        let target = fs::read_link(path).map_err(|e| e.to_string())?;
        Ok(format!("symlink:{}", reference(target.as_os_str().as_encoded_bytes())))
    } else {
        bounded(path).map(|bytes| reference(&bytes))
    }
}
fn load<T: serde::de::DeserializeOwned + Default>(bundle: &Path, file: &str, eval: &mut Evaluation) -> T {
    match bounded(&bundle.join(file)).and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string())) {
        Ok(value) => value,
        Err(error) => { eval.unavailable("input-unavailable", format!("{file}: {error}")); T::default() }
    }
}
// Gregorian calendar validation and ordinal conversion; never compare unchecked strings.
fn day(date: &str) -> Option<i64> {
    if date.len() != 10 || date.as_bytes()[4] != b'-' || date.as_bytes()[7] != b'-' { return None; }
    let y: i64 = date.get(..4)?.parse().ok()?;
    let m: usize = date.get(5..7)?.parse().ok()?;
    let d: i64 = date.get(8..)?.parse().ok()?;
    if !(1970..=9999).contains(&y) || !(1..=12).contains(&m) { return None; }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let months = [31, if leap {29} else {28},31,30,31,30,31,31,30,31,30,31];
    if d < 1 || d > months[m - 1] { return None; }
    let prev = y - 1;
    Some(365 * prev + prev / 4 - prev / 100 + prev / 400 + months[..m - 1].iter().sum::<i64>() + d)
}
fn fresh(date: &str, as_of: &str, max_days: i64) -> bool {
    matches!((day(date), day(as_of)), (Some(d), Some(now)) if (0..=max_days).contains(&(now-d)))
}
// r[impl molten.dependency_audit.controls]
fn exception_valid(exception: &Exception, as_of: &str) -> bool {
    ![&exception.package, &exception.version, &exception.source, &exception.advisory, &exception.owner,
        &exception.rationale, &exception.review_ref].iter().any(|s| s.trim().is_empty())
        && ![&exception.package, &exception.version, &exception.source, &exception.advisory].iter().any(|s| s.contains('*'))
        && exception.review_ref.starts_with("https://")
        && matches!((day(&exception.reviewed_at), day(&exception.expires), day(as_of)),
            (Some(start), Some(end), Some(now)) if start <= now && now <= end && end > start && end-start <= 90)
}
fn license_allowed(expression: &str, policy: &Policy) -> Result<bool, String> {
    let expression = spdx::Expression::parse(expression).map_err(|e| e.to_string())?;
    Ok(expression.evaluate(|req| {
        let term = req.to_string();
        if let Some((license, exception)) = term.split_once(" WITH ") {
            policy.allowed_licenses.iter().any(|s| s == license)
                && policy.allowed_exceptions.iter().any(|s| s == exception)
        } else { policy.allowed_licenses.contains(&term) }
    }))
}
fn source_allowed(package: &Package, release: &Value, workspace: &BTreeSet<&str>) -> bool {
    if workspace.contains(package.id.as_str()) && package.source.is_none() { return true; }
    if source(package) == CRATES_IO { return true; }
    let Some(git) = source(package).strip_prefix("git+") else { return false; };
    let Some((coordinate, revision)) = git.rsplit_once('#') else { return false; };
    if revision.len() != 40 || !revision.bytes().all(|b| b.is_ascii_hexdigit()) { return false; }
    let coordinate = coordinate.split('?').next().unwrap_or(coordinate);
    release.get("dependencies").and_then(Value::as_array).is_some_and(|rows| rows.iter().any(|row| {
        row["source_coordinate"].as_str() == Some(coordinate) && row["immutable_revision"].as_str() == Some(revision)
    }))
}
fn paths(metadata: &Metadata) -> BTreeMap<String, Vec<String>> {
    let nodes: BTreeMap<&str, &Node> = metadata.resolve.iter().flat_map(|r| &r.nodes).map(|n| (n.id.as_str(), n)).collect();
    let roots: Vec<&Package> = metadata.packages.iter().filter(|p| p.name == "molten" && metadata.workspace_members.contains(&p.id)).collect();
    let mut result = BTreeMap::new();
    let mut queue = VecDeque::new();
    for root in roots { queue.push_back((root.id.clone(), vec![root.id.clone()])); }
    while let Some((id, path)) = queue.pop_front() {
        if result.contains_key(&id) { continue; }
        if let Some(node) = nodes.get(id.as_str()) {
            for target in node.deps.iter().filter(|d| d.dep_kinds.iter().any(|k| k.kind.as_deref() != Some("dev"))) {
                let mut next = path.clone(); next.push(target.pkg.clone()); queue.push_back((target.pkg.clone(), next));
            }
        }
        result.insert(id, path);
    }
    result
}
fn evaluate(metadata: &Metadata, policy: &Policy, release: &Value, scanner: &Value, binding: &Value, as_of: &str) -> Evaluation {
    let mut eval = Evaluation::default();
    if day(as_of).is_none() { eval.unavailable("invalid-date", "as-of must be a Gregorian YYYY-MM-DD"); }
    if policy.schema != "molten.dependency-audit-policy.v2" { eval.unavailable("policy-schema", "missing or unsupported audit policy"); }
    if release["schema"] != "molten.release-dependency-profile.v2" { eval.unavailable("release-profile", "missing existing release profile"); }
    if binding["schema"] != "molten.dependency-audit-binding.v2" || binding["profile"] != "molten-default"
        || binding["features"] != json!([]) || binding["default_features"] != true
        || binding["target"].as_str().is_none_or(str::is_empty) {
        eval.unavailable("profile-binding", "only an explicit target and molten-default features are reviewed");
    }
    if binding["scanner_version"] != SCANNER || !fresh(SCANNER_DATE, as_of, 90) {
        eval.unavailable("stale-scanner", "scanner version differs from pin or pinned release is older than 90 days");
    }
    let revision = binding["database_revision"].as_str().unwrap_or("");
    let database_date = binding["database_date"].as_str().unwrap_or("");
    // --no-fetch uses Database::open: cargo-audit deliberately omits Git metadata.
    // The capture's independently verified Git commit/date remain mandatory below.
    let reported_revision = &scanner["database"]["last-commit"];
    let reported_date = &scanner["database"]["last-updated"];
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        || scanner["database"].get("last-commit").is_none() || scanner["database"].get("last-updated").is_none()
        || (!reported_revision.is_null() && reported_revision.as_str() != Some(revision))
        || scanner["database"]["advisory-count"].as_u64().unwrap_or(0) == 0
        || !fresh(database_date, as_of, 14)
        || (!reported_date.is_null() && reported_date.as_str().and_then(|s| s.get(..10)) != Some(database_date)) {
        eval.unavailable("stale-database", "RustSec revision/date/count missing, mismatched, future or older than 14 days");
    }
    let settings = &scanner["settings"];
    if settings["ignore"] != json!([]) || !settings["severity"].is_null() || settings["target_arch"] != json!([])
        || settings["target_os"] != json!([])
        || !["unmaintained", "unsound", "notice"].iter().all(|kind| settings["informational_warnings"].as_array().is_some_and(|a| a.contains(&json!(kind)))) {
        eval.unavailable("scanner-settings", "scan settings omitted/suppressed required RustSec observations");
    }
    let list = scanner["vulnerabilities"]["list"].as_array();
    if list.is_none() || scanner["vulnerabilities"]["count"].as_u64() != list.map(|a| a.len() as u64)
        || scanner["vulnerabilities"]["found"].as_bool() != list.map(|a| !a.is_empty())
        || !scanner["warnings"].is_object() {
        eval.unavailable("scanner-output", "missing or inconsistent raw RustSec machine output");
    }
    let paths = paths(metadata);
    for exception in &policy.exceptions {
        if !exception_valid(exception, as_of) {
            eval.unavailable("expired-or-invalid-exception", format!("{} {} {} review {} expires {}", exception.package,
                exception.version, exception.advisory, exception.review_ref, exception.expires));
        }
        if !metadata.packages.iter().any(|p| paths.contains_key(&p.id) && p.name == exception.package
            && p.version == exception.version && source(p) == exception.source) {
            eval.unavailable("stale-exception-binding", format!("review {} no longer binds a selected exact package/version/source", exception.review_ref));
        }
    }
    if metadata.resolve.is_none() || paths.is_empty() { eval.unavailable("resolved-closure", "no resolved molten root/default closure"); }
    if paths.keys().any(|id| !metadata.packages.iter().any(|p| &p.id == id)
        || !metadata.resolve.iter().flat_map(|r| &r.nodes).any(|n| &n.id == id)) {
        eval.unavailable("resolved-closure", "selected dependency graph contains a missing package or resolve node");
    }
    // The reviewed default profile enables no root features. The crate's own
    // dev-dependency `molten = { path = ".", features = ["test-fixtures"] }` makes
    // cargo report that one feature on the root node of every `cargo metadata`
    // resolve, so exactly that set is reviewed; any other root feature — a changed
    // default, an optional union or a new dev feature — still requires review.
    let reviewed_root_features: BTreeSet<String> = ["test-fixtures"].iter().map(|feature| (*feature).to_string()).collect();
    if metadata.resolve.iter().flat_map(|r| &r.nodes).any(|n| !n.features.is_empty()
        && metadata.packages.iter().any(|p| p.id == n.id && p.name == "molten" && metadata.workspace_members.contains(&p.id))
        && n.features.iter().cloned().collect::<BTreeSet<String>>() != reviewed_root_features) {
        eval.unavailable("profile-binding", "resolved root features differ from the reviewed default profile plus the crate's own test-fixtures dev-dependency");
    }
    let workspace: BTreeSet<&str> = metadata.workspace_members.iter().map(String::as_str).collect();
    let mut observations: Vec<&Value> = list.into_iter().flatten().collect();
    if let Some(warnings) = scanner["warnings"].as_object() {
        for values in warnings.values() {
            if let Some(values) = values.as_array() { observations.extend(values); }
            else { eval.unavailable("scanner-output", "malformed RustSec warning group"); }
        }
    }
    for package in &metadata.packages {
        let Some(path) = paths.get(&package.id) else { continue; };
        if !source_allowed(package, release, &workspace) {
            eval.package_finding("source-policy", package, path, "source/revision has no exact existing release-profile binding", Value::Null);
        }
        match package.license.as_deref().map(|s| license_allowed(s, policy)) {
            Some(Ok(true)) => {},
            Some(Ok(false)) => eval.package_finding("license-policy", package, path, "SPDX expression is outside reviewed audit license choices", json!(package.license)),
            _ => eval.package_finding("unavailable-license", package, path, "missing or invalid SPDX expression; license-file alone is not an expression", json!(package.license)),
        }
        let features = metadata.resolve.iter().flat_map(|r| &r.nodes).find(|n| n.id == package.id).map(|n| &n.features);
        eval.packages.push(json!({"id":package.id,"name":package.name,"version":package.version,"source":source(package),
            "manifest_path":package.manifest_path,"license_expression":package.license,"declared_license_file":package.license_file,
            "features":features,"dependency_path":path}));
        for observation in &observations {
            let observed = &observation["package"];
            if observed["name"] != package.name || observed["version"] != package.version
                || observed["source"].as_str() != package.source.as_deref() { continue; }
            let advisory = observation["advisory"]["id"].as_str().unwrap_or("");
            eval.package_finding("advisory", package, path,
                format!("{}: {}", advisory, observation["advisory"]["title"].as_str().unwrap_or("RustSec warning")), (*observation).clone());
            if let Some(exception) = policy.exceptions.iter().find(|e| e.package == package.name && e.version == package.version
                && e.source == source(package) && e.advisory == advisory && exception_valid(e, as_of)) {
                if let Some(last) = eval.findings.last_mut() {
                    last.disposition = "excepted".into();
                    last.evidence = json!({"scanner":observation,"review":exception});
                }
            }
        }
    }
    eval
}
fn commit_day(bytes: &[u8]) -> Option<i64> {
    let text = std::str::from_utf8(bytes).ok()?;
    let committer = text.split_once("\n\n")?.0.lines().find_map(|line| line.strip_prefix("committer "))?;
    let seconds: i64 = committer.split_whitespace().rev().nth(1)?.parse().ok()?;
    day("1970-01-01")?.checked_add(seconds.div_euclid(86_400))
}

fn validate_database_capture(bundle: &Path, binding: &Value, eval: &mut Evaluation) {
    // Recompute the interoperability Git object identity from the retained bytes;
    // never insert an invented commit into the scanner's original JSON.
    let check = || -> Result<(), String> {
        use std::io::Write;
        let commit = bounded(&bundle.join("database-commit.txt"))?;
        let mut child = std::process::Command::new("git")
            .args(["hash-object", "-t", "commit", "--stdin"])
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null()).spawn().map_err(|error| error.to_string())?;
        child.stdin.take().ok_or("Git commit input unavailable")?.write_all(&commit)
            .map_err(|error| error.to_string())?;
        let output = child.wait_with_output().map_err(|error| error.to_string())?;
        let revision = std::str::from_utf8(&output.stdout).map_err(|error| error.to_string())?.trim();
        let expected_date = binding["database_date"].as_str().and_then(day);
        if !output.status.success() || binding["database_revision"].as_str() != Some(revision)
            || expected_date.is_none() || commit_day(&commit) != expected_date {
            return Err("retained RustSec Git commit identity/date mismatch".into());
        }
        Ok(())
    };
    if let Err(error) = check() { eval.unavailable("stale-database", error); }
}

fn validate_bindings(root: &Path, bundle: &Path, binding: &Value, eval: &mut Evaluation) {
    let required = ["metadata.json", "policy.json", "release-profile.json", "scanner.json", "scanner.stderr", "scanner.version",
        "Cargo.lock", "source-files.json", "status.json", "rustc.txt", "cargo.txt", "database-tree.txt", "database-commit.txt",
        "notices.json", "license-guard.txt", "release-guard.json", "source-guard.txt"];
    validate_database_capture(bundle, binding, eval);
    let Some(files) = binding["files"].as_object() else { eval.unavailable("stale-binding", "no captured-file bindings"); return; };
    for file in required {
        match bounded(&bundle.join(file)) {
            Ok(bytes) if files.get(file).and_then(Value::as_str) == Some(reference(&bytes).as_str()) => {},
            _ => eval.unavailable("stale-binding", format!("{file}: missing or mismatched bytes")),
        }
    }
    for (file, expected) in files {
        if Path::new(file).components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            eval.unavailable("stale-binding", "unsafe captured input path"); continue;
        }
        if !bounded(&bundle.join(file)).is_ok_and(|bytes| binding_matches(&bytes, expected.as_str().unwrap_or(""))) {
            eval.unavailable("stale-binding", format!("{file}: mismatched capture"));
        }
    }
    let source_files: BTreeMap<String, String> = load(bundle, "source-files.json", eval);
    for required in ["Cargo.toml", "Cargo.lock", "flake.lock", "build-plan.json", "release-policy-build-plan.json", "license-policy.tsv",
        "THIRD_PARTY_LICENSES.md", "config/dependency-audit/policy.ncl", "config/release-dependencies/profile.ncl", "tools/dependency-audit.rs", "scripts/dependency-audit.sh"] {
        if !source_files.contains_key(required) { eval.unavailable("stale-binding", format!("source inventory omits {required}")); }
    }
    for (file, expected) in source_files {
        if Path::new(&file).components().any(|c| !matches!(c, std::path::Component::Normal(_)))
            || !source_reference(&root.join(&file)).is_ok_and(|value| value == expected) {
            eval.unavailable("stale-binding", format!("source input changed or unavailable: {file}"));
        }
    }
    if bounded(&root.join("Cargo.lock")).ok() != bounded(&bundle.join("Cargo.lock")).ok() {
        eval.unavailable("stale-binding", "capture lock differs from source lock");
    }
    let scanner_path = binding["scanner_path"].as_str().unwrap_or("");
    if !file_reference(Path::new(scanner_path)).is_ok_and(|value| binding["scanner_ref"].as_str() == Some(value.as_str())) {
        eval.unavailable("stale-binding", "pinned scanner executable unavailable or changed");
    }
    let normalizer_path = binding["normalizer_path"].as_str().unwrap_or("");
    if !file_reference(Path::new(normalizer_path)).is_ok_and(|value| binding["normalizer_ref"].as_str() == Some(value.as_str())) {
        eval.unavailable("stale-binding", "pinned normalizer executable unavailable or changed");
    }
    let status: Value = load(bundle, "status.json", eval);
    for operation in ["metadata", "policy", "release_profile", "license_guard", "release_guard", "source_guard", "notice_capture", "rustc", "cargo"] {
        if status[operation] != 0 { eval.unavailable("prerequisite-unavailable", format!("{operation}: nonzero or absent status; inspect retained stderr")); }
    }
    if !matches!(status["scanner"].as_i64(), Some(0 | 1)) {
        eval.unavailable("scanner-unavailable", "scanner failed; inspect scanner.stderr (never a clean scan)");
    }
}
fn capture_notice_tree(bundle: &Path, base: &Path, relative: &Path, total: &mut usize, depth: usize) -> Result<Vec<Value>, String> {
    let unavailable = |reason: &str| vec![json!({"path":relative,"unavailable":reason})];
    let Ok(canonical_base) = base.canonicalize() else { return Ok(unavailable("package source root unavailable")); };
    let Ok(canonical) = base.join(relative).canonicalize() else { return Ok(unavailable("declared/discovered notice unavailable")); };
    if !canonical.starts_with(&canonical_base) { return Ok(unavailable("notice escapes package root")); }
    if canonical.is_dir() {
        if depth == 4 { return Ok(unavailable("notice directory depth exceeds four")); }
        let Ok(entries) = fs::read_dir(&canonical) else { return Ok(unavailable("notice directory unreadable")); };
        let mut children = Vec::new();
        for entry in entries.take(257) {
            let Ok(entry) = entry else { return Ok(unavailable("notice directory entry unreadable")); };
            children.push(relative.join(entry.file_name()));
        }
        if children.is_empty() || children.len() > 256 { return Ok(unavailable("empty or oversized notice directory")); }
        children.sort();
        let mut evidence = Vec::new();
        for child in children { evidence.extend(capture_notice_tree(bundle, base, &child, total, depth + 1)?); }
        return Ok(evidence);
    }
    let Ok(bytes) = bounded(&canonical) else { return Ok(unavailable("notice is unreadable or oversized")); };
    if bytes.is_empty() || bytes.len() > 1024 * 1024 || *total + bytes.len() > LIMIT as usize {
        return Ok(unavailable("notice byte budget exhausted or empty artifact"));
    }
    *total += bytes.len();
    let hash = blake3::hash(&bytes).to_hex().to_string();
    fs::create_dir_all(bundle.join("notices")).map_err(|e| e.to_string())?;
    fs::write(bundle.join("notices").join(&hash), &bytes).map_err(|e| e.to_string())?;
    Ok(vec![json!({"path":relative,"ref":reference(&bytes),"artifact":format!("notices/{hash}"),"bytes":bytes.len()})])
}
fn capture_notices(bundle: &Path, metadata: &Metadata) -> Result<BTreeMap<String, Vec<Value>>, String> {
    let mut inventory = BTreeMap::new();
    let mut total = 0usize;
    let selected = paths(metadata);
    for package in &metadata.packages {
        if !selected.contains_key(&package.id) { continue; }
        let mut evidence = Vec::new();
        let base = if metadata.workspace_members.contains(&package.id) { Path::new(&metadata.workspace_root) }
            else { Path::new(&package.manifest_path).parent().unwrap_or(Path::new("")) };
        let mut candidates = BTreeSet::new();
        if let Some(declared) = &package.license_file { candidates.insert(PathBuf::from(declared)); }
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let upper = name.to_ascii_uppercase();
                if ["LICENSE", "LICENCE", "NOTICE", "COPYING", "COPYRIGHT", "AUTHORS"].iter().any(|prefix| upper.starts_with(prefix)) {
                    candidates.insert(PathBuf::from(name));
                }
            }
        }
        for relative in candidates {
            evidence.extend(capture_notice_tree(bundle, base, &relative, &mut total, 0)?);
        }
        for artifact in &mut evidence {
            artifact["scope"] = json!(if metadata.workspace_members.contains(&package.id) { "workspace-owned-policy" } else { "package-source" });
        }
        inventory.insert(package.id.clone(), evidence);
    }
    Ok(inventory)
}
fn notices(bundle: &Path, metadata: &Metadata, eval: &mut Evaluation) {
    let inventory: BTreeMap<String, Vec<Value>> = load(bundle, "notices.json", eval);
    for package in &metadata.packages {
        let Some(row) = eval.packages.iter_mut().find(|p| p["id"] == package.id) else { continue; };
        let evidence = inventory.get(&package.id).cloned().unwrap_or_default();
        row["notice_evidence"] = json!(evidence);
        let path: Vec<String> = row["dependency_path"].as_array().into_iter().flatten().filter_map(|s| s.as_str().map(str::to_string)).collect();
        let valid = !evidence.is_empty() && evidence.iter().all(|e| {
            let artifact = e["artifact"].as_str().unwrap_or("");
            let Some(hash) = artifact.strip_prefix("notices/") else { return false; };
            hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                && bounded(&bundle.join(artifact)).is_ok_and(|bytes| !bytes.is_empty()
                    && e["ref"].as_str() == Some(reference(&bytes).as_str()) && e["bytes"].as_u64() == Some(bytes.len() as u64))
        });
        if !valid {
            eval.package_finding("unavailable-notices", package, &path, "no complete, bound local license/notice evidence; legal obligations remain a separate review", Value::Null);
        }
    }
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(denied) => if denied { std::process::ExitCode::FAILURE } else { std::process::ExitCode::SUCCESS },
        Err(error) => { eprintln!("dependency audit unavailable: {error}"); std::process::ExitCode::FAILURE }
    }
}
fn run() -> Result<bool, String> {
    let mut args = std::env::args().skip(1);
    let mut bundle = None; let mut root = None; let mut as_of = None; let mut capture = None;
    while let Some(arg) = args.next() {
        if arg == "--self-test" { return control_tests::run().map(|()| false); }
        let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
        match arg.as_str() { "--bundle" => bundle = Some(PathBuf::from(value)), "--root" => root = Some(PathBuf::from(value)),
            "--capture-notices" => capture = Some(PathBuf::from(value)),
            "--as-of" => as_of = Some(value), _ => return Err(format!("unknown argument {arg}")) }
    }
    let bundle = bundle.ok_or("missing --bundle")?;
    if let Some(path) = capture {
        let metadata: Metadata = serde_json::from_slice(&bounded(&path)?).map_err(|e| e.to_string())?;
        println!("{}", serde_json::to_string_pretty(&capture_notices(&bundle, &metadata)?).map_err(|e| e.to_string())?);
        return Ok(false);
    }
    let root = root.ok_or("missing --root")?;
    let as_of = as_of.ok_or("missing --as-of")?;
    let mut inputs = Evaluation::default();
    let metadata: Metadata = load(&bundle, "metadata.json", &mut inputs);
    let policy: Policy = load(&bundle, "policy.json", &mut inputs);
    let release: Value = load(&bundle, "release-profile.json", &mut inputs);
    let scanner: Value = load(&bundle, "scanner.json", &mut inputs);
    let binding: Value = load(&bundle, "binding.json", &mut inputs);
    let mut eval = evaluate(&metadata, &policy, &release, &scanner, &binding, &as_of);
    eval.findings.extend(inputs.findings);
    validate_bindings(&root, &bundle, &binding, &mut eval);
    notices(&bundle, &metadata, &mut eval);
    eval.findings.sort_by(|a,b| (&a.package,&a.version,&a.kind,&a.detail).cmp(&(&b.package,&b.version,&b.kind,&b.detail)));
    let denied = eval.denied();
    let report = json!({"schema":"molten.dependency-audit-report.v2","decision":if denied {"deny"} else {"pass"},"as_of":as_of,
        "binding_ref":reference(&bounded(&bundle.join("binding.json"))?),"binding":binding,"evaluation":eval,
        "non_claim":"Observation only: not legal/distribution approval, release authority, runtime security qualification or a vulnerability-free claim. Yanked registry state is not queried; this scan is offline."});
    fs::write(bundle.join("report.json"), serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut summary = format!("# Resolved dependency audit\n\nDecision: **{}**. Profile: `molten-default`. Target: `{}`. As of: `{as_of}`.\n\nScanner: `{SCANNER}`. RustSec: `{}`.\n\n{} resolved packages; {} findings. Complete paths, expressions, notices, exceptions and exact input refs: `report.json`. Raw maintained scan: `scanner.json`; failures: `status.json` and retained stderr.\n\n",
        report["decision"].as_str().unwrap_or("deny"),binding["target"].as_str().unwrap_or("unavailable"),binding["database_revision"].as_str().unwrap_or("unavailable"),eval.packages.len(),eval.findings.len());
    for finding in eval.findings.iter().take(100) {
        summary.push_str(&format!("- {}: {} {} [{}] {}\n", finding.kind, finding.package, finding.version, finding.disposition, finding.detail.replace(['\n','\r'], " ")));
    }
    if eval.findings.len() > 100 { summary.push_str("\nSummary truncated at 100 findings; machine report is complete.\n"); }
    summary.push_str("\nNo legal/distribution approval, release authority or runtime security qualification is implied.\n");
    fs::write(bundle.join("summary.md"), summary).map_err(|e| e.to_string())?;
    println!("dependency audit {}: {} packages, {} findings; {}",if denied {"deny"} else {"pass"},eval.packages.len(),eval.findings.len(),bundle.display());
    Ok(denied)
}
