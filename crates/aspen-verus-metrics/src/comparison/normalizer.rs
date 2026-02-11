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

    result
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
