//! Configuration and remote management handlers.
//!
//! These are local-only operations that don't require cluster connection.

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

use super::output::PijulConfigOutput;
use super::output::PijulRemoteOutput;
use crate::output::print_output;
use crate::output::print_success;

// =============================================================================
// Config Handlers
// =============================================================================

/// Get the global config path.
fn global_config_path() -> Result<PathBuf> {
    // Try XDG_CONFIG_HOME first
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(config_home).join("aspen-pijul").join("config.toml"));
    }

    // Fall back to $HOME/.config
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("could not determine home directory")?;

    Ok(PathBuf::from(home).join(".config").join("aspen-pijul").join("config.toml"))
}

/// Get the local config path (current working directory).
pub(super) fn local_config_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    Ok(cwd.join(".aspen").join("pijul").join("config.toml"))
}

/// Load configuration from TOML file.
pub(super) fn load_config(path: &PathBuf) -> Result<toml::Table> {
    if !path.exists() {
        return Ok(toml::Table::new());
    }

    let content =
        std::fs::read_to_string(path).with_context(|| format!("failed to read config file: {}", path.display()))?;

    content.parse().with_context(|| format!("failed to parse config file: {}", path.display()))
}

/// Save configuration to TOML file.
pub(super) fn save_config(path: &PathBuf, table: &toml::Table) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(table).context("failed to serialize config")?;

    std::fs::write(path, content).with_context(|| format!("failed to write config file: {}", path.display()))
}

/// Get a value from config using dot notation (e.g., "user.name").
fn get_value(table: &toml::Table, key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current: &toml::Value = &toml::Value::Table(table.clone());

    for part in parts {
        match current {
            toml::Value::Table(t) => {
                current = t.get(part)?;
            }
            _ => return None,
        }
    }

    match current {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        _ => Some(current.to_string()),
    }
}

/// Set a value in config using dot notation.
///
/// If an intermediate path component exists but is not a table, it will be overwritten
/// with a new table. This matches the behavior of tools like `git config`.
fn set_value(table: &mut toml::Table, key: &str, value: &str) {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 1 {
        table.insert(parts[0].to_string(), toml::Value::String(value.to_string()));
        return;
    }

    // Navigate/create nested tables
    let mut current = table;
    for part in &parts[..parts.len() - 1] {
        // or_insert_with guarantees we have a Value here, and we ensure it's a Table
        // by using or_insert_with to create one if the entry doesn't exist.
        // If the entry exists but isn't a table, overwrite it with a new table.
        let entry = current.entry(part.to_string()).or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if !entry.is_table() {
            *entry = toml::Value::Table(toml::Table::new());
        }
        // SAFETY: We just ensured this is a table above - the condition guarantees
        // entry.is_table() == true at this point, so as_table_mut() cannot return None.
        current = entry.as_table_mut().expect("set_value: entry is guaranteed to be a table by preceding logic");
    }

    current.insert(parts[parts.len() - 1].to_string(), toml::Value::String(value.to_string()));
}

/// Unset a value in config using dot notation.
fn unset_value(table: &mut toml::Table, key: &str) -> bool {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 1 {
        return table.remove(parts[0]).is_some();
    }

    // Navigate to parent table
    let mut current = table;
    for part in &parts[..parts.len() - 1] {
        match current.get_mut(*part) {
            Some(toml::Value::Table(t)) => current = t,
            _ => return false,
        }
    }

    current.remove(parts[parts.len() - 1]).is_some()
}

/// Flatten config table to key-value pairs.
fn flatten_config(table: &toml::Table, prefix: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();

    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };

        match value {
            toml::Value::Table(t) => {
                result.extend(flatten_config(t, &full_key));
            }
            toml::Value::String(s) => {
                result.push((full_key, s.clone()));
            }
            _ => {
                result.push((full_key, value.to_string()));
            }
        }
    }

    result
}

pub(super) fn config_get(key: &str, global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let table = load_config(&path)?;
    let value = get_value(&table, key);

    let output = PijulConfigOutput {
        key: Some(key.to_string()),
        value,
        entries: Vec::new(),
        global,
    };
    print_output(&output, json);
    Ok(())
}

pub(super) fn config_set(key: &str, value: &str, global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let mut table = load_config(&path)?;
    set_value(&mut table, key, value);
    save_config(&path, &table)?;

    let output = PijulConfigOutput {
        key: Some(key.to_string()),
        value: Some(value.to_string()),
        entries: Vec::new(),
        global,
    };
    print_output(&output, json);
    print_success(&format!("Set {} = {}", key, value), json);
    Ok(())
}

pub(super) fn config_list(global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let table = load_config(&path)?;
    let entries = flatten_config(&table, "");

    let output = PijulConfigOutput {
        key: None,
        value: None,
        entries,
        global,
    };
    print_output(&output, json);
    Ok(())
}

pub(super) fn config_unset(key: &str, global: bool, json: bool) -> Result<()> {
    let path = if global {
        global_config_path()?
    } else {
        local_config_path()?
    };

    let mut table = load_config(&path)?;
    let removed = unset_value(&mut table, key);
    save_config(&path, &table)?;

    if removed {
        print_success(&format!("Unset {}", key), json);
    } else {
        print_success(&format!("Key '{}' was not set", key), json);
    }

    let output = PijulConfigOutput {
        key: Some(key.to_string()),
        value: None,
        entries: Vec::new(),
        global,
    };
    if json {
        print_output(&output, json);
    }
    Ok(())
}

// =============================================================================
// Remote Handlers
// =============================================================================

/// Load remotes from the local config file.
fn load_remotes() -> Result<std::collections::HashMap<String, String>> {
    let path = local_config_path()?;
    let table = load_config(&path)?;

    let mut remotes = std::collections::HashMap::new();
    if let Some(toml::Value::Table(remotes_table)) = table.get("remotes") {
        for (name, value) in remotes_table {
            if let toml::Value::String(url) = value {
                remotes.insert(name.clone(), url.clone());
            }
        }
    }
    Ok(remotes)
}

/// Save remotes to the local config file.
fn save_remotes(remotes: &std::collections::HashMap<String, String>) -> Result<()> {
    let path = local_config_path()?;
    let mut table = load_config(&path)?;

    // Build remotes table
    let mut remotes_table = toml::Table::new();
    for (name, url) in remotes {
        remotes_table.insert(name.clone(), toml::Value::String(url.clone()));
    }
    table.insert("remotes".to_string(), toml::Value::Table(remotes_table));

    save_config(&path, &table)
}

pub(super) fn remote_list(json: bool) -> Result<()> {
    let remotes = load_remotes()?;

    let output = PijulRemoteOutput {
        name: None,
        url: None,
        remotes: remotes.into_iter().collect(),
    };
    print_output(&output, json);
    Ok(())
}

pub(super) fn remote_add(name: &str, url: &str, json: bool) -> Result<()> {
    let mut remotes = load_remotes()?;

    if remotes.contains_key(name) {
        anyhow::bail!("Remote '{}' already exists. Use 'remote remove' first.", name);
    }

    remotes.insert(name.to_string(), url.to_string());
    save_remotes(&remotes)?;

    let output = PijulRemoteOutput {
        name: Some(name.to_string()),
        url: Some(url.to_string()),
        remotes: Vec::new(),
    };
    print_output(&output, json);
    print_success(&format!("Added remote '{}' -> {}", name, url), json);
    Ok(())
}

pub(super) fn remote_remove(name: &str, json: bool) -> Result<()> {
    let mut remotes = load_remotes()?;

    if remotes.remove(name).is_none() {
        anyhow::bail!("Remote '{}' not found", name);
    }

    save_remotes(&remotes)?;

    print_success(&format!("Removed remote '{}'", name), json);
    Ok(())
}

pub(super) fn remote_show(name: &str, json: bool) -> Result<()> {
    let remotes = load_remotes()?;

    let output = if let Some(url) = remotes.get(name) {
        PijulRemoteOutput {
            name: Some(name.to_string()),
            url: Some(url.clone()),
            remotes: Vec::new(),
        }
    } else {
        PijulRemoteOutput {
            name: Some(name.to_string()),
            url: None,
            remotes: Vec::new(),
        }
    };
    print_output(&output, json);
    Ok(())
}
