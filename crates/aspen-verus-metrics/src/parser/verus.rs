//! Parser for Verus specification files.
//!
//! Parses `verus/*.rs` files to extract exec function definitions from verus! macro blocks.
//! Uses a hybrid approach:
//! 1. `syn` to find verus! macro invocations in the AST
//! 2. State machine parser for the macro content (handles Verus-specific syntax)
//!
//! Handles ensures/requires clauses and supports recursive directory traversal.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use syn::ItemMacro;
use syn::visit::Visit;
use walkdir::WalkDir;

use crate::FunctionKind;
use crate::FunctionParam;
use crate::FunctionSignature;
use crate::ParsedFunction;

/// Parse all Verus files in a directory (non-recursive for backwards compatibility).
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

/// Parse all Verus files in a directory recursively.
///
/// Traverses subdirectories to find all .rs files containing verus! macros.
pub fn parse_verus_dir_recursive(dir: &Path) -> Result<Vec<ParsedFunction>> {
    let mut functions = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Only process .rs files
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }

        let file_functions = parse_file(path)?;
        functions.extend(file_functions);
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
        source: content,
    };

    visitor.visit_file(&syntax);

    Ok(visitor.functions)
}

struct VerusVisitor {
    functions: Vec<ParsedFunction>,
    file_path: PathBuf,
    source: String,
}

impl Visit<'_> for VerusVisitor {
    fn visit_item_macro(&mut self, item: &ItemMacro) {
        // Check if this is a verus! macro
        let path = &item.mac.path;
        if path.is_ident("verus") {
            // Parse the content of the verus! macro using state machine
            let tokens = &item.mac.tokens;
            let content = tokens.to_string();

            // Calculate line offset for this macro in the source
            let macro_line = self.find_macro_line_number(item);

            if let Ok(functions) = VerusBlockParser::new(&content, &self.file_path, macro_line).parse() {
                self.functions.extend(functions);
            }
        }

        syn::visit::visit_item_macro(self, item);
    }
}

impl VerusVisitor {
    fn find_macro_line_number(&self, _item: &ItemMacro) -> u32 {
        // Search for "verus!" in source to find line number
        if let Some(pos) = self.source.find("verus!") {
            let before = &self.source[..pos];
            return before.matches('\n').count() as u32 + 1;
        }
        1
    }
}

/// State machine parser for verus! macro block content.
///
/// Properly handles:
/// - Comments (line and block)
/// - String literals
/// - Nested braces/brackets/parens
/// - Verus-specific syntax (ensures, requires, exec fn, spec fn, proof fn)
struct VerusBlockParser<'a> {
    #[allow(dead_code)]
    content: &'a str, // Kept for potential future debug output
    chars: Vec<char>,
    pos: usize,
    file_path: &'a Path,
    base_line: u32,
}

/// Parser state for tracking context.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParserState {
    /// Normal parsing
    Normal,
    /// Inside a line comment
    LineComment,
    /// Inside a block comment
    BlockComment,
    /// Inside a string literal
    String,
    /// Inside a character literal
    Char,
}

impl<'a> VerusBlockParser<'a> {
    fn new(content: &'a str, file_path: &'a Path, base_line: u32) -> Self {
        Self {
            content,
            chars: content.chars().collect(),
            pos: 0,
            file_path,
            base_line,
        }
    }

    /// Parse the verus block and extract all exec functions.
    fn parse(&mut self) -> Result<Vec<ParsedFunction>> {
        let mut functions = Vec::new();

        while self.pos < self.chars.len() {
            // Skip whitespace and comments
            self.skip_trivia();

            if self.pos >= self.chars.len() {
                break;
            }

            // Look for function definitions
            if let Some(func) = self.try_parse_function()? {
                functions.push(func);
            } else {
                // Skip to next potential function start
                self.advance_to_next_item();
            }
        }

        Ok(functions)
    }

    /// Skip whitespace and comments.
    fn skip_trivia(&mut self) {
        let mut state = ParserState::Normal;

        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            let next = self.peek_char(1);

            match state {
                ParserState::Normal => {
                    if c.is_whitespace() {
                        self.pos += 1;
                    } else if c == '/' && next == Some('/') {
                        state = ParserState::LineComment;
                        self.pos += 2;
                    } else if c == '/' && next == Some('*') {
                        state = ParserState::BlockComment;
                        self.pos += 2;
                    } else {
                        return;
                    }
                }
                ParserState::LineComment => {
                    self.pos += 1;
                    if c == '\n' {
                        state = ParserState::Normal;
                    }
                }
                ParserState::BlockComment => {
                    self.pos += 1;
                    if c == '*' && next == Some('/') {
                        self.pos += 1;
                        state = ParserState::Normal;
                    }
                }
                _ => return,
            }
        }
    }

    /// Try to parse a function at the current position.
    fn try_parse_function(&mut self) -> Result<Option<ParsedFunction>> {
        let start_pos = self.pos;

        // Check for attributes (like #[verifier(external_body)])
        let mut skip_body = false;
        while self.match_char('#') {
            if self.match_char('[') {
                let attr = self.read_until_balanced(']');
                if attr.contains("external_body") {
                    skip_body = true;
                }
                self.skip_trivia();
            }
        }

        // Check for visibility
        if self.match_keyword("pub") {
            self.skip_trivia();
        }

        // Check for function kind
        let kind = if self.match_keyword("spec") {
            self.skip_trivia();
            if !self.match_keyword("fn") {
                self.pos = start_pos;
                return Ok(None);
            }
            FunctionKind::Spec
        } else if self.match_keyword("proof") {
            self.skip_trivia();
            if !self.match_keyword("fn") {
                self.pos = start_pos;
                return Ok(None);
            }
            FunctionKind::Proof
        } else if self.match_keyword("fn") {
            FunctionKind::Exec
        } else {
            self.pos = start_pos;
            return Ok(None);
        };

        // Skip spec and proof functions - we only want exec functions
        if !kind.is_executable() {
            // Skip to end of function
            self.skip_to_function_end();
            return Ok(None);
        }

        self.skip_trivia();

        // Parse function name
        let name = self.parse_identifier()?;
        if name.is_empty() {
            self.pos = start_pos;
            return Ok(None);
        }

        // Skip utility functions
        if is_utility_function(&name) {
            self.skip_to_function_end();
            return Ok(None);
        }

        self.skip_trivia();

        // Parse generics
        let generics = self.parse_generics()?;

        // Parse parameters
        if !self.match_char('(') {
            self.pos = start_pos;
            return Ok(None);
        }
        let params_str = self.read_until_balanced(')');
        let params = parse_params(&params_str);

        self.skip_trivia();

        // Parse return type
        let return_type = self.parse_return_type()?;

        self.skip_trivia();

        // Skip ensures/requires clauses
        self.skip_spec_clauses();

        // Parse body
        if !self.match_char('{') {
            self.pos = start_pos;
            return Ok(None);
        }
        let body = self.read_until_balanced('}');

        // Calculate line number
        let before_fn: String = self.chars[..start_pos].iter().collect();
        let line_number = self.base_line + before_fn.matches('\n').count() as u32;

        let signature = FunctionSignature {
            params,
            return_type,
            is_const: false,
            is_async: false,
            generics,
        };

        Ok(Some(ParsedFunction {
            name,
            file_path: self.file_path.to_path_buf(),
            line_number,
            signature,
            body: Some(body.trim().to_string()),
            raw_body: Some(body.clone()),
            kind,
            skip_body,
        }))
    }

    /// Parse an identifier.
    fn parse_identifier(&mut self) -> Result<String> {
        let mut ident = String::new();

        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }

        Ok(ident)
    }

    /// Parse generic parameters.
    fn parse_generics(&mut self) -> Result<Vec<String>> {
        if !self.match_char('<') {
            return Ok(Vec::new());
        }

        let generics_str = self.read_until_balanced('>');
        let generics = parse_generic_params(&generics_str);
        Ok(generics)
    }

    /// Parse the return type.
    fn parse_return_type(&mut self) -> Result<Option<String>> {
        if !self.match_str("->") {
            return Ok(None);
        }

        self.skip_trivia();

        // Verus uses (result: Type) syntax
        if self.match_char('(') {
            let inner = self.read_until_balanced(')');
            // Parse (result: Type) or just Type
            if let Some(colon_pos) = inner.find(':') {
                return Ok(Some(inner[colon_pos + 1..].trim().to_string()));
            } else {
                return Ok(Some(inner.trim().to_string()));
            }
        }

        // Regular return type
        let mut ty = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];

            if c == '{' || self.lookahead_keyword("ensures") || self.lookahead_keyword("requires") {
                break;
            }

            ty.push(c);
            self.pos += 1;
        }

        let ty = ty.trim();
        if ty.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ty.to_string()))
        }
    }

    /// Skip ensures/requires clauses.
    fn skip_spec_clauses(&mut self) {
        loop {
            self.skip_trivia();

            if self.lookahead_keyword("ensures") {
                self.match_keyword("ensures");
                self.skip_to_clause_end();
            } else if self.lookahead_keyword("requires") {
                self.match_keyword("requires");
                self.skip_to_clause_end();
            } else if self.lookahead_keyword("recommends") {
                self.match_keyword("recommends");
                self.skip_to_clause_end();
            } else if self.lookahead_keyword("decreases") {
                self.match_keyword("decreases");
                self.skip_to_clause_end();
            } else {
                break;
            }
        }
    }

    /// Skip to the end of a spec clause (until { or next clause keyword).
    fn skip_to_clause_end(&mut self) {
        let mut paren_depth = 0;

        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];

            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
            } else if paren_depth == 0 {
                if c == '{' {
                    return;
                }
                if c == ',' {
                    // Could be end of clause, check for next keyword
                    self.pos += 1;
                    self.skip_trivia();
                    if self.lookahead_keyword("ensures")
                        || self.lookahead_keyword("requires")
                        || self.lookahead_keyword("recommends")
                        || self.lookahead_keyword("decreases")
                    {
                        return;
                    }
                    continue;
                }
            }

            self.pos += 1;
        }
    }

    /// Skip to the end of a function body.
    fn skip_to_function_end(&mut self) {
        // Find opening brace
        while self.pos < self.chars.len() && self.chars[self.pos] != '{' {
            self.pos += 1;
        }

        if self.pos < self.chars.len() {
            self.pos += 1; // Skip '{'
            let _ = self.read_until_balanced('}');
        }
    }

    /// Advance to the next potential item.
    fn advance_to_next_item(&mut self) {
        // Always advance at least one character to prevent infinite loops
        if self.pos < self.chars.len() {
            self.pos += 1;
        }

        // Skip to next potential function start
        while self.pos < self.chars.len() {
            if self.lookahead_keyword("fn")
                || self.lookahead_keyword("pub")
                || self.lookahead_keyword("spec")
                || self.lookahead_keyword("proof")
            {
                return;
            }
            self.pos += 1;
        }
    }

    /// Read content until a balanced closing character.
    fn read_until_balanced(&mut self, close: char) -> String {
        let open = match close {
            ')' => '(',
            ']' => '[',
            '}' => '{',
            '>' => '<',
            _ => return String::new(),
        };

        let mut depth = 1;
        let mut result = String::new();
        let mut state = ParserState::Normal;

        while self.pos < self.chars.len() && depth > 0 {
            let c = self.chars[self.pos];
            let next = self.peek_char(1);

            match state {
                ParserState::Normal => {
                    if c == '/' && next == Some('/') {
                        state = ParserState::LineComment;
                        result.push(c);
                        self.pos += 1;
                        continue;
                    } else if c == '/' && next == Some('*') {
                        state = ParserState::BlockComment;
                        result.push(c);
                        self.pos += 1;
                        continue;
                    } else if c == '"' {
                        state = ParserState::String;
                    } else if c == '\'' {
                        state = ParserState::Char;
                    } else if c == open {
                        depth += 1;
                    } else if c == close {
                        depth -= 1;
                        if depth == 0 {
                            self.pos += 1;
                            return result;
                        }
                    }
                    result.push(c);
                }
                ParserState::LineComment => {
                    result.push(c);
                    if c == '\n' {
                        state = ParserState::Normal;
                    }
                }
                ParserState::BlockComment => {
                    result.push(c);
                    if c == '*' && next == Some('/') {
                        result.push('/');
                        self.pos += 2;
                        state = ParserState::Normal;
                        continue;
                    }
                }
                ParserState::String => {
                    result.push(c);
                    if c == '\\' && next.is_some() {
                        result.push(self.chars[self.pos + 1]);
                        self.pos += 2;
                        continue;
                    } else if c == '"' {
                        state = ParserState::Normal;
                    }
                }
                ParserState::Char => {
                    result.push(c);
                    if c == '\\' && next.is_some() {
                        result.push(self.chars[self.pos + 1]);
                        self.pos += 2;
                        continue;
                    } else if c == '\'' {
                        state = ParserState::Normal;
                    }
                }
            }

            self.pos += 1;
        }

        result
    }

    /// Match a single character.
    fn match_char(&mut self, c: char) -> bool {
        if self.pos < self.chars.len() && self.chars[self.pos] == c {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Match a string.
    fn match_str(&mut self, s: &str) -> bool {
        let s_chars: Vec<char> = s.chars().collect();
        if self.pos + s_chars.len() > self.chars.len() {
            return false;
        }

        for (i, c) in s_chars.iter().enumerate() {
            if self.chars[self.pos + i] != *c {
                return false;
            }
        }

        self.pos += s_chars.len();
        true
    }

    /// Match a keyword (must be followed by non-identifier char).
    fn match_keyword(&mut self, keyword: &str) -> bool {
        let kw_chars: Vec<char> = keyword.chars().collect();
        if self.pos + kw_chars.len() > self.chars.len() {
            return false;
        }

        for (i, c) in kw_chars.iter().enumerate() {
            if self.chars[self.pos + i] != *c {
                return false;
            }
        }

        // Check that it's not part of a larger identifier
        let next_idx = self.pos + kw_chars.len();
        if next_idx < self.chars.len() {
            let next = self.chars[next_idx];
            if next.is_alphanumeric() || next == '_' {
                return false;
            }
        }

        self.pos += kw_chars.len();
        true
    }

    /// Check if a keyword is ahead without consuming.
    fn lookahead_keyword(&self, keyword: &str) -> bool {
        let kw_chars: Vec<char> = keyword.chars().collect();
        if self.pos + kw_chars.len() > self.chars.len() {
            return false;
        }

        for (i, c) in kw_chars.iter().enumerate() {
            if self.chars[self.pos + i] != *c {
                return false;
            }
        }

        // Check that it's not part of a larger identifier
        let next_idx = self.pos + kw_chars.len();
        if next_idx < self.chars.len() {
            let next = self.chars[next_idx];
            if next.is_alphanumeric() || next == '_' {
                return false;
            }
        }

        true
    }

    /// Peek ahead by n characters.
    fn peek_char(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }
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

/// Parse generic parameters.
fn parse_generic_params(generics_str: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for c in generics_str.chars() {
        match c {
            '<' => {
                depth += 1;
                current.push(c);
            }
            '>' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    // Extract just the type parameter name (before any : bounds)
                    if let Some(colon_pos) = trimmed.find(':') {
                        params.push(trimmed[..colon_pos].trim().to_string());
                    } else {
                        params.push(trimmed.to_string());
                    }
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }

    // Handle last parameter
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        if let Some(colon_pos) = trimmed.find(':') {
            params.push(trimmed[..colon_pos].trim().to_string());
        } else {
            params.push(trimmed.to_string());
        }
    }

    params
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

        let mut parser = VerusBlockParser::new(content, Path::new("test.rs"), 1);
        let functions = parser.parse().unwrap();

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

        let mut parser = VerusBlockParser::new(content, Path::new("test.rs"), 1);
        let functions = parser.parse().unwrap();

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
    fn test_parse_generics() {
        let generics = parse_generic_params("T: Copy, U: Clone + Send");
        assert_eq!(generics.len(), 2);
        assert_eq!(generics[0], "T");
        assert_eq!(generics[1], "U");
    }

    #[test]
    fn test_parse_function_with_generics() {
        let content = r#"
            pub fn compute<T: Copy>(value: T) -> (result: T) {
                value
            }
        "#;

        let mut parser = VerusBlockParser::new(content, Path::new("test.rs"), 1);
        let functions = parser.parse().unwrap();

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "compute");
        assert_eq!(functions[0].signature.generics.len(), 1);
        assert_eq!(functions[0].signature.generics[0], "T");
    }

    #[test]
    fn test_handles_comments_in_body() {
        let content = r#"
            pub fn with_comments(x: u64) -> (result: u64) {
                // This is a comment
                let y = x + 1; // inline comment
                /* block comment */ y
            }
        "#;

        let mut parser = VerusBlockParser::new(content, Path::new("test.rs"), 1);
        let functions = parser.parse().unwrap();

        assert_eq!(functions.len(), 1);
        assert!(functions[0].body.as_ref().unwrap().contains("comment"));
    }

    #[test]
    fn test_handles_strings_with_braces() {
        let content = r#"
            pub fn with_string(x: u64) -> (result: u64) {
                let s = "{}}}}}";
                x
            }
        "#;

        let mut parser = VerusBlockParser::new(content, Path::new("test.rs"), 1);
        let functions = parser.parse().unwrap();

        assert_eq!(functions.len(), 1);
        assert!(functions[0].body.as_ref().unwrap().contains("}}}}"));
    }

    #[test]
    fn test_external_body_attribute() {
        let content = r#"
            #[verifier(external_body)]
            pub fn external_fn(x: u64) -> (result: u64) {
                x
            }
        "#;

        let mut parser = VerusBlockParser::new(content, Path::new("test.rs"), 1);
        let functions = parser.parse().unwrap();

        assert_eq!(functions.len(), 1);
        assert!(functions[0].skip_body);
    }
}
