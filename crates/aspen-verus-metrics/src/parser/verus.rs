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

/// Control flow action returned by `read_until_balanced_normal`.
enum BalancedAction {
    /// The balanced close was found; return the accumulated result.
    Finished(String),
    /// A state transition that already advanced `pos` (comment starts); caller
    /// should `continue` to skip the normal `pos += 1`.
    SkipAndTransition(ParserState),
    /// A state transition (or staying in Normal) where `pos` should be
    /// incremented normally by the loop.
    Transition(ParserState),
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

        let skip_body = self.try_parse_function_attributes();

        let kind = match self.try_parse_function_kind(start_pos) {
            Some(k) => k,
            None => return Ok(None),
        };

        // Skip spec and proof functions - we only want exec functions
        if !kind.is_executable() {
            self.skip_to_function_end();
            return Ok(None);
        }

        self.skip_trivia();

        let (name, generics, params) = match self.try_parse_function_signature(start_pos)? {
            Some(sig) => sig,
            None => return Ok(None),
        };

        self.try_parse_function_body(start_pos, name, generics, params, kind, skip_body)
    }

    /// Parse function attributes and return whether the body should be skipped.
    fn try_parse_function_attributes(&mut self) -> bool {
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
        skip_body
    }

    /// Parse visibility and function kind (spec/proof/exec).
    ///
    /// Returns `None` if no function keyword is found, resetting position.
    fn try_parse_function_kind(&mut self, start_pos: usize) -> Option<FunctionKind> {
        if self.match_keyword("pub") {
            self.skip_trivia();
        }

        if self.match_keyword("spec") {
            self.skip_trivia();
            if !self.match_keyword("fn") {
                self.pos = start_pos;
                return None;
            }
            Some(FunctionKind::Spec)
        } else if self.match_keyword("proof") {
            self.skip_trivia();
            if !self.match_keyword("fn") {
                self.pos = start_pos;
                return None;
            }
            Some(FunctionKind::Proof)
        } else if self.match_keyword("fn") {
            Some(FunctionKind::Exec)
        } else {
            self.pos = start_pos;
            None
        }
    }

    /// Parse function name, generics, and parameters.
    ///
    /// Returns `None` if parsing fails, resetting position.
    fn try_parse_function_signature(
        &mut self,
        start_pos: usize,
    ) -> Result<Option<(String, Vec<String>, Vec<FunctionParam>)>> {
        let name = self.parse_identifier()?;
        if name.is_empty() {
            self.pos = start_pos;
            return Ok(None);
        }

        if is_utility_function(&name) {
            self.skip_to_function_end();
            return Ok(None);
        }

        self.skip_trivia();
        let generics = self.parse_generics()?;

        if !self.match_char('(') {
            self.pos = start_pos;
            return Ok(None);
        }
        let params_str = self.read_until_balanced(')');
        let params = parse_params(&params_str);
        self.skip_trivia();

        Ok(Some((name, generics, params)))
    }

    /// Parse return type, spec clauses, body, and construct the `ParsedFunction`.
    fn try_parse_function_body(
        &mut self,
        start_pos: usize,
        name: String,
        generics: Vec<String>,
        params: Vec<FunctionParam>,
        kind: FunctionKind,
        skip_body: bool,
    ) -> Result<Option<ParsedFunction>> {
        let return_type = self.parse_return_type()?;
        self.skip_trivia();
        self.skip_spec_clauses();

        if !self.match_char('{') {
            self.pos = start_pos;
            return Ok(None);
        }
        let body = self.read_until_balanced('}');

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

    /// Skip to the end of a spec clause (until the function body `{`).
    ///
    /// Handles nested braces in ensures/requires expressions (e.g., `match` expressions).
    /// Strategy: any `{` at brace_depth 0 is tracked. When we see `}` that closes it,
    /// we continue. The function body `{` is the first `{` at depth 0 that ISN'T
    /// inside a nested brace group from the clause.
    fn skip_to_clause_end(&mut self) {
        let mut paren_depth: i32 = 0;
        let mut brace_depth: i32 = 0;

        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];

            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
            } else if c == '{' {
                if brace_depth == 0 && paren_depth == 0 {
                    // Is this an expression brace (match/if/etc) or the function body?
                    // Look at the last non-whitespace token before this `{`
                    let mut look = self.pos.wrapping_sub(1);
                    while look < self.chars.len() && self.chars[look].is_whitespace() {
                        look = look.wrapping_sub(1);
                    }
                    // Check if the preceding context looks like a match/if/else
                    let is_expression_brace = if look < self.chars.len() {
                        // Look back up to 40 chars to find match/if/else/loop
                        // This handles patterns like `match some_long_expr {`
                        let start = look.saturating_sub(40);
                        let context: String = self.chars[start..=look].iter().collect();
                        // Check if there's a match/if/else keyword in this context
                        // that would make this `{` an expression brace
                        let words: Vec<&str> = context.split_whitespace().collect();
                        words.iter().any(|w| matches!(*w, "match" | "if" | "else" | "loop"))
                    } else {
                        false
                    };

                    if is_expression_brace {
                        brace_depth += 1;
                    } else {
                        // This is the function body `{`
                        return;
                    }
                } else {
                    brace_depth += 1;
                }
            } else if c == '}' {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
            } else if paren_depth == 0 && brace_depth == 0 && c == ',' {
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

        let mut depth: u32 = 1;
        let mut result = String::new();
        let mut state = ParserState::Normal;

        while self.pos < self.chars.len() && depth > 0 {
            let c = self.chars[self.pos];
            let next = self.peek_char(1);

            match state {
                ParserState::Normal => {
                    let action = self.read_until_balanced_normal(c, next, open, close, &mut depth, &mut result);
                    match action {
                        BalancedAction::Finished(s) => return s,
                        BalancedAction::SkipAndTransition(new_state) => {
                            state = new_state;
                            continue;
                        }
                        BalancedAction::Transition(new_state) => state = new_state,
                    }
                }
                ParserState::LineComment | ParserState::BlockComment => {
                    state = self.read_until_balanced_comment(c, next, state, &mut result);
                    if state == ParserState::Normal && c == '*' {
                        // Block comment close already advanced pos
                        continue;
                    }
                }
                ParserState::String | ParserState::Char => {
                    state = self.read_until_balanced_literal(c, next, state, &mut result);
                }
            }

            self.pos += 1;
        }

        result
    }

    /// Handle a character in normal state during balanced reading.
    ///
    /// Returns `Finished` when balanced close is found, `SkipAndTransition` for
    /// comment starts (which advance pos internally), or `Transition` for other cases.
    fn read_until_balanced_normal(
        &mut self,
        c: char,
        next: Option<char>,
        open: char,
        close: char,
        depth: &mut u32,
        result: &mut String,
    ) -> BalancedAction {
        // Comment starts: push first char, advance pos, skip normal increment
        if c == '/' && next == Some('/') {
            result.push(c);
            self.pos += 1;
            return BalancedAction::SkipAndTransition(ParserState::LineComment);
        }
        if c == '/' && next == Some('*') {
            result.push(c);
            self.pos += 1;
            return BalancedAction::SkipAndTransition(ParserState::BlockComment);
        }

        // Literal starts: push char, transition, let loop increment pos
        if c == '"' {
            result.push(c);
            return BalancedAction::Transition(ParserState::String);
        }
        if c == '\'' {
            result.push(c);
            return BalancedAction::Transition(ParserState::Char);
        }

        // Delimiter tracking
        if c == open {
            *depth += 1;
        } else if c == close {
            *depth -= 1;
            if *depth == 0 {
                self.pos += 1;
                return BalancedAction::Finished(result.clone());
            }
        }
        result.push(c);
        BalancedAction::Transition(ParserState::Normal)
    }

    /// Handle a character in comment state (line or block).
    ///
    /// For block comment close (`*/`), advances pos past the `/` so the caller
    /// should `continue` to skip the normal `pos += 1`.
    fn read_until_balanced_comment(
        &mut self,
        c: char,
        next: Option<char>,
        state: ParserState,
        result: &mut String,
    ) -> ParserState {
        result.push(c);
        match state {
            ParserState::LineComment => {
                if c == '\n' {
                    ParserState::Normal
                } else {
                    state
                }
            }
            ParserState::BlockComment => {
                if c == '*' && next == Some('/') {
                    result.push('/');
                    self.pos += 2;
                    ParserState::Normal
                } else {
                    state
                }
            }
            _ => state,
        }
    }

    /// Handle a character in string or char literal state.
    ///
    /// Handles escape sequences by consuming the escaped character.
    fn read_until_balanced_literal(
        &mut self,
        c: char,
        next: Option<char>,
        state: ParserState,
        result: &mut String,
    ) -> ParserState {
        result.push(c);

        // Handle escape sequences in both string and char literals
        if c == '\\' && next.is_some() {
            result.push(self.chars[self.pos + 1]);
            self.pos += 1;
            return state;
        }

        let closing_quote = match state {
            ParserState::String => '"',
            ParserState::Char => '\'',
            _ => return state,
        };

        if c == closing_quote { ParserState::Normal } else { state }
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
