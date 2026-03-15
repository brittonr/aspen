//! Template rendering.
//!
//! Replaces placeholder strings in templates with decrypted secret values.

use std::collections::HashMap;
use std::fs;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

use crate::manifest::TemplateEntry;

/// Load a template's source text from either `content` or `file`.
pub fn load_template_source(template: &TemplateEntry) -> Result<String> {
    if !template.content.is_empty() {
        return Ok(template.content.clone());
    }

    if !template.file.is_empty() {
        return fs::read_to_string(&template.file)
            .with_context(|| format!("cannot read template file '{}'", template.file));
    }

    bail!("neither content nor file was specified for template '{}'", template.name);
}

/// Render a template by replacing placeholders with secret values.
pub fn render_template(
    source: &str,
    placeholder_map: &HashMap<String, String>,
    secrets: &HashMap<String, Vec<u8>>,
) -> String {
    let mut rendered = source.to_string();

    for (placeholder, secret) in placeholder_map {
        if let Some(value) = secrets.get(secret.as_str()) {
            let value_str = String::from_utf8_lossy(value);
            rendered = rendered.replace(placeholder, &value_str);
        }
    }

    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_single_placeholder() {
        let mut placeholders = HashMap::new();
        placeholders.insert("<<PASS>>".to_string(), "db-password".to_string());

        let mut secrets = HashMap::new();
        secrets.insert("db-password".to_string(), b"hunter2".to_vec());

        let result = render_template("password=<<PASS>>", &placeholders, &secrets);
        assert_eq!(result, "password=hunter2");
    }

    #[test]
    fn render_multiple_placeholders() {
        let mut placeholders = HashMap::new();
        placeholders.insert("<<HOST>>".to_string(), "db-host".to_string());
        placeholders.insert("<<PASS>>".to_string(), "db-pass".to_string());

        let mut secrets = HashMap::new();
        secrets.insert("db-host".to_string(), b"localhost".to_vec());
        secrets.insert("db-pass".to_string(), b"secret".to_vec());

        let result = render_template("host=<<HOST>>\npass=<<PASS>>", &placeholders, &secrets);
        assert_eq!(result, "host=localhost\npass=secret");
    }

    #[test]
    fn render_missing_secret_leaves_placeholder() {
        let mut placeholders = HashMap::new();
        placeholders.insert("<<PASS>>".to_string(), "missing".to_string());

        let secrets = HashMap::new();

        let result = render_template("password=<<PASS>>", &placeholders, &secrets);
        assert_eq!(result, "password=<<PASS>>");
    }

    #[test]
    fn load_template_from_content() {
        let template = TemplateEntry {
            group: None,
            name: "test".to_string(),
            content: "hello <<NAME>>".to_string(),
            file: String::new(),
            path: "/tmp/test".to_string(),
            mode: "0440".to_string(),
            owner: None,
            uid: 0,
            gid: 0,
            restart_units: vec![],
            reload_units: vec![],
        };

        assert_eq!(load_template_source(&template).unwrap(), "hello <<NAME>>");
    }

    #[test]
    fn load_template_no_source_fails() {
        let template = TemplateEntry {
            group: None,
            name: "test".to_string(),
            content: String::new(),
            file: String::new(),
            path: "/tmp/test".to_string(),
            mode: "0440".to_string(),
            owner: None,
            uid: 0,
            gid: 0,
            restart_units: vec![],
            reload_units: vec![],
        };

        assert!(load_template_source(&template).is_err());
    }
}
