//! `cargo aspen-plugin init` — scaffold a new plugin project.

use std::path::Path;

use crate::templates;
use crate::templates::TemplateContext;

pub fn run(name: &str, template: &str, description: &str, priority: u32, output: Option<&str>) -> anyhow::Result<()> {
    // Validate priority
    if !(900..=999).contains(&priority) {
        anyhow::bail!("priority must be in range 900–999, got {priority}");
    }

    // Validate name
    if name.is_empty() {
        anyhow::bail!("plugin name must not be empty");
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        anyhow::bail!("plugin name must contain only alphanumeric characters, hyphens, and underscores");
    }

    let ctx = TemplateContext {
        name,
        description,
        priority,
    };

    let rendered = templates::render(template, &ctx)?;

    // Determine output directory
    let out_dir = output.map(|s| s.to_string()).unwrap_or_else(|| name.to_string());
    let out_path = Path::new(&out_dir);

    if out_path.exists() {
        anyhow::bail!("directory '{}' already exists", out_path.display());
    }

    // Create directory structure
    std::fs::create_dir_all(out_path.join("src"))?;
    std::fs::create_dir_all(out_path.join(".cargo"))?;

    // Write files
    std::fs::write(out_path.join("Cargo.toml"), &rendered.cargo_toml)?;
    std::fs::write(out_path.join("plugin.json"), &rendered.plugin_json)?;
    std::fs::write(out_path.join("src/lib.rs"), &rendered.lib_rs)?;
    std::fs::write(out_path.join("README.md"), &rendered.readme)?;
    std::fs::write(out_path.join(".cargo/config.toml"), templates::cargo_config())?;
    std::fs::write(out_path.join(".gitignore"), "/target\n/Cargo.lock\n")?;

    println!("✓ Created plugin '{name}' at {}", out_path.display());
    println!("  Template: {template}");
    println!("  Priority: {priority}");
    println!();
    println!("  Next steps:");
    println!("    cd {}", out_path.display());
    println!("    cargo aspen-plugin check");
    println!("    cargo aspen-plugin build --release");

    Ok(())
}
