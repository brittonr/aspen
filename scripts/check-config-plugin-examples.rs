#!/usr/bin/env -S RUSTC_WRAPPER= CARGO_INCREMENTAL= nix develop -c cargo -q -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1"
tempfile = "3"
---

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;

const PLUGIN_EXAMPLE: &str = r##"
use aspen_plugin_api::{
    plugin_kv_key_allowed, protocol_identifier_collisions, validate_plugin_host_access,
    PluginHostAccess, PluginManifest, PluginProtocol,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_json = r#"{
        "name": "standalone-config-plugin-example",
        "version": "1.2.3",
        "wasm_hash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "handles": ["ConfigExample"],
        "protocols": [{
            "identifier": "aspen.example.config.v1",
            "version": 1,
            "max_concurrent_sessions": 4,
            "max_chunk_size_bytes": 4096,
            "max_in_flight_bytes": 16384,
            "session_timeout_ms": 30000
        }],
        "priority": 900,
        "fuel_limit": null,
        "memory_limit": null,
        "enabled": true,
        "kv_prefixes": ["example:"],
        "permissions": { "kv_read": true, "blob_read": true },
        "dependencies": [{ "name": "core-schema", "min_version": "1.0.0", "optional": false }]
    }"#;

    let manifest: PluginManifest = serde_json::from_str(manifest_json)?;
    assert!(plugin_kv_key_allowed(&manifest, "example:key"));
    assert!(!plugin_kv_key_allowed(&manifest, "other:key"));
    assert!(validate_plugin_host_access(&manifest, PluginHostAccess::KvRead { key: "example:key" }).is_ok());
    assert!(validate_plugin_host_access(&manifest, PluginHostAccess::KvWrite { key: "example:key" }).is_err());
    assert!(
        validate_plugin_host_access(&manifest, PluginHostAccess::Protocol { identifier: "aspen.example.config.v1" })
            .is_ok()
    );

    let mut duplicate = manifest.clone();
    duplicate.name = "duplicate-example".to_string();
    duplicate.protocols = vec![PluginProtocol {
        identifier: "aspen.example.config.v1".to_string(),
        version: 1,
        max_concurrent_sessions: 1,
        max_chunk_size_bytes: 1024,
        max_in_flight_bytes: 1024,
        session_timeout_ms: 1000,
    }];
    let collisions = protocol_identifier_collisions(&[manifest.clone(), duplicate]);
    assert_eq!(collisions.len(), 1);

    let serialized = serde_json::to_string(&manifest)?;
    assert!(serialized.contains("standalone-config-plugin-example"));
    Ok(())
}
"##;

const NODE_CONFIG_FIXTURE: &str = r#"let schema = import "@REPO@/crates/aspen-nickel/src/schema/node_config.ncl" in
{
  node_id = 1,
  cookie = "standalone-config-plugin-example",
  bootstrap_peers = [{ node_id = 2, endpoint = "iroh://peer" }],
  feature_bundles = [{ name = 'minimal }],
  metrics = { prometheus = true, scrape_interval_secs = 10 },
  trust = { policy_id = "trust-main", threshold = 2, secret_ref = "sops://cluster/share-1" },
} | schema.NodeConfig
"#;

fn run(command: &mut Command) -> Result<String> {
    let rendered = format!("{:?}", command);
    let output = command.output().with_context(|| format!("failed to run {rendered}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "command failed: {rendered}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn nickel_base() -> Vec<String> {
    if let Ok(path) = std::env::var("ASPEN_NICKEL_BIN") {
        return vec![path];
    }
    if Command::new("nickel").arg("--version").output().is_ok() {
        return vec!["nickel".to_string()];
    }
    vec![
        "nix".to_string(),
        "run".to_string(),
        "nixpkgs#nickel".to_string(),
        "--".to_string(),
    ]
}

fn main() -> Result<()> {
    let repo = std::env::current_dir().context("current working directory")?;
    let temp = tempfile::tempdir().context("create temp example workspace")?;
    let example_root = temp.path().join("plugin-api-example");
    fs::create_dir_all(example_root.join("src"))?;
    fs::write(
        example_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"config-plugin-standalone-example\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\naspen-plugin-api = {{ path = {:?} }}\nserde_json = \"1\"\n",
            repo.join("crates/aspen-plugin-api")
        ),
    )?;
    fs::write(example_root.join("src/main.rs"), PLUGIN_EXAMPLE)?;
    run(Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(example_root.join("Cargo.toml")))?;

    let fixture = temp.path().join("node-config-positive.ncl");
    fs::write(&fixture, NODE_CONFIG_FIXTURE.replace("@REPO@", repo.to_str().context("repo path utf8")?))?;
    let base = nickel_base();
    let nickel = Path::new(&base[0]);
    let mut typecheck = Command::new(nickel);
    typecheck.args(&base[1..]).arg("typecheck").arg("crates/aspen-nickel/src/schema/node_config.ncl");
    run(&mut typecheck)?;
    let mut export = Command::new(nickel);
    export.args(&base[1..]).arg("export").arg("--format").arg("json").arg(&fixture);
    run(&mut export)?;

    println!("config/plugin standalone examples OK: plugin-api manifest/protocol example + Nickel node config fixture");
    Ok(())
}
