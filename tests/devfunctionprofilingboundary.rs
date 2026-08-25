#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "borrowed source fixtures make the structural boundary test explicit"
)]
#![allow(
    tigerstyle::no_recursion,
    reason = "the test walks a finite repository source tree without following links"
)]
#![allow(
    tigerstyle::no_unwrap,
    reason = "repository test setup failures must stop the test with their local context"
)]
#![allow(
    tigerstyle::non_trait_imports,
    reason = "filesystem and path modules are explicit test-shell dependencies"
)]
#![allow(
    tigerstyle::unbounded_collection_growth,
    reason = "the repository source tree and diagnostic sets bound these test-only vectors"
)]

use std::fs;
use std::path::Path;

const FLUX_REVISION: &str = "2a1916465ae6649aebef3758233cfea98e5d33db";
const PROFILER_TOKENS: &[&str] = &["flux_profiler", "flux-profiler", "#[timed", "enable_profiler"];

fn dependency_issues(cargo_manifest: &str, flake: &str) -> Vec<&'static str> {
    let required_cargo_fragments = [
        "git = \"https://github.com/gattaca-com/flux\"",
        "optional = true",
        "default-features = false",
        "cfg(not(all(target_arch = \"x86_64\", target_os = \"linux\")))",
        "features = [\"disable-profiling\"]",
        FLUX_REVISION,
    ];
    let mut issues = Vec::new();
    if required_cargo_fragments.iter().any(|fragment| !cargo_manifest.contains(fragment)) {
        issues.push("Cargo dependency is not pinned and optional");
    }
    if !flake.contains(&format!("github:gattaca-com/flux/{FLUX_REVISION}")) {
        issues.push("Nix source is not pinned to the Cargo revision");
    }
    issues
}

fn core_placement_issues(files: &[(&str, &str)]) -> Vec<String> {
    let mut issues = Vec::new();
    for (path, source) in files {
        for token in PROFILER_TOKENS {
            if source.contains(token) {
                issues.push(format!("{path} contains forbidden profiler token {token}"));
            }
        }
    }
    issues
}

fn capture_command_issues(document: &str) -> Vec<String> {
    document
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("flux-profiler "))
        .filter(|line| !line.contains("--duration") && !line.contains("--max-mem"))
        .map(|line| format!("unbounded capture command: {line}"))
        .collect()
}

fn collect_rust_files(root: &Path, files: &mut Vec<(String, String)>) {
    if !root.exists() {
        return;
    }
    for entry in fs::read_dir(root).expect("read core directory") {
        let path = entry.expect("read core entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push((path.display().to_string(), fs::read_to_string(&path).expect("read core source")));
        }
    }
}

#[test]
fn repository_uses_one_pinned_profiler_revision() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_manifest = fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo manifest");
    let flake = fs::read_to_string(root.join("flake.nix")).expect("read flake");
    assert_eq!(dependency_issues(&cargo_manifest, &flake), Vec::<&str>::new());
}

#[test]
fn unpinned_or_mismatched_dependency_is_denied() {
    let unpinned = "flux-profiler = { git = \"https://github.com/gattaca-com/flux\", branch = \"main\" }";
    let mismatched_flake = "flux-src.url = \"github:gattaca-com/flux/other-revision\";";
    assert!(!dependency_issues(unpinned, mismatched_flake).is_empty());
}

#[test]
fn pure_cores_have_no_profiler_side_effects() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut owned_files = Vec::new();
    collect_rust_files(&root.join("crates/molten-core"), &mut owned_files);
    collect_rust_files(&root.join("crates/aspen-core"), &mut owned_files);
    let borrowed_files: Vec<_> = owned_files.iter().map(|(path, source)| (path.as_str(), source.as_str())).collect();
    assert_eq!(core_placement_issues(&borrowed_files), Vec::<String>::new());
}

#[test]
fn profiler_reference_in_a_pure_core_is_denied() {
    let fixture = [("crates/molten-core/src/bad.rs", "#[timed] fn transition() {}")];
    assert_eq!(core_placement_issues(&fixture).len(), 1);
}

#[test]
fn documented_capture_is_bounded() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let document =
        fs::read_to_string(root.join("docs/development-function-profiling.md")).expect("read profiling guide");
    assert_eq!(capture_command_issues(&document), Vec::<String>::new());
}

#[test]
fn unbounded_capture_command_is_denied() {
    let fixture = "```sh\nflux-profiler --out target/unbounded.fxt\n```";
    assert_eq!(capture_command_issues(fixture).len(), 1);
}
