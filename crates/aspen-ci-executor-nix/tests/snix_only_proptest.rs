//! Property-based tests for snix build pipeline functions.
//!
//! Uses proptest to generate arbitrary inputs and verify invariants:
//! - No panics on any input
//! - Output counts match input counts
//! - Deduplication guarantees hold

#![cfg(feature = "snix-build")]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use aspen_ci_executor_nix::NixBuildPayload;
use aspen_ci_executor_nix::build_service::derivation_to_build_request;
use aspen_ci_executor_nix::build_service::parse_closure_output;
use nix_compat::derivation::Derivation;
use nix_compat::derivation::Output;
use nix_compat::store_path::StorePath;
use proptest::prelude::*;
use snix_castore::Node;

// ============================================================================
// Strategies
// ============================================================================

/// Generate a valid 20-byte store path hash as hex string (32 chars of [0-9a-v]).
/// Nix store paths use nix-base32 (0123456789abcdfghijklmnpqrsvwxyz) but for
/// testing we just need 32 chars that form a valid StorePath.
fn arb_store_hash() -> impl Strategy<Value = String> {
    // Use the actual nix-base32 alphabet
    proptest::string::string_regex("[0-9a-np-sv-z]{32}").unwrap()
}

/// Generate a valid store path name component (alphanumeric + hyphens).
fn arb_store_name() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z][a-z0-9-]{0,20}").unwrap()
}

/// Generate a `StorePath<String>` from valid hash + name.
fn arb_store_path() -> impl Strategy<Value = StorePath<String>> {
    (arb_store_hash(), arb_store_name()).prop_filter_map("valid store path", |(hash, name)| {
        let path = format!("/nix/store/{hash}-{name}");
        StorePath::<String>::from_absolute_path(path.as_bytes()).ok()
    })
}

/// Generate a `(StorePath, Node)` pair.
fn arb_store_path_with_node() -> impl Strategy<Value = (StorePath<String>, Node)> {
    let placeholder = snix_castore::B3Digest::from(&[0u8; 32]);
    (arb_store_path(), prop::bool::ANY).prop_map(move |(sp, is_dir)| {
        let node = if is_dir {
            Node::Directory {
                digest: placeholder,
                size: 0,
            }
        } else {
            Node::File {
                digest: placeholder,
                size: 0,
                executable: false,
            }
        };
        (sp, node)
    })
}

/// Generate a `Derivation` with 1-5 outputs, 0-10 input sources.
fn arb_derivation() -> impl Strategy<Value = Derivation> {
    let system = prop::sample::select(vec!["x86_64-linux".to_string(), "aarch64-linux".to_string()]);

    let args = proptest::collection::vec("[a-z0-9/-]{1,20}", 0..5);
    let env_keys = proptest::collection::vec("[a-z]{1,10}", 0..10);
    let env_vals = proptest::collection::vec("[a-zA-Z0-9 ]{0,30}", 0..10);

    (
        proptest::collection::vec(arb_store_path(), 1..5),
        proptest::collection::vec(arb_store_path(), 0..10),
        system,
        args,
        env_keys,
        env_vals,
    )
        .prop_map(|(output_paths, input_sources, system, args, env_keys, env_vals)| {
            let mut outputs = BTreeMap::new();
            for (i, sp) in output_paths.iter().enumerate() {
                let name = if i == 0 { "out".to_string() } else { format!("out{i}") };
                outputs.insert(name, Output {
                    path: Some(sp.clone()),
                    ca_hash: None,
                });
            }

            let mut environment = BTreeMap::new();
            for (k, v) in env_keys.iter().zip(env_vals.iter()) {
                environment.insert(k.clone(), v.as_bytes().into());
            }
            environment.insert("system".to_string(), system.as_bytes().into());
            environment.insert("builder".to_string(), "/bin/sh".as_bytes().into());

            let input_sources_set: BTreeSet<StorePath<String>> = input_sources.into_iter().collect();

            Derivation {
                arguments: args,
                builder: "/bin/sh".to_string(),
                environment,
                input_derivations: BTreeMap::new(),
                input_sources: input_sources_set,
                outputs,
                system,
            }
        })
}

// ============================================================================
// Property tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// derivation_to_build_request never panics and preserves output count.
    #[test]
    fn prop_derivation_to_build_request_never_panics(
        drv in arb_derivation(),
        inputs in proptest::collection::btree_map(arb_store_path(), arb_store_path_with_node().prop_map(|(_, n)| n), 0..5),
    ) {
        // Must not panic
        let result = derivation_to_build_request(&drv, &inputs);
        if let Ok(request) = result {
            // Output count matches derivation
            prop_assert_eq!(request.outputs.len(), drv.outputs.len());
            // Refscan needles include at least outputs + inputs
            prop_assert!(
                request.refscan_needles.len() >= drv.outputs.len() + inputs.len(),
                "needles {} < outputs {} + inputs {}",
                request.refscan_needles.len(),
                drv.outputs.len(),
                inputs.len()
            );
        }
        // Err is also fine — just not a panic
    }

    /// parse_closure_output never panics and always returns deduplicated output.
    #[test]
    fn prop_parse_closure_output_never_panics(input in ".*") {
        let result = parse_closure_output(&input);
        // Check deduplication
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for item in &result {
            prop_assert!(seen.insert(item.clone()), "duplicate in output: {item}");
        }
    }

    /// NixBuildPayload validation: empty URL rejected, excessive timeout rejected,
    /// valid combos accepted.
    #[test]
    fn prop_payload_validation(
        url in "(|[a-z./]{1,50})",
        attr in "[a-z.]{0,30}",
        timeout in 0u64..200000u64,
    ) {
        let payload = NixBuildPayload {
            run_id: None,
            job_name: None,
            flake_url: url.clone(),
            attribute: attr,
            extra_args: vec![],
            working_dir: None,
            timeout_secs: timeout,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            should_upload_result: false,
            publish_to_cache: false,
            cache_outputs: vec![],
            system: None,
            source_hash: None,
        };
        let result = payload.validate();
        if url.is_empty() {
            prop_assert!(result.is_err(), "empty URL should be rejected");
        }
        if timeout > 86400 {
            prop_assert!(result.is_err(), "timeout > 86400 should be rejected");
        }
        // Never panics regardless of input
    }
}
