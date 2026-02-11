//! Parser for Verus specification files.
//!
//! Parses `verus/*.rs` files to extract exec function definitions from verus! macro blocks.
//! Handles the Verus-specific syntax including ensures/requires clauses.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use syn::ItemMacro;
use syn::visit::Visit;

use crate::FunctionKind;
use crate::FunctionParam;
use crate::FunctionSignature;
use crate::ParsedFunction;

/// Parse all Verus files in a directory.
pub fn parse_verus_dir(dir: &Path) -> Result<Vec<ParsedFunction>> {
    let mut functions = Vec::new();

    for entry in fs::read_dir(dir).context("Failed to read verus directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "rs") {
            let file_functions = parse_file(&path)?;
            functions.extend(file_functions);
        }
    }

    Ok(functions)
}

/// Parse a single Verus file.
pub fn parse_file(path: &Path) -> Result<Vec<ParsedFunction>> {
    let content = fs::read_to_string(path).context("Failed to read file")?;

    let syntax = syn::parse_file(&content).context("Failed to parse Verus file")?;

    let mut visitor = VerusVisitor {
        functions: Vec::new(),
        file_path: path.to_path_buf(),
    };

    visitor.visit_file(&syntax);

    Ok(visitor.functions)
}

struct VerusVisitor {
    functions: Vec<ParsedFunction>,
    file_path: PathBuf,
}

impl Visit<'_> for VerusVisitor {
    fn visit_item_macro(&mut self, item: &ItemMacro) {
        // Check if this is a verus! macro
        let path = &item.mac.path;
        if path.is_ident("verus") {
            // Parse the content of the verus! macro
            if let Ok(functions) = self.parse_verus_macro_content(item) {
                self.functions.extend(functions);
            }
        }

        syn::visit::visit_item_macro(self, item);
    }
}

impl VerusVisitor {
    fn parse_verus_macro_content(&self, item: &ItemMacro) -> Result<Vec<ParsedFunction>> {
        let tokens = &item.mac.tokens;
        let content = tokens.to_string();

        // Parse the verus block content
        let mut functions = Vec::new();

        // We need to parse the verus-specific function syntax
        // verus functions look like:
        // pub fn name(args) -> (result: Type) ensures ... { body }
        // pub fn name(args) requires ... ensures ... { body }

        // Use a custom parser for verus function syntax
        let parsed_fns = parse_verus_functions(&content, &self.file_path)?;
        functions.extend(parsed_fns);

        Ok(functions)
    }
}

/// Parse verus functions from macro content.
fn parse_verus_functions(content: &str, file_path: &Path) -> Result<Vec<ParsedFunction>> {
    let mut functions = Vec::new();

    // Find function definitions in the verus block
    // We use a regex-like approach to find function boundaries, then parse each
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        // Look for "pub fn " or "fn " patterns (excluding "spec fn" and "proof fn")
        if let Some(fn_start) = find_exec_fn_start(&chars, pos) {
            // Check if this is spec or proof fn (should skip)
            let prefix_start = fn_start.saturating_sub(10);
            let prefix: String = chars[prefix_start..fn_start].iter().collect();

            if prefix.contains("spec") || prefix.contains("proof") {
                pos = fn_start + 1;
                continue;
            }

            // Parse the function
            if let Some((func, end_pos)) = parse_single_verus_function(&chars, fn_start, file_path) {
                functions.push(func);
                pos = end_pos;
            } else {
                pos = fn_start + 1;
            }
        } else {
            break;
        }
    }

    Ok(functions)
}

/// Find the start of an exec fn definition.
fn find_exec_fn_start(chars: &[char], start: usize) -> Option<usize> {
    let s: String = chars[start..].iter().collect();

    // Find "pub fn " or standalone "fn " (but not "spec fn" or "proof fn")
    let patterns = ["pub fn ", "fn "];

    for pattern in patterns {
        if let Some(idx) = s.find(pattern) {
            let abs_pos = start + idx;

            // Check if preceded by "spec " or "proof "
            let check_start = abs_pos.saturating_sub(10);
            let prefix: String = chars[check_start..abs_pos].iter().collect();

            if !prefix.contains("spec") && !prefix.contains("proof") {
                return Some(abs_pos);
            }
        }
    }

    None
}

/// Parse a single verus function starting at the given position.
fn parse_single_verus_function(chars: &[char], start: usize, file_path: &Path) -> Option<(ParsedFunction, usize)> {
    let remaining: String = chars[start..].iter().collect();

    // Extract function name
    let name_start = remaining.find("fn ")? + 3;
    let name_end = remaining[name_start..].find('(')?;
    let name = remaining[name_start..name_start + name_end].trim().to_string();

    // Skip utility functions
    if is_utility_function(&name) {
        return None;
    }

    // Find the opening brace of the function body
    // This is after any ensures/requires clauses
    let mut brace_depth;
    let mut body_start = 0;
    let mut body_end = 0;

    // Find parameter list end
    let mut paren_depth = 0;
    let mut params_end = 0;
    let mut in_params = false;

    for (i, c) in remaining.chars().enumerate() {
        if c == '(' {
            if !in_params {
                in_params = true;
            }
            paren_depth += 1;
        } else if c == ')' {
            paren_depth -= 1;
            if paren_depth == 0 && in_params {
                params_end = i;
                break;
            }
        }
    }

    // Extract parameters
    let params_str = &remaining[name_start + name_end + 1..params_end];
    let params = parse_params(params_str);

    // Find return type and skip ensures/requires
    let after_params = &remaining[params_end + 1..];

    // Look for the function body opening brace
    // Skip over: -> (result: Type) ensures ... requires ...
    let mut skip_pos = 0;
    let mut found_body_start = false;

    for (byte_idx, c) in after_params.char_indices() {
        if c == '{' {
            // Check if this is the body start (not inside ensures expression)
            let before = &after_params[..byte_idx];

            // Count unmatched parens/braces to ensure we're at the right level
            let open_parens = before.matches('(').count();
            let close_parens = before.matches(')').count();

            if open_parens == close_parens {
                body_start = start + params_end + 1 + byte_idx + 1;
                found_body_start = true;
                skip_pos = byte_idx + 1;
                break;
            }
        }
    }

    if !found_body_start {
        return None;
    }

    // Find matching closing brace
    brace_depth = 1;
    for (byte_idx, c) in after_params[skip_pos..].char_indices() {
        if c == '{' {
            brace_depth += 1;
        } else if c == '}' {
            brace_depth -= 1;
            if brace_depth == 0 {
                body_end = start + params_end + 1 + skip_pos + byte_idx;
                break;
            }
        }
    }

    if body_end == 0 {
        return None;
    }

    // Extract body
    let body: String = chars[body_start..body_end].iter().collect();
    let body = body.trim().to_string();

    // Check for #[verifier(external_body)] attribute
    let prefix: String = chars[start.saturating_sub(50)..start].iter().collect();
    let skip_body = prefix.contains("external_body");

    // Parse return type
    let return_type = parse_verus_return_type(after_params);

    // Calculate line number
    let before_fn: String = chars[..start].iter().collect();
    let line_number = before_fn.matches('\n').count() as u32 + 1;

    let signature = FunctionSignature {
        params,
        return_type,
        is_const: remaining.starts_with("const "),
        is_async: false,      // Verus doesn't have async
        generics: Vec::new(), // TODO: Parse generics
    };

    let func = ParsedFunction {
        name,
        file_path: file_path.to_path_buf(),
        line_number,
        signature,
        body: Some(body.clone()),
        raw_body: Some(body),
        kind: FunctionKind::Exec,
        skip_body,
    };

    Some((func, body_end + 1))
}

/// Parse function parameters from a string.
fn parse_params(params_str: &str) -> Vec<FunctionParam> {
    let mut params = Vec::new();

    // Split by comma, but respect nested types
    let mut current = String::new();
    let mut depth = 0;

    for c in params_str.chars() {
        match c {
            '<' | '(' | '[' => {
                depth += 1;
                current.push(c);
            }
            '>' | ')' | ']' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                if let Some(param) = parse_single_param(&current) {
                    params.push(param);
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }

    // Don't forget the last parameter
    if let Some(param) = parse_single_param(&current) {
        params.push(param);
    }

    params
}

/// Parse a single parameter.
fn parse_single_param(param: &str) -> Option<FunctionParam> {
    let param = param.trim();
    if param.is_empty() {
        return None;
    }

    // Format: name: Type
    let parts: Vec<&str> = param.splitn(2, ':').collect();
    if parts.len() == 2 {
        Some(FunctionParam {
            name: parts[0].trim().to_string(),
            ty: parts[1].trim().to_string(),
        })
    } else {
        None
    }
}

/// Parse Verus return type (handles -> (result: Type) syntax).
fn parse_verus_return_type(after_params: &str) -> Option<String> {
    let trimmed = after_params.trim();

    if !trimmed.starts_with("->") {
        return None;
    }

    let after_arrow = trimmed[2..].trim();

    // Verus uses (result: Type) syntax
    if let Some(stripped) = after_arrow.strip_prefix('(') {
        // Find matching paren
        let mut depth = 1;
        let mut end_byte = 0;

        for (byte_idx, c) in stripped.char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    end_byte = byte_idx;
                    break;
                }
            }
        }

        let inner = &stripped[..end_byte];

        // Parse (result: Type) or just Type
        if let Some(colon_pos) = inner.find(':') {
            Some(inner[colon_pos + 1..].trim().to_string())
        } else {
            Some(inner.trim().to_string())
        }
    } else {
        // Regular return type until ensures/requires/{
        let end_markers = ["ensures", "requires", "{"];
        let mut end = after_arrow.len();

        for marker in end_markers {
            if let Some(pos) = after_arrow.find(marker) {
                end = end.min(pos);
            }
        }

        let ty = after_arrow[..end].trim();
        if ty.is_empty() { None } else { Some(ty.to_string()) }
    }
}

/// Check if a function name is a common utility.
fn is_utility_function(name: &str) -> bool {
    matches!(name, "new" | "default" | "from" | "into" | "clone" | "eq" | "hash" | "fmt" | "drop")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verus_function() {
        let content = r#"
            pub fn is_lock_expired(deadline_ms: u64, now_ms: u64) -> (result: bool)
                ensures result == (deadline_ms == 0 || now_ms > deadline_ms)
            {
                deadline_ms == 0 || now_ms > deadline_ms
            }
        "#;

        let functions = parse_verus_functions(content, Path::new("test.rs")).unwrap();

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "is_lock_expired");
        assert_eq!(functions[0].kind, FunctionKind::Exec);
    }

    #[test]
    fn test_skip_spec_functions() {
        let content = r#"
            pub spec fn is_expired(entry: LockEntrySpec) -> bool {
                entry.deadline_ms == 0
            }

            pub fn is_lock_expired(deadline_ms: u64) -> (result: bool) {
                deadline_ms == 0
            }
        "#;

        let functions = parse_verus_functions(content, Path::new("test.rs")).unwrap();

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "is_lock_expired");
    }

    #[test]
    fn test_parse_params() {
        let params = parse_params("deadline_ms: u64, now_ms: u64");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "deadline_ms");
        assert_eq!(params[0].ty, "u64");
    }

    #[test]
    fn test_parse_complex_params() {
        let params = parse_params("entry: Option<&LockEntry>, current_time: u64");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "entry");
        assert_eq!(params[0].ty, "Option<&LockEntry>");
    }

    #[test]
    fn test_parse_verus_return_type() {
        assert_eq!(parse_verus_return_type("-> (result: bool)"), Some("bool".to_string()));

        assert_eq!(parse_verus_return_type("-> (result: u64) ensures result >= 1"), Some("u64".to_string()));

        assert_eq!(parse_verus_return_type("-> bool {"), Some("bool".to_string()));
    }
}
