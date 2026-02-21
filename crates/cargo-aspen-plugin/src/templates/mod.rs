//! Template rendering for plugin scaffolding.

pub mod basic;
pub mod hook;
pub mod kv;
pub mod timer;

/// Available template names.
pub const TEMPLATES: &[&str] = &["basic", "kv", "timer", "hook"];

/// A rendered plugin template.
pub struct RenderedTemplate {
    pub cargo_toml: String,
    pub plugin_json: String,
    pub lib_rs: String,
    pub readme: String,
}

/// Context for template rendering.
pub struct TemplateContext<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub priority: u32,
}

impl TemplateContext<'_> {
    /// Convert kebab-case name to PascalCase for struct names.
    pub fn struct_name(&self) -> String {
        self.name
            .split('-')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                }
            })
            .collect()
    }

    /// Convert kebab-case name to snake_case for crate names.
    pub fn crate_name(&self) -> String {
        self.name.replace('-', "_")
    }
}

/// Render a template by name.
pub fn render(template: &str, ctx: &TemplateContext<'_>) -> anyhow::Result<RenderedTemplate> {
    match template {
        "basic" => Ok(basic::render(ctx)),
        "kv" => Ok(kv::render(ctx)),
        "timer" => Ok(timer::render(ctx)),
        "hook" => Ok(hook::render(ctx)),
        _ => anyhow::bail!("unknown template '{}'. Available: {}", template, TEMPLATES.join(", ")),
    }
}

/// Shared Cargo.toml template.
pub fn cargo_toml(ctx: &TemplateContext<'_>) -> String {
    format!(
        r#"[package]
name = "aspen-{name}"
version = "0.1.0"
edition = "2024"
description = "{description}"
license = "AGPL-3.0-or-later"

[lib]
crate-type = ["cdylib"]

[dependencies]
aspen-wasm-guest-sdk = {{ git = "https://github.com/aspen-cloud/aspen", branch = "main" }}
serde_json = "1.0"
"#,
        name = ctx.name,
        description = ctx.description,
    )
}

/// Shared .cargo/config.toml for wasm32 default target.
pub fn cargo_config() -> &'static str {
    r#"[build]
target = "wasm32-unknown-unknown"
"#
}

/// Shared README template.
pub fn readme(ctx: &TemplateContext<'_>) -> String {
    format!(
        r#"# {name}

{description}

## Building

```bash
cargo aspen-plugin build --release
```

## Installing

```bash
aspen-cli plugin install \
  target/wasm32-unknown-unknown/release/{crate_name}.wasm \
  --manifest plugin.json
```

## Development

```bash
# Validate configuration
cargo aspen-plugin check

# Build debug
cargo aspen-plugin build

# Sign for distribution
cargo aspen-plugin sign --key ~/.config/aspen/plugin-signing-key
```
"#,
        name = ctx.name,
        description = ctx.description,
        crate_name = ctx.crate_name(),
    )
}
