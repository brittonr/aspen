//! `cargo aspen-plugin verify` — verify WASM binary signature.

use aspen_plugin_signing::PluginSignature;
use aspen_plugin_signing::verifier;

pub fn run(expected_key: Option<&str>) -> anyhow::Result<()> {
    // Read plugin.json
    let manifest_data = std::fs::read_to_string("plugin.json")?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_data)?;

    // Extract signature
    let sig_value = manifest
        .get("signature")
        .ok_or_else(|| anyhow::anyhow!("no 'signature' field in plugin.json — plugin is unsigned"))?;

    let sig: PluginSignature = serde_json::from_value(sig_value.clone())?;

    // Check expected key if provided
    if let Some(expected) = expected_key {
        if sig.author_pubkey != expected {
            anyhow::bail!("author key mismatch: expected {}, got {}", expected, sig.author_pubkey);
        }
    }

    // Find and read the WASM binary
    let wasm_path = find_wasm_binary()?;
    let wasm_bytes = std::fs::read(&wasm_path)?;

    println!("Verifying {} ({} bytes)...", wasm_path.display(), wasm_bytes.len());

    // Verify
    verifier::verify_plugin(&wasm_bytes, &sig)?;

    println!("✓ Signature valid");
    println!("  Author: {}", &sig.author_pubkey[..16]);
    println!("  WASM hash: {}", &sig.wasm_hash[..16]);

    Ok(())
}

/// Find the WASM binary (same logic as sign.rs).
fn find_wasm_binary() -> anyhow::Result<std::path::PathBuf> {
    for profile in &["release", "debug"] {
        let dir = format!("target/wasm32-unknown-unknown/{profile}");
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "wasm") {
                    return Ok(path);
                }
            }
        }
    }
    anyhow::bail!("no .wasm file found in target/wasm32-unknown-unknown/. Run `cargo aspen-plugin build` first.")
}
