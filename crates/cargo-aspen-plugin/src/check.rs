//! `cargo aspen-plugin check` — validate plugin configuration.

use aspen_plugin_api::PluginInfo;

pub fn run() -> anyhow::Result<()> {
    println!("Checking plugin configuration...\n");

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check plugin.json exists and is valid
    let manifest_path = "plugin.json";
    if !std::path::Path::new(manifest_path).exists() {
        anyhow::bail!("no plugin.json found — run this from a plugin directory");
    }

    let manifest_data = std::fs::read_to_string(manifest_path)?;
    let info: PluginInfo = match serde_json::from_str(&manifest_data) {
        Ok(m) => m,
        Err(e) => {
            anyhow::bail!("invalid plugin.json: {e}");
        }
    };

    // Validate name
    if info.name.is_empty() {
        errors.push("plugin name is empty".to_string());
    }

    // Validate version
    if info.version.is_empty() {
        errors.push("plugin version is empty".to_string());
    }

    // Validate priority
    if !(900..=999).contains(&info.priority) {
        errors.push(format!("priority {} is out of range (must be 900–999)", info.priority));
    }

    // Validate handles
    if info.handles.is_empty() {
        errors.push("no request types in 'handles' — plugin won't handle any requests".to_string());
    }

    // Check Cargo.toml
    let cargo_path = "Cargo.toml";
    if std::path::Path::new(cargo_path).exists() {
        let cargo_data = std::fs::read_to_string(cargo_path)?;
        if !cargo_data.contains("cdylib") {
            errors.push("Cargo.toml missing crate-type = [\"cdylib\"] — required for WASM plugins".to_string());
        }
        if !cargo_data.contains("aspen-wasm-guest-sdk") {
            warnings.push("Cargo.toml doesn't depend on aspen-wasm-guest-sdk".to_string());
        }
    } else {
        errors.push("no Cargo.toml found".to_string());
    }

    // Check KV prefixes
    if info.kv_prefixes.is_empty() {
        warnings.push(format!("no kv_prefixes set — will default to '__plugin:{}:'", info.name));
    }

    // Print results
    println!("  Plugin: {}", info.name);
    println!("  Version: {}", info.version);
    println!("  Priority: {}", info.priority);
    println!("  Handles: {}", info.handles.join(", "));
    println!(
        "  KV Prefixes: {}",
        if info.kv_prefixes.is_empty() {
            format!("(default: __plugin:{}:)", info.name)
        } else {
            info.kv_prefixes.join(", ")
        }
    );
    println!("  Permissions:");
    println!("    kv_read: {}", info.permissions.kv_read);
    println!("    kv_write: {}", info.permissions.kv_write);
    println!("    blob_read: {}", info.permissions.blob_read);
    println!("    blob_write: {}", info.permissions.blob_write);
    println!("    cluster_info: {}", info.permissions.cluster_info);
    println!("    randomness: {}", info.permissions.randomness);
    println!("    signing: {}", info.permissions.signing);
    println!("    timers: {}", info.permissions.timers);
    println!("    hooks: {}", info.permissions.hooks);
    println!();

    for w in &warnings {
        println!("  ⚠ {w}");
    }
    for e in &errors {
        println!("  ✗ {e}");
    }

    if errors.is_empty() {
        println!("  ✓ Plugin configuration is valid");
        Ok(())
    } else {
        anyhow::bail!("{} error(s) found", errors.len())
    }
}

#[cfg(test)]
mod tests {
    use aspen_plugin_api::PluginInfo;
    use aspen_plugin_api::PluginPermissions;

    #[test]
    fn valid_manifest_parses() {
        let json = r#"{
            "name": "test-plugin",
            "version": "0.1.0",
            "handles": ["Ping"],
            "priority": 950,
            "permissions": { "kv_read": true }
        }"#;
        let info: PluginInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "test-plugin");
        assert_eq!(info.priority, 950);
        assert!(info.permissions.kv_read);
    }

    #[test]
    fn priority_validation() {
        assert!((900..=999).contains(&900));
        assert!((900..=999).contains(&999));
        assert!(!(900..=999).contains(&899));
        assert!(!(900..=999).contains(&1000));
    }

    #[test]
    fn default_permissions_all_false() {
        let perms = PluginPermissions::default();
        assert!(!perms.kv_read);
        assert!(!perms.kv_write);
        assert!(!perms.blob_read);
        assert!(!perms.blob_write);
        assert!(!perms.timers);
        assert!(!perms.hooks);
    }
}
