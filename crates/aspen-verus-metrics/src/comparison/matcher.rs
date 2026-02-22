//! Function matching between production and Verus code.
//!
//! Compares production verified functions with their Verus counterparts.

use std::collections::HashMap;

use crate::ComparisonResult;
use crate::CrateConfig;
use crate::FunctionSignature;
use crate::ParsedFunction;
use crate::comparison::normalizer::bodies_match;
use crate::comparison::normalizer::generate_diff;

/// Compare production and Verus functions, returning comparison results.
pub fn compare_functions(
    production: &[ParsedFunction],
    verus: &[ParsedFunction],
    config: &CrateConfig,
) -> Vec<(String, ComparisonResult)> {
    let mut results = Vec::new();

    // Build lookup maps — use Vec for production to handle name collisions
    let mut prod_by_name: HashMap<&str, Vec<&ParsedFunction>> = HashMap::new();
    for f in production {
        prod_by_name.entry(f.name.as_str()).or_default().push(f);
    }

    let verus_by_name: HashMap<&str, &ParsedFunction> =
        verus.iter().filter(|f| f.kind.is_executable()).map(|f| (f.name.as_str(), f)).collect();

    // Track which production functions are matched
    let mut matched_prod_names: std::collections::HashSet<&str> = std::collections::HashSet::new();

    // Check all Verus exec functions have production counterparts
    for (name, verus_fn) in &verus_by_name {
        // Skip configured functions
        if config.skip_functions.contains(&name.to_string()) {
            continue;
        }

        if let Some(prod_fns) = prod_by_name.get(name) {
            // If multiple production functions with same name, pick the best match:
            // 1. Prefer signature-compatible match
            // 2. Fall back to first one
            let best = prod_fns
                .iter()
                .find(|pf| signatures_compatible(&pf.signature, &verus_fn.signature, config))
                .unwrap_or(&prod_fns[0]);
            let result = compare_single_function(best, verus_fn, config);
            results.push((name.to_string(), result));
            matched_prod_names.insert(name);
        } else {
            results.push((name.to_string(), ComparisonResult::MissingProduction {
                verus_function: name.to_string(),
                verus_file: verus_fn.file_path.clone(),
            }));
        }
    }

    // Check for production functions without Verus specs (informational only)
    // Only report one per unique name, preferring the first
    for (name, prod_fns) in &prod_by_name {
        if !verus_by_name.contains_key(name) && !config.skip_functions.contains(&name.to_string()) {
            results.push((name.to_string(), ComparisonResult::MissingVerus {
                production_function: name.to_string(),
                production_file: prod_fns[0].file_path.clone(),
            }));
        }
    }

    // Sort by function name for consistent output
    results.sort_by(|a, b| a.0.cmp(&b.0));

    results
}

/// Compare a single pair of functions.
fn compare_single_function(prod: &ParsedFunction, verus: &ParsedFunction, config: &CrateConfig) -> ComparisonResult {
    // Check if body comparison should be skipped
    if verus.skip_body {
        return ComparisonResult::SkippedExternalBody;
    }

    // Compare signatures
    if !signatures_compatible(&prod.signature, &verus.signature, config) {
        return ComparisonResult::SignatureDrift {
            production: prod.signature.clone(),
            verus: verus.signature.clone(),
        };
    }

    // Compare bodies
    let prod_body = prod.body.as_deref().unwrap_or("");
    let verus_body = verus.body.as_deref().unwrap_or("");

    if bodies_match(prod_body, verus_body, config) {
        ComparisonResult::Match
    } else {
        let diff = generate_diff(prod_body, verus_body, config);
        ComparisonResult::BodyDrift {
            production_body: prod_body.to_string(),
            verus_body: verus_body.to_string(),
            diff,
        }
    }
}

/// Check if two signatures are compatible.
///
/// Allows for minor differences like Verus's (result: Type) syntax.
fn signatures_compatible(prod: &FunctionSignature, verus: &FunctionSignature, config: &CrateConfig) -> bool {
    // Check parameter count
    if prod.params.len() != verus.params.len() {
        return false;
    }

    // Check each parameter
    for (p, v) in prod.params.iter().zip(verus.params.iter()) {
        // Names should match (ignoring leading underscore prefix for unused params)
        let p_name = p.name.strip_prefix('_').unwrap_or(&p.name);
        let v_name = v.name.strip_prefix('_').unwrap_or(&v.name);
        if p_name != v_name {
            return false;
        }

        // Types should be compatible (allowing for some variation)
        if !types_compatible(&p.ty, &v.ty, config) {
            return false;
        }
    }

    // Check return types
    match (&prod.return_type, &verus.return_type) {
        (None, None) => true,
        (Some(p), Some(v)) => types_compatible(p, v, config),
        _ => false,
    }
}

/// Check if two types are compatible.
fn types_compatible(prod_type: &str, verus_type: &str, _config: &CrateConfig) -> bool {
    let norm_prod = normalize_type(prod_type);
    let norm_verus = normalize_type(verus_type);

    if norm_prod == norm_verus {
        return true;
    }

    // TODO: Add support for configured type mappings if needed

    false
}

/// Normalize a type string for comparison.
fn normalize_type(ty: &str) -> String {
    ty.replace(' ', "")
        .replace("&'_", "&") // Elided lifetimes
        .replace("'static", "") // Static lifetime
        .replace("std::collections::", "") // Fully-qualified std paths
        .replace("std::vec::", "")
        .replace("std::string::", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FunctionParam;

    fn make_signature(params: &[(&str, &str)], ret: Option<&str>) -> FunctionSignature {
        FunctionSignature {
            params: params
                .iter()
                .map(|(n, t)| FunctionParam {
                    name: n.to_string(),
                    ty: t.to_string(),
                })
                .collect(),
            return_type: ret.map(|s| s.to_string()),
            is_const: false,
            is_async: false,
            generics: vec![],
        }
    }

    #[test]
    fn test_signatures_compatible_identical() {
        let sig = make_signature(&[("x", "u64"), ("y", "u64")], Some("bool"));
        assert!(signatures_compatible(&sig, &sig, &CrateConfig::default()));
    }

    #[test]
    fn test_signatures_compatible_different_params() {
        let sig1 = make_signature(&[("x", "u64")], Some("bool"));
        let sig2 = make_signature(&[("x", "u64"), ("y", "u64")], Some("bool"));
        assert!(!signatures_compatible(&sig1, &sig2, &CrateConfig::default()));
    }

    #[test]
    fn test_types_compatible_with_whitespace() {
        assert!(types_compatible("Option<u64>", "Option< u64 >", &CrateConfig::default()));
    }

    #[test]
    fn test_normalize_type() {
        assert_eq!(normalize_type("Option< u64 >"), "Option<u64>");
        assert_eq!(normalize_type("&'_ str"), "&str");
    }
}
