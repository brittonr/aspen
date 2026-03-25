//! Unit tests for pure functions in the snix build pipeline.
//!
//! Every test in this file runs without spawning any `nix` or `nix-store`
//! subprocess. Functions under test are deterministic transformations on
//! in-memory data structures.

#![cfg(feature = "snix-build")]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::path::PathBuf;

use aspen_ci_executor_nix::build_service::derivation_to_build_request;
use aspen_ci_executor_nix::build_service::output_sandbox_path_to_store_path;
use aspen_ci_executor_nix::build_service::parse_closure_output;
use aspen_ci_executor_nix::eval::NixEvalOutput;
use aspen_ci_executor_nix::eval::NixEvaluator;
use aspen_ci_executor_nix::eval::extract_drv_path_string;
use aspen_ci_executor_nix::eval::parse_derivation;
use nix_compat::derivation::Derivation;
use nix_compat::derivation::Output;
use nix_compat::store_path::StorePath;
use snix_build::buildservice::BuildConstraints;
use snix_castore::Node;

// ============================================================================
// Helper: build a test Derivation
// ============================================================================

fn make_drv(name: &str) -> Derivation {
    let store_path = StorePath::<String>::from_absolute_path(
        format!("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-{name}").as_bytes(),
    )
    .unwrap();

    let mut outputs = BTreeMap::new();
    outputs.insert("out".to_string(), Output {
        path: Some(store_path),
        ca_hash: None,
    });

    let mut environment = BTreeMap::new();
    environment.insert("name".to_string(), name.as_bytes().into());
    environment
        .insert("out".to_string(), format!("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-{name}").as_bytes().into());
    environment.insert("builder".to_string(), "/bin/sh".as_bytes().into());
    environment.insert("system".to_string(), "x86_64-linux".as_bytes().into());

    Derivation {
        arguments: vec!["-e".to_string(), "/builder.sh".to_string()],
        builder: "/bin/sh".to_string(),
        environment,
        input_derivations: BTreeMap::new(),
        input_sources: BTreeSet::new(),
        outputs,
        system: "x86_64-linux".to_string(),
    }
}

// ============================================================================
// derivation_to_build_request
// ============================================================================

#[test]
fn test_drv_to_request_empty_inputs() {
    let drv = make_drv("empty");
    let inputs = BTreeMap::new();

    let request = derivation_to_build_request(&drv, &inputs).unwrap();

    assert_eq!(request.command_args, vec!["/bin/sh", "-e", "/builder.sh"]);
    assert_eq!(request.working_dir, PathBuf::from("build"));
    assert_eq!(request.inputs_dir, PathBuf::from("nix/store"));
    assert_eq!(request.scratch_paths, vec![PathBuf::from("build"), PathBuf::from("nix/store")]);
    assert_eq!(request.outputs.len(), 1);
    assert!(request.outputs[0].to_str().unwrap().contains("empty"));
    // No inputs → inputs map may only contain builder if it exists on disk
    // (which it won't in CI). Either way, no crash.
}

#[test]
fn test_drv_to_request_with_resolved_inputs() {
    let drv = make_drv("with-inputs");

    let input_path =
        StorePath::<String>::from_absolute_path(b"/nix/store/mp57d33657rf34lzvlbpfa1gjfv5gmpg-bar").unwrap();
    let placeholder_digest = snix_castore::B3Digest::from(&[0u8; 32]);
    let input_node = Node::Directory {
        digest: placeholder_digest,
        size: 42,
    };

    let mut inputs = BTreeMap::new();
    inputs.insert(input_path, input_node);

    let request = derivation_to_build_request(&drv, &inputs).unwrap();

    assert_eq!(request.inputs.len(), 1);
    // refscan_needles: at least 1 output + 1 input = 2
    assert!(request.refscan_needles.len() >= 2, "expected >=2 needles, got {}", request.refscan_needles.len());
}

#[test]
fn test_drv_to_request_env_vars_merged() {
    let drv = make_drv("env-test");
    let inputs = BTreeMap::new();

    let request = derivation_to_build_request(&drv, &inputs).unwrap();

    let env_keys: HashSet<String> = request.environment_vars.iter().map(|e| e.key.clone()).collect();
    // NIX defaults
    assert!(env_keys.contains("HOME"));
    assert!(env_keys.contains("NIX_STORE"));
    assert!(env_keys.contains("PATH"));
    assert!(env_keys.contains("TMPDIR"));
    // Derivation env
    assert!(env_keys.contains("name"));
    assert!(env_keys.contains("out"));
}

#[test]
fn test_drv_to_request_constraints() {
    let drv = make_drv("constraints");
    let inputs = BTreeMap::new();

    let request = derivation_to_build_request(&drv, &inputs).unwrap();

    assert!(request.constraints.contains(&BuildConstraints::System("x86_64-linux".to_string())));
    assert!(request.constraints.contains(&BuildConstraints::ProvideBinSh));
    // Non-FOD: no NetworkAccess
    assert!(!request.constraints.contains(&BuildConstraints::NetworkAccess));
}

#[test]
fn test_drv_to_request_fod_gets_network_access() {
    let mut drv = make_drv("fod");

    // Make it a fixed-output derivation
    let store_path =
        StorePath::<String>::from_absolute_path(b"/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-fod").unwrap();
    drv.outputs.clear();
    drv.outputs.insert("out".to_string(), Output {
        path: Some(store_path),
        ca_hash: Some(nix_compat::nixhash::CAHash::Flat(nix_compat::nixhash::NixHash::Sha256([0u8; 32]))),
    });

    let request = derivation_to_build_request(&drv, &BTreeMap::new()).unwrap();
    assert!(request.constraints.contains(&BuildConstraints::NetworkAccess));
}

#[test]
fn test_drv_to_request_non_fod_no_network() {
    let drv = make_drv("non-fod");
    let request = derivation_to_build_request(&drv, &BTreeMap::new()).unwrap();
    assert!(!request.constraints.contains(&BuildConstraints::NetworkAccess));
}

// ============================================================================
// parse_closure_output
// ============================================================================

#[test]
fn test_parse_closure_standard() {
    let input = "/nix/store/abc123-foo\n/nix/store/def456-bar\n/nix/store/ghi789-baz\n";
    let result = parse_closure_output(input);
    assert_eq!(result, vec!["abc123-foo", "def456-bar", "ghi789-baz"]);
}

#[test]
fn test_parse_closure_empty() {
    let result = parse_closure_output("");
    assert!(result.is_empty());
}

#[test]
fn test_parse_closure_dedup() {
    let input = "/nix/store/abc123-foo\n/nix/store/abc123-foo\n/nix/store/def456-bar\n";
    let result = parse_closure_output(input);
    assert_eq!(result, vec!["abc123-foo", "def456-bar"]);
}

#[test]
fn test_parse_closure_skips_non_store_lines() {
    let input = "some random line\n/nix/store/abc123-foo\n/tmp/garbage\n";
    let result = parse_closure_output(input);
    assert_eq!(result, vec!["abc123-foo"]);
}

#[test]
fn test_parse_closure_whitespace_handling() {
    let input = "  /nix/store/abc123-foo  \n\n  \n/nix/store/def456-bar\n";
    let result = parse_closure_output(input);
    assert_eq!(result, vec!["abc123-foo", "def456-bar"]);
}

// ============================================================================
// output_sandbox_path_to_store_path
// ============================================================================

#[test]
fn test_sandbox_path_typical() {
    let p = PathBuf::from("/tmp/builds/uuid-here/scratches/nix/store/abc123-hello");
    let result = output_sandbox_path_to_store_path(&p);
    assert_eq!(result, Some(PathBuf::from("/nix/store/abc123-hello")));
}

#[test]
fn test_sandbox_path_no_marker() {
    let p = PathBuf::from("/tmp/builds/uuid-here/something/else");
    let result = output_sandbox_path_to_store_path(&p);
    assert_eq!(result, None);
}

#[test]
fn test_sandbox_path_empty_basename() {
    let p = PathBuf::from("/tmp/builds/nix/store/");
    let result = output_sandbox_path_to_store_path(&p);
    assert_eq!(result, None);
}

#[test]
fn test_sandbox_path_nested_subpaths() {
    // Should extract only first component after nix/store/
    let p = PathBuf::from("/sandbox/nix/store/abc123-pkg/bin/hello");
    let result = output_sandbox_path_to_store_path(&p);
    assert_eq!(result, Some(PathBuf::from("/nix/store/abc123-pkg")));
}

// ============================================================================
// parse_derivation
// ============================================================================

#[test]
fn test_parse_derivation_valid_aterm() {
    let drv_aterm = br#"Derive([("out","/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0-test","","")],[],[],"x86_64-linux","/bin/sh",["-c","echo hello"],[("builder","/bin/sh"),("name","test"),("out","/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0-test"),("system","x86_64-linux")])"#;

    let (drv, output_paths) = parse_derivation(drv_aterm).unwrap();
    assert_eq!(drv.outputs.len(), 1);
    assert_eq!(output_paths.len(), 1);
    assert!(output_paths[0].starts_with("/nix/store/"));
}

#[test]
fn test_parse_derivation_invalid() {
    let result = parse_derivation(b"not valid aterm");
    assert!(result.is_err());
}

#[test]
fn test_parse_derivation_empty() {
    let result = parse_derivation(b"");
    assert!(result.is_err());
}

// ============================================================================
// extract_drv_path_string
// ============================================================================

#[test]
fn test_extract_drv_path_valid() {
    let output = NixEvalOutput {
        value: Some(snix_eval::Value::String(snix_eval::NixString::from(
            "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0-test.drv",
        ))),
        errors: vec![],
        warnings: vec![],
    };
    let result = extract_drv_path_string(&output).unwrap();
    assert_eq!(result, "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0-test.drv");
}

#[test]
fn test_extract_drv_path_non_drv() {
    let output = NixEvalOutput {
        value: Some(snix_eval::Value::String(snix_eval::NixString::from(
            "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0-test",
        ))),
        errors: vec![],
        warnings: vec![],
    };
    let err = extract_drv_path_string(&output).unwrap_err();
    assert!(err.message.contains("unexpected drvPath"));
}

#[test]
fn test_extract_drv_path_non_string() {
    let output = NixEvalOutput {
        value: Some(snix_eval::Value::Integer(42)),
        errors: vec![],
        warnings: vec![],
    };
    let err = extract_drv_path_string(&output).unwrap_err();
    assert!(err.message.contains("expected string"));
}

#[test]
fn test_extract_drv_path_none() {
    let output = NixEvalOutput {
        value: None,
        errors: vec![],
        warnings: vec![],
    };
    let err = extract_drv_path_string(&output).unwrap_err();
    assert!(err.message.contains("no value"));
}

// ============================================================================
// NixEvaluator::evaluate_pure (no subprocess — fully in-process via snix-eval)
// ============================================================================

fn test_evaluator() -> NixEvaluator {
    use std::num::NonZeroUsize;
    use std::sync::Arc;
    NixEvaluator::new(
        Arc::new(snix_castore::blobservice::MemoryBlobService::default()),
        Arc::new(
            snix_castore::directoryservice::RedbDirectoryService::new_temporary("test".to_string(), Default::default())
                .expect("failed to create temp dir service"),
        ),
        Arc::new(snix_store::pathinfoservice::LruPathInfoService::with_capacity(
            "test".to_string(),
            NonZeroUsize::new(100).unwrap(),
        )),
    )
}

#[test]
fn test_pure_eval_arithmetic() {
    let eval = test_evaluator();
    let result = eval.evaluate_pure("1 + 2");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert!(result.value.is_some());
}

#[test]
fn test_pure_eval_attrset() {
    let eval = test_evaluator();
    let result = eval.evaluate_pure("{ a = 1; b = \"hello\"; }");
    assert!(result.errors.is_empty());
    match result.value {
        Some(snix_eval::Value::Attrs(_)) => {}
        other => panic!("expected attrs, got {:?}", other),
    }
}

#[test]
fn test_pure_eval_syntax_error() {
    let eval = test_evaluator();
    let result = eval.evaluate_pure("{ a = ; }");
    assert!(!result.errors.is_empty());
    assert!(result.value.is_none());
}

#[test]
fn test_pure_eval_undefined_variable() {
    let eval = test_evaluator();
    let result = eval.evaluate_pure("undefinedVar");
    assert!(!result.errors.is_empty());
}

#[test]
fn test_pure_eval_type_error() {
    let eval = test_evaluator();
    let result = eval.evaluate_pure("1 + \"string\"");
    assert!(!result.errors.is_empty());
}

#[test]
fn test_pure_eval_source_too_large() {
    let eval = test_evaluator();
    // 1MB + 1
    let huge = "x".repeat(1_048_577);
    let result = eval.evaluate_pure(&huge);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("source too large"));
}
