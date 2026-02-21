//! `cargo aspen-plugin build` — compile plugin to WASM.

use std::process::Command;

pub fn run(release: bool) -> anyhow::Result<()> {
    // Verify we're in a plugin directory
    if !std::path::Path::new("Cargo.toml").exists() {
        anyhow::bail!("no Cargo.toml found — run this from a plugin directory");
    }
    if !std::path::Path::new("plugin.json").exists() {
        anyhow::bail!("no plugin.json found — run this from a plugin directory");
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--target").arg("wasm32-unknown-unknown");

    if release {
        cmd.arg("--release");
    }

    println!("Building plugin for wasm32-unknown-unknown...");
    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("build failed with exit code {}", status);
    }

    // Find the output .wasm file
    let profile = if release { "release" } else { "debug" };
    let target_dir = format!("target/wasm32-unknown-unknown/{profile}");

    if let Ok(entries) = std::fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "wasm") {
                println!("✓ Built: {}", path.display());
                return Ok(());
            }
        }
    }

    println!("✓ Build succeeded (output in {target_dir}/)");
    Ok(())
}
