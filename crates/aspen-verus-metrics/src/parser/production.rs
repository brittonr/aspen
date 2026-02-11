//! Parser for production verified functions.
//!
//! Parses `src/verified/*.rs` files to extract public function definitions.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use syn::ItemFn;
use syn::Visibility;
use syn::visit::Visit;

use super::extract_body_string;
use super::extract_signature;
use crate::FunctionKind;
use crate::ParsedFunction;

/// Parse all verified files in a directory.
pub fn parse_verified_dir(dir: &Path) -> Result<Vec<ParsedFunction>> {
    let mut functions = Vec::new();

    for entry in fs::read_dir(dir).context("Failed to read verified directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "rs") {
            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // Skip mod.rs
            if filename == "mod" {
                continue;
            }

            let file_functions = parse_file(&path)?;
            functions.extend(file_functions);
        }
    }

    Ok(functions)
}

/// Parse a single production Rust file.
pub fn parse_file(path: &Path) -> Result<Vec<ParsedFunction>> {
    let content = fs::read_to_string(path).context("Failed to read file")?;

    let syntax = syn::parse_file(&content).context("Failed to parse Rust file")?;

    let mut visitor = ProductionVisitor {
        functions: Vec::new(),
        file_path: path.to_path_buf(),
        source: &content,
    };

    visitor.visit_file(&syntax);

    Ok(visitor.functions)
}

struct ProductionVisitor<'a> {
    functions: Vec<ParsedFunction>,
    file_path: PathBuf,
    source: &'a str,
}

impl<'a> Visit<'_> for ProductionVisitor<'a> {
    fn visit_item_fn(&mut self, item: &ItemFn) {
        // Only process public functions
        if !matches!(item.vis, Visibility::Public(_)) {
            return;
        }

        let name = item.sig.ident.to_string();

        // Skip common utility functions
        if is_utility_function(&name) {
            return;
        }

        let signature = extract_signature(&item.sig);
        let body = extract_body_string(&item.block);

        // Calculate line number from span
        let line_number = self.calculate_line_number(item);

        self.functions.push(ParsedFunction {
            name,
            file_path: self.file_path.clone(),
            line_number,
            signature,
            body: Some(body.clone()),
            raw_body: Some(body),
            kind: FunctionKind::Regular,
            skip_body: false,
        });

        // Continue visiting nested items
        syn::visit::visit_item_fn(self, item);
    }
}

impl<'a> ProductionVisitor<'a> {
    fn calculate_line_number(&self, item: &ItemFn) -> u32 {
        // Search for function name in source
        let fn_name = &item.sig.ident.to_string();
        let pattern = format!("fn {}", fn_name);

        self.source
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains(&pattern))
            .map(|(i, _)| i as u32 + 1)
            .unwrap_or(1)
    }
}

/// Check if a function name is a common utility that shouldn't be compared.
fn is_utility_function(name: &str) -> bool {
    matches!(name, "new" | "default" | "from" | "into" | "clone" | "eq" | "hash" | "fmt" | "drop")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = r#"
            pub fn is_lock_expired(deadline_ms: u64, now_ms: u64) -> bool {
                deadline_ms == 0 || now_ms > deadline_ms
            }
        "#;

        let syntax: syn::File = syn::parse_str(source).unwrap();
        let mut visitor = ProductionVisitor {
            functions: Vec::new(),
            file_path: PathBuf::from("test.rs"),
            source,
        };
        visitor.visit_file(&syntax);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "is_lock_expired");
        assert_eq!(visitor.functions[0].signature.params.len(), 2);
    }

    #[test]
    fn test_skip_private_functions() {
        let source = r#"
            fn private_fn() {}
            pub fn public_fn() {}
        "#;

        let syntax: syn::File = syn::parse_str(source).unwrap();
        let mut visitor = ProductionVisitor {
            functions: Vec::new(),
            file_path: PathBuf::from("test.rs"),
            source,
        };
        visitor.visit_file(&syntax);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "public_fn");
    }

    #[test]
    fn test_skip_utility_functions() {
        let source = r#"
            pub fn new() -> Self {}
            pub fn important_function() {}
        "#;

        let syntax: syn::File = syn::parse_str(source).unwrap();
        let mut visitor = ProductionVisitor {
            functions: Vec::new(),
            file_path: PathBuf::from("test.rs"),
            source,
        };
        visitor.visit_file(&syntax);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "important_function");
    }
}
