//! Embedded NixOS/flake-compat for in-process flake evaluation.
//!
//! flake-compat is a pure Nix file that parses `flake.lock`, fetches inputs
//! via `fetchTarball`/`builtins.path`, and calls the flake's `outputs` function.
//! By evaluating it through snix-eval, we get full flake evaluation without
//! any `nix` subprocess — snix-eval's builtin `fetchTarball` handles HTTP
//! downloads, tarball unpacking, narHash verification, and store path computation.
//!
//! Source: <https://github.com/NixOS/flake-compat>
//! Pinned: commit 5edf11c44bc78a0d334f6334cdaf7d60d732daab (2025-12-29)
//! License: MIT

use std::io;
use std::path::PathBuf;

/// Maximum size of the eval expression we construct (Tiger Style).
const MAX_EVAL_EXPRESSION_SIZE: usize = 8192;

/// NixOS/flake-compat default.nix, pinned at commit 5edf11c44bc78a0d334f6334cdaf7d60d732daab.
///
/// This file is MIT licensed. It parses flake.lock, resolves inputs via
/// `fetchTarball` (github/gitlab/tarball/sourcehut), `builtins.path` (path),
/// and `builtins.fetchGit` (git — not supported by snix, triggers fallback).
///
/// We pass `src = { outPath = <dir>; }` to skip flake-compat's `tryFetchGit`
/// on the root source, which would fail in snix since `fetchGit` is unimplemented.
const FLAKE_COMPAT_NIX: &str = include_str!("flake_compat_bundled.nix");

/// Write the embedded flake-compat to a temp file and return its path.
///
/// The file is written to `$TMPDIR/aspen-flake-compat-<pid>/default.nix`.
/// The caller is responsible for cleanup (or let the OS handle it).
pub fn write_flake_compat_to_temp() -> io::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("aspen-flake-compat-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("default.nix");
    std::fs::write(&path, FLAKE_COMPAT_NIX)?;
    Ok(path)
}

/// Build the Nix expression that evaluates a flake via flake-compat.
///
/// Returns an expression like:
/// ```nix
/// let
///   compat = import /tmp/aspen-flake-compat-123/default.nix {
///     src = { outPath = /path/to/flake; };
///     system = "x86_64-linux";
///   };
/// in compat.outputs.packages."x86_64-linux".default.drvPath
/// ```
///
/// Passing `src = { outPath = ...; }` (attrset with outPath) instead of a bare
/// path skips flake-compat's `tryFetchGit` call on the root source, which would
/// fail because snix-eval doesn't implement `builtins.fetchGit`.
pub fn build_flake_compat_expr(
    compat_nix_path: &str,
    flake_dir: &str,
    attribute: &str,
    system: &str,
) -> io::Result<String> {
    // Build attribute path with proper quoting
    let attr_path = if attribute.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = attribute
            .split('.')
            .map(|part| {
                if part.contains('-') || part.starts_with(|c: char| c.is_ascii_digit()) {
                    format!("\"{part}\"")
                } else {
                    part.to_string()
                }
            })
            .collect();
        format!(".{}", parts.join("."))
    };

    let expr = format!(
        "let\n\
         \x20 compat = import {compat_nix_path} {{\n\
         \x20   src = {{ outPath = {flake_dir}; }};\n\
         \x20   system = \"{system}\";\n\
         \x20 }};\n\
         in compat.outputs{attr_path}.drvPath\n"
    );

    if expr.len() > MAX_EVAL_EXPRESSION_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "flake-compat expression too large: {} bytes exceeds {} limit",
                expr.len(),
                MAX_EVAL_EXPRESSION_SIZE
            ),
        ));
    }

    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_flake_compat_parses() {
        let parsed = rnix::Root::parse(FLAKE_COMPAT_NIX);
        assert!(parsed.errors().is_empty(), "flake-compat has syntax errors: {:?}", parsed.errors());
    }

    #[test]
    fn test_embedded_flake_compat_not_empty() {
        assert!(FLAKE_COMPAT_NIX.len() > 1000, "flake-compat seems truncated");
        assert!(FLAKE_COMPAT_NIX.contains("fetchTarball"), "flake-compat should contain fetchTarball");
        assert!(FLAKE_COMPAT_NIX.contains("flake.lock"), "flake-compat should reference flake.lock");
    }

    #[test]
    fn test_write_flake_compat_to_temp() {
        let path = write_flake_compat_to_temp().expect("should write");
        assert!(path.exists(), "temp file should exist");
        let content = std::fs::read_to_string(&path).expect("should read");
        assert_eq!(content, FLAKE_COMPAT_NIX);
        // Cleanup
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn test_build_expr_simple() {
        let expr =
            build_flake_compat_expr("/tmp/compat/default.nix", "/path/to/flake", "default", "x86_64-linux").unwrap();
        assert!(expr.contains("import /tmp/compat/default.nix"));
        assert!(expr.contains("outPath = /path/to/flake"));
        assert!(expr.contains("system = \"x86_64-linux\""));
        assert!(expr.contains("compat.outputs.default.drvPath"));
    }

    #[test]
    fn test_build_expr_complex_attribute() {
        let expr = build_flake_compat_expr(
            "/tmp/compat/default.nix",
            "/path/to/flake",
            "packages.x86_64-linux.default",
            "x86_64-linux",
        )
        .unwrap();
        assert!(expr.contains(r#"compat.outputs.packages."x86_64-linux".default.drvPath"#));
    }

    #[test]
    fn test_build_expr_empty_attribute() {
        let expr = build_flake_compat_expr("/tmp/compat/default.nix", "/path/to/flake", "", "x86_64-linux").unwrap();
        assert!(expr.contains("compat.outputs.drvPath"));
    }
}
