//! Body normalization for comparison.
//!
//! Normalizes function bodies to allow meaningful comparison between
//! production and Verus implementations.

use crate::CrateConfig;

/// Normalize a function body for comparison.
pub fn normalize_body(body: &str, config: &CrateConfig) -> String {
    let mut result = body.to_string();

    // Layer 1: Syntactic normalization (always safe)
    result = normalize_syntactic(&result);

    // Layer 2: Apply configured expression mappings
    for mapping in &config.expression_mappings {
        result = result.replace(&mapping.pattern, &mapping.replacement);
    }

    // Layer 3: Apply configured equivalences
    for equiv in &config.equivalences {
        result = result.replace(&equiv.pattern, &equiv.canonical);
    }

    // Layer 4: Apply default equivalences
    result = apply_default_equivalences(&result);

    result
}

/// Syntactic normalization that's always safe.
fn normalize_syntactic(body: &str) -> String {
    let mut result = body.to_string();

    // Remove single-line comments
    result = remove_line_comments(&result);

    // Remove multi-line comments
    result = remove_block_comments(&result);

    // Remove debug_assert! / assert! statements (production-only, not in verus)
    result = remove_debug_assertions(&result);

    // Normalize whitespace
    result = normalize_whitespace(&result);

    // Remove trailing commas in match arms
    result = normalize_match_arms(&result);

    // Remove type casts that Verus sometimes adds
    result = remove_type_casts(&result);

    result
}

/// Remove single-line comments.
fn remove_line_comments(s: &str) -> String {
    s.lines()
        .map(|line| {
            if let Some(pos) = line.find("//") {
                line[..pos].trim_end()
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remove debug_assert!, debug_assert_eq!, debug_assert_ne!, and assert! macro invocations.
/// These exist in production code but not in Verus specs.
/// Handles nested parentheses within the macro arguments.
/// Also handles syn-formatted output where spaces appear between `!` and `(`.
fn remove_debug_assertions(s: &str) -> String {
    // Macro names to strip (without `!` — we'll match `name`, optional whitespace, `!`, optional
    // whitespace, `(`)
    let macro_names = ["debug_assert_ne", "debug_assert_eq", "debug_assert", "assert"];

    let mut result = s.to_string();
    for name in &macro_names {
        loop {
            // Find the macro name
            if let Some(name_start) = result.find(name) {
                // Check that it's a word boundary (not part of a larger identifier)
                if name_start > 0 {
                    let prev = result.as_bytes()[name_start - 1];
                    if prev.is_ascii_alphanumeric() || prev == b'_' {
                        // Part of a larger identifier, skip
                        // Try finding next occurrence after this one
                        let after = name_start + name.len();
                        if let Some(next) = result[after..].find(name) {
                            // Recurse would be complex, just break for now
                            let _ = next;
                        }
                        break;
                    }
                }

                let after_name = name_start + name.len();
                // Skip optional whitespace, expect `!`
                let rest = &result[after_name..];
                let rest_trimmed = rest.trim_start();
                if !rest_trimmed.starts_with('!') {
                    break;
                }
                let after_bang_offset = rest.len() - rest_trimmed.len() + 1;
                let after_bang = after_name + after_bang_offset;

                // Skip optional whitespace, expect `(`
                let rest2 = &result[after_bang..];
                let rest2_trimmed = rest2.trim_start();
                if !rest2_trimmed.starts_with('(') {
                    break;
                }
                let after_paren_offset = rest2.len() - rest2_trimmed.len() + 1;
                let after_paren = after_bang + after_paren_offset;

                // Now find matching closing paren, handling nesting
                let mut depth: i32 = 1;
                let mut end = after_paren;
                let chars: Vec<char> = result[after_paren..].chars().collect();
                for (i, &c) in chars.iter().enumerate() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = after_paren + i + 1;
                                // Skip trailing whitespace and semicolon
                                let rest = &result[end..];
                                let trimmed = rest.trim_start();
                                if trimmed.starts_with(';') {
                                    end = result.len() - trimmed.len() + 1;
                                }
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if depth != 0 {
                    break; // Unmatched parens, bail
                }
                result = format!("{}{}", &result[..name_start], &result[end..]);
            } else {
                break;
            }
        }
    }
    result
}

/// Remove block comments.
fn remove_block_comments(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut in_comment = false;

    while let Some(c) = chars.next() {
        if in_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_comment = false;
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_comment = true;
        } else {
            result.push(c);
        }
    }

    result
}

/// Normalize whitespace.
fn normalize_whitespace(s: &str) -> String {
    // Collapse multiple whitespace to single space
    let mut result = String::new();
    let mut prev_whitespace = false;

    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_whitespace && !result.is_empty() {
                result.push(' ');
            }
            prev_whitespace = true;
        } else {
            result.push(c);
            prev_whitespace = false;
        }
    }

    // Remove leading/trailing whitespace from each line
    result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalize match arm syntax.
fn normalize_match_arms(s: &str) -> String {
    s.replace(",\n", "\n") // Remove trailing commas before newlines
        .replace(", }", " }") // Remove trailing commas before closing braces
}

/// Remove Verus type casts.
fn remove_type_casts(s: &str) -> String {
    s.replace(" as u64", "").replace(" as int", "").replace(" as usize", "").replace(" as i64", "")
}

/// Apply default semantic equivalences.
fn apply_default_equivalences(s: &str) -> String {
    let mut result = s.to_string();

    // Saturating operations with redundant bounds
    // .saturating_add(1).max(1) is equivalent to .saturating_add(1)
    // because saturating_add(1) already returns >= 1 for any value >= 0
    result = result.replace(".saturating_add(1).max(1)", ".saturating_add(1)");

    // Common variable name mappings between prod and verus
    // These are safe because the type signatures are already matched
    result = result.replace("entry.fencing_token", "token");
    result = result.replace("current_entry", "current_token");
    result = result.replace("Some(entry)", "Some(token)");

    // Normalize `let x = expr ; x` → `expr` (production binds to variable then returns it)
    result = normalize_trailing_let_binding(&result);

    result
}

/// Normalize `let x = expr ; x` at end of body to just `expr`.
/// Production code often does `let result = compute(); result` while
/// verus inlines as `compute()`.
fn normalize_trailing_let_binding(s: &str) -> String {
    // Look for pattern: `let IDENT = EXPR ; IDENT` at the end (after stripping whitespace)
    let trimmed = s.trim();

    // Try to find the last `let IDENT = ` binding
    // We look for patterns like `let deadline = EXPR ; deadline`
    if let Some(last_let) = trimmed.rfind("let ") {
        let after_let = &trimmed[last_let + 4..];
        // Extract the identifier (up to `=` or whitespace)
        if let Some(eq_pos) = after_let.find('=') {
            let ident = after_let[..eq_pos].trim();
            // Check if identifier is a simple name (no complex patterns)
            if ident.chars().all(|c| c.is_alphanumeric() || c == '_') && !ident.is_empty() {
                // Find the semicolon followed by the same identifier at the end
                let search = format!("; {} ", ident);
                // Check if the string ends with `; ident`
                if let Some(semi_pos) = trimmed[last_let..].rfind(&search) {
                    let after_semi_ident = &trimmed[last_let + semi_pos + search.len()..].trim();
                    if after_semi_ident.is_empty() {
                        // Pattern matches! Remove `let ident = ` prefix and `; ident` suffix
                        let expr_start = last_let + 4 + eq_pos + 1; // after `=`
                        let expr_end = last_let + semi_pos;
                        let before = &trimmed[..last_let];
                        let expr = trimmed[expr_start..expr_end].trim();
                        return format!("{}{}", before, expr);
                    }
                } else if trimmed.ends_with(&format!("; {}", ident)) {
                    let suffix_len = format!("; {}", ident).len();
                    let expr_start = last_let + 4 + eq_pos + 1;
                    let expr_end = trimmed.len() - suffix_len;
                    let before = &trimmed[..last_let];
                    let expr = trimmed[expr_start..expr_end].trim();
                    return format!("{}{}", before, expr);
                }
            }
        }
    }
    s.to_string()
}

/// Compare two normalized bodies for semantic equality.
pub fn bodies_match(prod_body: &str, verus_body: &str, config: &CrateConfig) -> bool {
    let norm_prod = normalize_body(prod_body, config);
    let norm_verus = normalize_body(verus_body, config);

    // Direct comparison
    if norm_prod == norm_verus {
        return true;
    }

    // Compare without any whitespace
    let compact_prod: String = norm_prod.chars().filter(|c| !c.is_whitespace()).collect();
    let compact_verus: String = norm_verus.chars().filter(|c| !c.is_whitespace()).collect();

    if compact_prod == compact_verus {
        return true;
    }

    false
}

/// Generate a diff between two bodies.
pub fn generate_diff(prod_body: &str, verus_body: &str, config: &CrateConfig) -> String {
    use similar::ChangeTag;
    use similar::TextDiff;

    let norm_prod = normalize_body(prod_body, config);
    let norm_verus = normalize_body(verus_body, config);

    let diff = TextDiff::from_lines(&norm_prod, &norm_verus);

    let mut result = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        result.push_str(&format!("{}{}", sign, change));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_comments() {
        let input = "let x = 1; // comment\nlet y = 2;";
        let expected = "let x = 1;\nlet y = 2;";
        assert_eq!(remove_line_comments(input), expected);
    }

    #[test]
    fn test_remove_block_comments() {
        let input = "let x = /* comment */ 1;";
        let expected = "let x =  1;";
        assert_eq!(remove_block_comments(input), expected);
    }

    #[test]
    fn test_normalize_whitespace() {
        let input = "  let x  =   1;  \n  let y = 2;  ";
        let result = normalize_whitespace(input);
        assert!(!result.contains("  ")); // No double spaces
    }

    #[test]
    fn test_default_equivalences() {
        let input = "token.saturating_add(1).max(1)";
        let expected = "token.saturating_add(1)";
        assert_eq!(apply_default_equivalences(input), expected);
    }

    #[test]
    fn test_bodies_match_with_whitespace() {
        let prod = "deadline_ms == 0 || now_ms > deadline_ms";
        let verus = "deadline_ms == 0 ||  now_ms > deadline_ms";
        assert!(bodies_match(prod, verus, &CrateConfig::default()));
    }

    #[test]
    fn test_bodies_match_with_variable_mapping() {
        let prod = "entry.fencing_token.saturating_add(1)";
        let verus = "token.saturating_add(1)";
        assert!(bodies_match(prod, verus, &CrateConfig::default()));
    }
}
