//! `cargo aspen-plugin sign` — sign the built WASM binary.

use std::path::Path;

use aspen_plugin_signing::keys;
use aspen_plugin_signing::signer;

pub fn run(key_path: &str) -> anyhow::Result<()> {
    // Load the signing key
    let signing_key = keys::load_secret_key(Path::new(key_path))?;

    // Find the WASM binary
    let wasm_path = find_wasm_binary()?;
    let wasm_bytes = std::fs::read(&wasm_path)?;

    println!("Signing {} ({} bytes)...", wasm_path.display(), wasm_bytes.len());

    // Sign
    let sig = signer::sign_plugin(&wasm_bytes, &signing_key);

    // Update plugin.json with signature info
    let manifest_path = "plugin.json";
    let manifest_data = std::fs::read_to_string(manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&manifest_data)?;

    manifest["signature"] = serde_json::json!({
        "author_pubkey": sig.author_pubkey,
        "signature": sig.signature,
        "wasm_hash": sig.wasm_hash,
        "signed_at_ms": sig.signed_at_ms,
    });

    let updated = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(manifest_path, updated)?;

    println!("✓ Signed with key {}", &sig.author_pubkey[..16]);
    println!("  WASM hash: {}", &sig.wasm_hash[..16]);
    println!("  Signature written to plugin.json");

    Ok(())
}

/// Find the release WASM binary in target/.
fn find_wasm_binary() -> anyhow::Result<std::path::PathBuf> {
    let release_dir = "target/wasm32-unknown-unknown/release";
    if let Ok(entries) = std::fs::read_dir(release_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "wasm") {
                return Ok(path);
            }
        }
    }

    // Fall back to debug
    let debug_dir = "target/wasm32-unknown-unknown/debug";
    if let Ok(entries) = std::fs::read_dir(debug_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "wasm") {
                return Ok(path);
            }
        }
    }

    anyhow::bail!("no .wasm file found in target/wasm32-unknown-unknown/. Run `cargo aspen-plugin build` first.")
}
