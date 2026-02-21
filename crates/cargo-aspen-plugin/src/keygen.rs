//! `cargo aspen-plugin keygen` — generate Ed25519 keypair.

use std::path::PathBuf;

use aspen_plugin_signing::keys;

pub fn run(output: Option<&str>) -> anyhow::Result<()> {
    let key = keys::generate_keypair();
    let pubkey = keys::public_key_hex(&key);

    let out_path = output.map(PathBuf::from).unwrap_or_else(default_key_path);

    // Don't overwrite existing keys
    if out_path.exists() {
        anyhow::bail!("key file already exists at {}. Remove it first or use --output.", out_path.display());
    }

    keys::save_secret_key(&out_path, &key)?;

    println!("✓ Generated Ed25519 keypair");
    println!("  Secret key: {}", out_path.display());
    println!("  Public key: {pubkey}");
    println!();
    println!("  Keep your secret key safe! Share only the public key.");
    println!("  Others can verify your plugins with:");
    println!("    cargo aspen-plugin verify --key {pubkey}");

    Ok(())
}

/// Default path for the signing key.
fn default_key_path() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    config_dir.join("aspen").join("plugin-signing-key")
}
