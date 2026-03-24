//! Call-flake.nix expression generation for native flake evaluation.
//!
//! This module embeds Nix's call-flake.nix expression and generates evaluation
//! expressions that combine the embedded function with flake.lock JSON and
//! pre-resolved input overrides. This enables in-process flake evaluation
//! via snix-eval without subprocess calls.
//!
//! # Usage
//!
//! ```ignore
//! use aspen_ci_executor_nix::call_flake::{build_overrides_expr, build_flake_eval_expr};
//!
//! // Build overrides from resolved inputs
//! let overrides = build_overrides_expr("/path/to/flake", "root", &resolved_inputs);
//!
//! // Build complete evaluation expression
//! let expr = build_flake_eval_expr(lock_json, &overrides, "packages.x86_64-linux.default")?;
//! ```

use std::io::Error;
use std::io::ErrorKind;
use std::io::{self};

use crate::flake_lock::ResolvedInput;

/// Maximum size of generated evaluation expression (Tiger Style).
const MAX_EVAL_EXPRESSION_SIZE: usize = 2_097_152; // 2MB

/// Nix's call-flake.nix expression, embedded verbatim.
/// Source: https://github.com/NixOS/nix/blob/2.28.x/src/libflake/call-flake.nix
///
/// This is a curried function taking three arguments:
///   1. lockFileStr: JSON string of the flake.lock contents
///   2. overrides: attrset of { nodeKey = { sourceInfo = { outPath = ...; }; dir = ""; }; }
///   3. fetchTreeFinal: builtin for fetching unfetched inputs (we stub this)
///
/// Returns: the flake's outputs attrset (with sourceInfo, inputs, etc. merged in)
const CALL_FLAKE_NIX: &str = r#"
lockFileStr:
overrides:
fetchTreeFinal:
let
  inherit (builtins) mapAttrs;
  lockFile = builtins.fromJSON lockFileStr;
  resolveInput =
    inputSpec: if builtins.isList inputSpec then getInputByPath lockFile.root inputSpec else inputSpec;
  getInputByPath =
    nodeName: path:
    if path == [ ] then
      nodeName
    else
      getInputByPath
        (resolveInput lockFile.nodes.${nodeName}.inputs.${builtins.head path})
        (builtins.tail path);
  allNodes = mapAttrs (
    key: node:
    let
      hasOverride = overrides ? ${key};
      isRelative = node.locked.type or null == "path" && builtins.substring 0 1 node.locked.path != "/";
      parentNode = allNodes.${getInputByPath lockFile.root node.parent};
      sourceInfo =
        if hasOverride then
          overrides.${key}.sourceInfo
        else if isRelative then
          parentNode.sourceInfo
        else
          fetchTreeFinal (node.info or { } // removeAttrs node.locked [ "dir" ]);
      subdir = overrides.${key}.dir or node.locked.dir or "";
      outPath =
        if !hasOverride && isRelative then
          parentNode.outPath + (if node.locked.path == "" then "" else "/" + node.locked.path)
        else
          sourceInfo.outPath + (if subdir == "" then "" else "/" + subdir);
      flake = import (outPath + "/flake.nix");
      inputs = mapAttrs (inputName: inputSpec: allNodes.${resolveInput inputSpec}.result) (
        node.inputs or { }
      );
      outputs = flake.outputs (inputs // { self = result; });
      result =
        outputs
        // sourceInfo
        // {
          inherit outPath;
          inherit inputs;
          inherit outputs;
          inherit sourceInfo;
          _type = "flake";
        };
    in
    {
      result =
        if node.flake or true then
          assert builtins.isFunction flake.outputs;
          result
        else
          sourceInfo // { inherit sourceInfo outPath; };
      inherit outPath sourceInfo;
    }
  ) lockFile.nodes;
in
allNodes.${lockFile.root}.result
"#;

/// Escape a string for inclusion in Nix expressions.
///
/// Escapes backslashes, double quotes, and dollar signs which have special
/// meaning in Nix string literals. The `''` sequence is also escaped for
/// multi-line strings.
fn escape_nix_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$").replace("''", "'''")
}

/// Quote a Nix attribute name if it contains special characters.
///
/// Attributes containing hyphens or starting with digits must be quoted
/// in Nix attribute paths.
fn quote_attr_if_needed(attr: &str) -> String {
    if attr.contains('-') || attr.starts_with(|c: char| c.is_ascii_digit()) {
        format!("\"{attr}\"")
    } else {
        attr.to_string()
    }
}

/// Build a Nix expression string for the overrides attrset.
///
/// Each node gets: `"key" = { sourceInfo = { outPath = "/nix/store/..."; narHash = "..."; rev =
/// "..."; ... }; dir = ""; };` The root node gets its outPath from flake_dir.
///
/// # Arguments
///
/// * `flake_dir` - Path to the flake source directory (for root node outPath)
/// * `root_key` - The root node key from flake.lock (usually "root")
/// * `resolved` - Array of resolved inputs with store paths
///
/// # Returns
///
/// A Nix attrset expression string ready for embedding in eval expressions.
pub fn build_overrides_expr(flake_dir: &str, root_key: &str, resolved: &[ResolvedInput]) -> String {
    let mut result = String::from("{\n");

    // Add root node with flake directory as outPath
    result.push_str(&format!(
        "  \"{}\" = {{ sourceInfo = {{ outPath = \"{}\"; }}; dir = \"\"; }};\n",
        escape_nix_string(root_key),
        escape_nix_string(flake_dir)
    ));

    // Add each resolved input
    for input in resolved {
        if !input.is_local {
            continue; // Skip inputs not available locally
        }

        result.push_str(&format!("  \"{}\" = {{ sourceInfo = {{", escape_nix_string(&input.node_key)));
        result.push_str(&format!(" outPath = \"{}\";", escape_nix_string(&input.store_path)));

        // Add metadata if available
        if let Some(ref nar_hash) = input.locked.nar_hash {
            result.push_str(&format!(" narHash = \"{}\";", escape_nix_string(nar_hash)));
        }
        if let Some(ref rev) = input.locked.rev {
            result.push_str(&format!(" rev = \"{}\";", escape_nix_string(rev)));
            // Add shortRev (first 7 characters)
            let short_rev = if rev.len() >= 7 { &rev[0..7] } else { rev };
            result.push_str(&format!(" shortRev = \"{}\";", escape_nix_string(short_rev)));
        }
        if let Some(last_modified) = input.locked.last_modified {
            result.push_str(&format!(" lastModified = {};", last_modified));
        }

        // Add dir field from locked.dir (default to "")
        let dir = input.locked.dir.as_deref().unwrap_or("");
        result.push_str(&format!(" }}; dir = \"{}\"; }};\n", escape_nix_string(dir)));
    }

    result.push_str("}\n");
    result
}

/// Build the complete evaluation expression.
///
/// Composes:
/// 1. The call-flake.nix function
/// 2. The lockFileStr argument (raw JSON from flake.lock)
/// 3. The overrides attrset
/// 4. A fetchTreeFinal stub that throws on call
/// 5. Attribute navigation to the requested path + .drvPath
///
/// Returns a Nix expression string ready for snix-eval.
///
/// # Arguments
///
/// * `lock_file_json` - Raw JSON content from flake.lock
/// * `overrides_expr` - Overrides attrset expression from `build_overrides_expr`
/// * `attribute` - Dot-separated attribute path (e.g., "packages.x86_64-linux.default")
///
/// # Errors
///
/// Returns an error if the resulting expression would exceed MAX_EVAL_EXPRESSION_SIZE.
pub fn build_flake_eval_expr(lock_file_json: &str, overrides_expr: &str, attribute: &str) -> io::Result<String> {
    // Escape lock file JSON for Nix multi-line string
    let escaped_json = escape_nix_string(lock_file_json);

    // Build attribute path with proper quoting
    let attr_path = if attribute.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = attribute.split('.').map(quote_attr_if_needed).collect();
        format!(".{}", parts.join("."))
    };

    // Construct the complete expression
    let mut expr = String::new();
    expr.push_str("let\n");
    expr.push_str("  callFlake = ");
    expr.push_str(CALL_FLAKE_NIX.trim());
    expr.push_str(";\n");
    expr.push_str(&format!("  lockFileStr = ''{}'';", escaped_json));
    expr.push_str("\n  overrides = ");
    expr.push_str(overrides_expr.trim());
    expr.push_str(";\n");
    expr.push_str("  fetchTreeFinal = attrs: throw \"fetchTreeFinal called for input type '${{attrs.type or \"unknown\"}}' — input not pre-resolved in overrides\";\n");
    expr.push_str("  flake = callFlake lockFileStr overrides fetchTreeFinal;\n");
    expr.push_str(&format!("in flake{}.drvPath\n", attr_path));

    // Check size limit (FLKEVAL-8)
    if expr.len() > MAX_EVAL_EXPRESSION_SIZE {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("evaluation expression too large: {} bytes exceeds {} limit", expr.len(), MAX_EVAL_EXPRESSION_SIZE),
        ));
    }

    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flake_lock::LockedInput;

    #[test]
    fn test_build_overrides_simple() {
        let overrides = build_overrides_expr("/path/to/flake", "root", &[]);

        // Should have root node only
        assert!(overrides.contains("\"root\""));
        assert!(overrides.contains("outPath = \"/path/to/flake\""));
        assert!(overrides.contains("dir = \"\""));
        assert!(overrides.starts_with('{'));
        assert!(overrides.ends_with("}\n"));
    }

    #[test]
    fn test_build_overrides_with_inputs() {
        let resolved = vec![ResolvedInput {
            node_key: "nixpkgs".to_string(),
            store_path: "/nix/store/abc123-source".to_string(),
            locked: LockedInput {
                input_type: "github".to_string(),
                nar_hash: Some("sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4=".to_string()),
                rev: Some("35bdbbce4d6e84baa7df6544d6127db8dd7fbaef".to_string()),
                last_modified: Some(1767927480),
                owner: Some("NixOS".to_string()),
                repo: Some("nixpkgs".to_string()),
                url: None,
                path: None,
                dir: Some("pkgs".to_string()),
                submodules: false,
            },
            is_local: true,
        }];

        let overrides = build_overrides_expr("/path/to/flake", "root", &resolved);

        // Should have root and nixpkgs
        assert!(overrides.contains("\"root\""));
        assert!(overrides.contains("\"nixpkgs\""));
        assert!(overrides.contains("outPath = \"/nix/store/abc123-source\""));
        assert!(overrides.contains("narHash = \"sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4=\""));
        assert!(overrides.contains("rev = \"35bdbbce4d6e84baa7df6544d6127db8dd7fbaef\""));
        assert!(overrides.contains("shortRev = \"35bdbbc\""));
        assert!(overrides.contains("lastModified = 1767927480"));
        assert!(overrides.contains("dir = \"pkgs\""));
    }

    #[test]
    fn test_build_flake_eval_expr_simple() {
        let lock_json = r#"{"version": 7, "root": "root", "nodes": {"root": {"inputs": {}}}}"#;
        let overrides = "{\n  \"root\" = { sourceInfo = { outPath = \"/path/to/flake\"; }; dir = \"\"; };\n}\n";

        let expr = build_flake_eval_expr(lock_json, overrides, "default").unwrap();

        // Should contain all components
        assert!(expr.contains("callFlake ="));
        assert!(expr.contains(r#"lockFileStr = ''"#));
        assert!(expr.contains("overrides ="));
        assert!(expr.contains("fetchTreeFinal = attrs: throw"));
        assert!(expr.contains("flake = callFlake"));
        assert!(expr.contains("in flake.default.drvPath"));
    }

    #[test]
    fn test_build_flake_eval_expr_complex_attribute() {
        let lock_json = r#"{"version": 7}"#;
        let overrides = "{ }";

        let expr = build_flake_eval_expr(lock_json, overrides, "packages.x86_64-linux.default").unwrap();

        // Should properly quote x86_64-linux
        assert!(expr.contains(r#"in flake.packages."x86_64-linux".default.drvPath"#));
    }

    #[test]
    fn test_nix_string_escaping() {
        // Test escaping of problematic characters
        assert_eq!(escape_nix_string(r#"test\path"#), r#"test\\path"#);
        assert_eq!(escape_nix_string(r#"test"quote"#), r#"test\"quote"#);
        assert_eq!(escape_nix_string("test$var"), r#"test\$var"#);
        assert_eq!(escape_nix_string("test''multiline"), "test'''multiline");

        // Combined escaping
        assert_eq!(escape_nix_string(r#"test\${"#), r#"test\\\${"#);
    }

    #[test]
    fn test_quote_attr_if_needed() {
        // Should quote attributes with hyphens
        assert_eq!(quote_attr_if_needed("x86_64-linux"), r#""x86_64-linux""#);
        assert_eq!(quote_attr_if_needed("arm64-darwin"), r#""arm64-darwin""#);

        // Should quote attributes starting with digits
        assert_eq!(quote_attr_if_needed("4coder"), r#""4coder""#);

        // Should not quote valid identifiers
        assert_eq!(quote_attr_if_needed("packages"), "packages");
        assert_eq!(quote_attr_if_needed("default"), "default");
        assert_eq!(quote_attr_if_needed("_private"), "_private");
    }

    #[test]
    fn test_expression_size_limit() {
        // Create a large lock file JSON that would exceed the limit
        let large_lock_json = "x".repeat(MAX_EVAL_EXPRESSION_SIZE);
        let overrides = "{}";

        let result = build_flake_eval_expr(&large_lock_json, overrides, "default");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert!(err.to_string().contains("evaluation expression too large"));
        assert!(err.to_string().contains("exceeds 2097152 limit"));
    }

    #[test]
    fn test_empty_attribute_path() {
        let lock_json = r#"{"version": 7}"#;
        let overrides = "{}";

        let expr = build_flake_eval_expr(lock_json, overrides, "").unwrap();

        // Should not have attribute navigation, just .drvPath
        assert!(expr.contains("in flake.drvPath"));
        assert!(!expr.contains("in flake..drvPath"));
    }

    #[test]
    fn test_skip_non_local_inputs() {
        let resolved = vec![
            ResolvedInput {
                node_key: "local".to_string(),
                store_path: "/nix/store/abc123-source".to_string(),
                locked: LockedInput {
                    input_type: "github".to_string(),
                    nar_hash: Some("sha256-test".to_string()),
                    rev: None,
                    last_modified: None,
                    owner: None,
                    repo: None,
                    url: None,
                    path: None,
                    dir: None,
                    submodules: false,
                },
                is_local: true,
            },
            ResolvedInput {
                node_key: "remote".to_string(),
                store_path: "/nix/store/def456-source".to_string(),
                locked: LockedInput {
                    input_type: "github".to_string(),
                    nar_hash: Some("sha256-test2".to_string()),
                    rev: None,
                    last_modified: None,
                    owner: None,
                    repo: None,
                    url: None,
                    path: None,
                    dir: None,
                    submodules: false,
                },
                is_local: false,
            },
        ];

        let overrides = build_overrides_expr("/path/to/flake", "root", &resolved);

        // Should include local input but not remote
        assert!(overrides.contains("\"local\""));
        assert!(!overrides.contains("\"remote\""));
    }

    #[test]
    fn test_short_rev_truncation() {
        let resolved = vec![ResolvedInput {
            node_key: "test".to_string(),
            store_path: "/nix/store/test-source".to_string(),
            locked: LockedInput {
                input_type: "github".to_string(),
                nar_hash: None,
                rev: Some("abcd".to_string()), // Shorter than 7 chars
                last_modified: None,
                owner: None,
                repo: None,
                url: None,
                path: None,
                dir: None,
                submodules: false,
            },
            is_local: true,
        }];

        let overrides = build_overrides_expr("/path/to/flake", "root", &resolved);

        // shortRev should be the full rev if less than 7 chars
        assert!(overrides.contains("shortRev = \"abcd\""));
    }

    #[test]
    fn test_missing_optional_fields() {
        let resolved = vec![ResolvedInput {
            node_key: "minimal".to_string(),
            store_path: "/nix/store/minimal-source".to_string(),
            locked: LockedInput {
                input_type: "github".to_string(),
                nar_hash: None,
                rev: None,
                last_modified: None,
                owner: None,
                repo: None,
                url: None,
                path: None,
                dir: None,
                submodules: false,
            },
            is_local: true,
        }];

        let overrides = build_overrides_expr("/path/to/flake", "root", &resolved);

        // Should only have outPath and dir
        assert!(overrides.contains("outPath = \"/nix/store/minimal-source\""));
        assert!(overrides.contains("dir = \"\""));
        assert!(!overrides.contains("narHash"));
        assert!(!overrides.contains("rev"));
        assert!(!overrides.contains("lastModified"));
    }
}
