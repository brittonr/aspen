//! Client API protocol definitions for Aspen.
//!
//! This crate provides the protocol definitions for the Aspen Client RPC API,
//! which allows clients to communicate with aspen-node instances over Iroh P2P
//! connections using the `aspen-client` ALPN identifier.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN to distinguish it from Raft RPC, allowing
//! clients to connect directly to nodes without needing HTTP. All communication
//! uses authenticated requests with optional capability tokens for authorization.
//!
//! # Core Types
//!
//! - [`AuthenticatedRequest`] - Wrapper for requests with optional auth tokens
//! - [`ClientRpcRequest`] - Enum of all supported client operations
//! - [`ClientRpcResponse`] - Enum of all possible response types
//!
//! # Protocol Constants
//!
//! - [`CLIENT_ALPN`] - ALPN identifier for client connections
//! - [`MAX_CLIENT_MESSAGE_SIZE`] - Maximum message size (1 MB)
//! - [`MAX_CLUSTER_NODES`] - Maximum nodes in cluster state response
//!
//! # Example
//!
//! ```rust
//! use aspen_client_api::{ClientRpcRequest, AuthenticatedRequest};
//!
//! let request = ClientRpcRequest::GetHealth;
//! let auth_request = AuthenticatedRequest::unauthenticated(request);
//! ```

#![cfg_attr(not(test), no_std)]
#![cfg_attr(
    test,
    allow(
        no_panic,
        no_unwrap,
        reason = "client-api tests use panic/expect assertions for concise protocol failure messages"
    )
)]

#[macro_use]
extern crate alloc;

pub mod messages;

// Re-export all public types for convenience
pub use messages::*;

#[cfg(test)]
mod tests {
    // Test assertions intentionally use panic/unwrap-style helpers for concise
    // failure messages; production code remains covered by Tiger Style lints.
    #![allow(no_panic, no_unwrap)]

    use super::*;

    fn decode_varint(bytes: &[u8]) -> u32 {
        let mut result = 0u32;
        let mut shift = 0u32;

        for &byte in bytes {
            result |= u32::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return result;
            }
            shift += 7;
        }

        panic!("unterminated postcard discriminant varint");
    }

    fn discriminant_of<T: serde::Serialize>(value: &T) -> u32 {
        let bytes = postcard::to_allocvec(value).expect("serialize discriminant");
        decode_varint(&bytes)
    }

    // =========================================================================
    // Postcard wire-format stability tests
    //
    // Regression: `aspen-client-api` features (`ci`, `secrets`, `automerge`)
    // used to default to OFF. When the CLI was built with different features
    // than the server, `#[cfg(feature)]` variants in ClientRpcRequest /
    // ClientRpcResponse shifted postcard discriminants, causing deserialization
    // crashes ("Found a bool that wasn't 0 or 1").
    //
    // Fix: all features are now default-on so the enum layout is always
    // identical.  These tests make sure it stays that way.
    // =========================================================================

    /// All features that gate enum variants must be default-on.
    /// If a new `#[cfg(feature = "X")]` is added to a variant, add "X" here.
    #[test]
    fn test_all_enum_layout_features_are_default() {
        // These features gate variants inside ClientRpcRequest / ClientRpcResponse.
        // They MUST be in [features] default so every build sees the same layout.
        //
        // If you add a new cfg(feature)-gated variant, add the feature here AND
        // to the `default` list in Cargo.toml.
        #[cfg(not(feature = "ci"))]
        compile_error!("feature `ci` must be default-on for postcard layout stability");
        #[cfg(not(feature = "secrets"))]
        compile_error!("feature `secrets` must be default-on for postcard layout stability");
        #[cfg(not(feature = "automerge"))]
        compile_error!("feature `automerge` must be default-on for postcard layout stability");
        #[cfg(not(feature = "auth"))]
        compile_error!("feature `auth` must be default-on for postcard layout stability");
    }

    /// Postcard roundtrip for every feature-gated response variant.
    /// If any variant is compiled-out, postcard discriminants shift and this
    /// test catches it because the variant won't exist at all.
    #[test]
    fn test_feature_gated_response_variants_postcard_roundtrip() {
        // CI-gated variants
        let ci_resp = ClientRpcResponse::CacheMigrationStartResult(ci::CacheMigrationStartResultResponse {
            started: true,
            status: None,
            error: None,
        });
        let bytes = postcard::to_allocvec(&ci_resp).expect("ci serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("ci deserialize");
        assert!(matches!(decoded, ClientRpcResponse::CacheMigrationStartResult(_)));

        // Automerge-gated variants
        let am_resp = ClientRpcResponse::AutomergeCreateResult(automerge::AutomergeCreateResultResponse {
            is_success: true,
            document_id: Some("doc1".into()),
            error: None,
        });
        let bytes = postcard::to_allocvec(&am_resp).expect("automerge serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("automerge deserialize");
        assert!(matches!(decoded, ClientRpcResponse::AutomergeCreateResult(_)));

        // Secrets-gated request variant roundtrip (no response variant is
        // directly secrets-gated, but the request enum has them)
        let sec_req = ClientRpcRequest::SecretsKvRead {
            mount: "secret".into(),
            path: "db/password".into(),
            version: None,
        };
        let bytes = postcard::to_allocvec(&sec_req).expect("secrets serialize");
        let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("secrets deserialize");
        assert_eq!(decoded.variant_name(), "SecretsKvRead");
    }

    /// Postcard roundtrip for federation git clone request/response variants.
    #[test]
    fn test_federation_git_clone_postcard_roundtrip() {
        // FederationGitListRefs request
        let req = ClientRpcRequest::FederationGitListRefs {
            origin_key: "a".repeat(52),
            repo_id: "bb".repeat(32),
            origin_addr_hint: Some("192.168.1.1:12345".to_string()),
        };
        let bytes = postcard::to_allocvec(&req).expect("FederationGitListRefs req serialize");
        let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("FederationGitListRefs req deserialize");
        assert_eq!(decoded.variant_name(), "FederationGitListRefs");

        // FederationGitFetch request
        let req = ClientRpcRequest::FederationGitFetch {
            origin_key: "c".repeat(52),
            repo_id: "dd".repeat(32),
            want: vec!["abc123".to_string()],
            have: vec![],
            origin_addr_hint: None,
        };
        let bytes = postcard::to_allocvec(&req).expect("FederationGitFetch req serialize");
        let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("FederationGitFetch req deserialize");
        assert_eq!(decoded.variant_name(), "FederationGitFetch");

        // FederationGitListRefs response
        let resp = ClientRpcResponse::FederationGitListRefs(GitBridgeListRefsResponse {
            is_success: true,
            refs: vec![GitBridgeRefInfo {
                ref_name: "refs/heads/main".to_string(),
                sha1: "a".repeat(40),
            }],
            head: Some("refs/heads/main".to_string()),
            error: None,
        });
        let bytes = postcard::to_allocvec(&resp).expect("FederationGitListRefs resp serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("FederationGitListRefs resp deserialize");
        assert!(matches!(decoded, ClientRpcResponse::FederationGitListRefs(_)));

        // FederationGitFetch response
        let resp = ClientRpcResponse::FederationGitFetch(GitBridgeFetchResponse {
            is_success: true,
            objects: vec![],
            skipped: 0,
            error: None,
            chunked_session_id: None,
            total_objects: 0,
            total_chunks: 0,
        });
        let bytes = postcard::to_allocvec(&resp).expect("FederationGitFetch resp serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("FederationGitFetch resp deserialize");
        assert!(matches!(decoded, ClientRpcResponse::FederationGitFetch(_)));
    }

    /// Postcard roundtrip for core (non-gated) variants that sit at both ends
    /// of the enum. If feature-gated variants in the middle are removed, the
    /// discriminant for these will shift and this catches it.
    #[test]
    fn test_first_and_last_response_variants_postcard_stable() {
        // First variant: Health
        let first = ClientRpcResponse::Health(HealthResponse {
            status: "healthy".into(),
            node_id: 1,
            raft_node_id: Some(1),
            uptime_seconds: 100,
            is_initialized: true,
            membership_node_count: Some(3),
            iroh_node_id: None,
        });
        let first_bytes = postcard::to_allocvec(&first).expect("first serialize");
        // The first variant's postcard discriminant should be 0
        assert_eq!(discriminant_of(&first), 0, "Health must be discriminant 0");

        let decoded: ClientRpcResponse = postcard::from_bytes(&first_bytes).expect("first deserialize");
        assert!(matches!(decoded, ClientRpcResponse::Health(_)));

        // CapabilityUnavailable — second-to-last non-gated variant
        let cap = ClientRpcResponse::CapabilityUnavailable(CapabilityUnavailableResponse {
            required_app: "test".into(),
            message: "not loaded".into(),
            hints: vec![],
        });
        let cap_bytes = postcard::to_allocvec(&cap).expect("cap serialize");
        let cap_decoded: ClientRpcResponse = postcard::from_bytes(&cap_bytes).expect("cap deserialize");
        assert!(matches!(cap_decoded, ClientRpcResponse::CapabilityUnavailable(_)));

        // Last non-gated variant before feature-gated section: PluginReloadResult
        // (moved here from after the automerge block to fix discriminant drift)
        let last = ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
            is_success: true,
            plugin_count: 0,
            error: None,
            message: "ok".into(),
        });
        let last_bytes = postcard::to_allocvec(&last).expect("last serialize");
        let last_decoded: ClientRpcResponse = postcard::from_bytes(&last_bytes).expect("last deserialize");
        assert!(matches!(last_decoded, ClientRpcResponse::PluginReloadResult(_)));
    }

    /// The Error variant must always have the same discriminant so the CLI
    /// retry logic (`e.code == "NOT_LEADER"`) works regardless of features.
    #[test]
    fn test_error_response_discriminant_is_stable() {
        let err = ClientRpcResponse::error("NOT_LEADER", "try another");
        let bytes = postcard::to_allocvec(&err).expect("serialize");
        // Error is variant index 14 (0-indexed) — if this changes, the
        // CLI's retry loop may break because it deserializes the wrong type.
        // Update this value if you intentionally reorder variants BEFORE Error.
        let discriminant = discriminant_of(&err);

        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        match decoded {
            ClientRpcResponse::Error(e) => {
                assert_eq!(e.code, "NOT_LEADER");
            }
            other => panic!("Error discriminant {discriminant} decoded as {other:?}"),
        }
    }

    // =========================================================================
    // Golden-file discriminant stability (fix #4)
    //
    // Postcard encodes enum variants as varint discriminants (0, 1, 2, ...).
    // If a variant is inserted, removed, or reordered, ALL subsequent
    // discriminants shift, silently breaking wire compatibility. These tests
    // pin critical discriminants so any change is caught at test time.
    // =========================================================================

    /// Pin the postcard discriminant of critical response variants.
    ///
    /// If you intentionally add/remove/reorder variants, update this table.
    /// Each entry is (variant_name, expected_discriminant_byte).
    ///
    /// NOTE: postcard uses varint encoding. For indices < 128 the discriminant
    /// is a single byte equal to the index. For indices >= 128 it's multi-byte.
    #[test]
    fn test_response_discriminant_golden_table() {
        // Critical variants with pinned discriminants.
        // Format: (variant, expected discriminant, name for error message)
        let golden: Vec<(ClientRpcResponse, u32, &str)> = vec![
            (
                ClientRpcResponse::Health(HealthResponse {
                    status: "ok".into(),
                    node_id: 0,
                    raft_node_id: None,
                    uptime_seconds: 0,
                    is_initialized: false,
                    membership_node_count: None,
                    iroh_node_id: None,
                }),
                0,
                "Health",
            ),
            (ClientRpcResponse::Pong, 12, "Pong"),
            (ClientRpcResponse::error("X", "X"), 14, "Error"), // 14 = after Pong(12) + ClusterState(13)
        ];

        assert_eq!(golden.len(), 3, "golden response table must pin three critical variants");
        assert!(
            golden.iter().all(|(_, expected, name)| *expected < 128 && !name.is_empty()),
            "golden response table entries must stay single-byte discriminants with stable names"
        );

        for (variant, expected, name) in &golden {
            let actual = discriminant_of(variant);
            assert_eq!(
                actual, *expected,
                "GOLDEN DISCRIMINANT MISMATCH: {name} expected {expected}, got {actual}. \
                 Did you add/remove/reorder variants BEFORE {name}?"
            );
        }
    }

    /// Pin the postcard discriminant of deploy response variants.
    ///
    /// Deploy variants are non-gated and placed BEFORE the feature-gated section
    /// (after CalendarExportResult, before CapabilityUnavailable).
    #[test]
    fn test_deploy_response_discriminant_golden_table() {
        let deploy_result = ClientRpcResponse::ClusterDeployResult(ClusterDeployResultResponse {
            is_accepted: true,
            deploy_id: Some("d1".into()),
            error: None,
        });
        let deploy_status = ClientRpcResponse::ClusterDeployStatusResult(ClusterDeployStatusResultResponse {
            is_found: false,
            deploy_id: None,
            status: None,
            artifact: None,
            nodes: vec![],
            started_at_ms: None,
            elapsed_ms: None,
            error: None,
        });
        let rollback = ClientRpcResponse::ClusterRollbackResult(ClusterRollbackResultResponse {
            is_accepted: false,
            deploy_id: None,
            error: None,
        });
        let node_upgrade = ClientRpcResponse::NodeUpgradeResult(NodeUpgradeResultResponse {
            is_accepted: false,
            error: None,
        });
        let node_rollback = ClientRpcResponse::NodeRollbackResult(NodeRollbackResultResponse {
            is_success: false,
            error: None,
        });

        // All deploy variants must have discriminants LESS than CapabilityUnavailable
        let cap = ClientRpcResponse::CapabilityUnavailable(CapabilityUnavailableResponse {
            required_app: String::new(),
            message: String::new(),
            hints: vec![],
        });
        let cap_disc = discriminant_of(&cap);

        let deploy_variants: Vec<(&ClientRpcResponse, &str)> = vec![
            (&deploy_result, "ClusterDeployResult"),
            (&deploy_status, "ClusterDeployStatusResult"),
            (&rollback, "ClusterRollbackResult"),
            (&node_upgrade, "NodeUpgradeResult"),
            (&node_rollback, "NodeRollbackResult"),
        ];

        for (variant, name) in &deploy_variants {
            let disc = discriminant_of(variant);
            assert!(
                disc < cap_disc,
                "Deploy variant {name} (discriminant {disc}) must appear BEFORE \
                 CapabilityUnavailable (discriminant {cap_disc})"
            );
        }

        // Verify deploy variants are consecutive (no gaps)
        let discs: Vec<u32> = deploy_variants.iter().map(|(v, _)| discriminant_of(v)).collect();
        for window in discs.windows(2) {
            assert_eq!(window[1], window[0].saturating_add(1), "Deploy response variants must be consecutive");
        }

        // Roundtrip test for each
        for (variant, name) in &deploy_variants {
            let bytes = postcard::to_allocvec(variant).expect("serialize");
            let _decoded: ClientRpcResponse =
                postcard::from_bytes(&bytes).unwrap_or_else(|e| panic!("Failed to deserialize {name}: {e}"));
        }
    }

    /// Pin the postcard discriminant of deploy request variants.
    #[test]
    fn test_deploy_request_discriminant_golden_table() {
        let deploy_requests: Vec<(ClientRpcRequest, &str)> = vec![
            (
                ClientRpcRequest::ClusterDeploy {
                    artifact: String::new(),
                    strategy: "rolling".into(),
                    max_concurrent: 1,
                    health_timeout_secs: 120,
                    expected_binary: None,
                },
                "ClusterDeploy",
            ),
            (ClientRpcRequest::ClusterDeployStatus, "ClusterDeployStatus"),
            (ClientRpcRequest::ClusterRollback, "ClusterRollback"),
            (
                ClientRpcRequest::NodeUpgrade {
                    deploy_id: String::new(),
                    artifact: String::new(),
                    expected_binary: None,
                },
                "NodeUpgrade",
            ),
            (
                ClientRpcRequest::NodeRollback {
                    deploy_id: String::new(),
                },
                "NodeRollback",
            ),
        ];

        // All deploy requests must appear BEFORE feature-gated variants (PluginReload is last non-gated)
        let plugin_disc = discriminant_of(&ClientRpcRequest::PluginReload { name: None });

        for (req, name) in &deploy_requests {
            let disc = discriminant_of(req);
            assert!(
                disc < plugin_disc,
                "Deploy request {name} (discriminant {disc}) must appear BEFORE \
                 PluginReload (discriminant {plugin_disc})"
            );
        }

        // Verify deploy requests are consecutive
        let discs: Vec<u32> = deploy_requests.iter().map(|(r, _)| discriminant_of(r)).collect();
        for window in discs.windows(2) {
            assert_eq!(window[1], window[0].saturating_add(1), "Deploy request variants must be consecutive");
        }

        // Roundtrip test for each
        for (req, name) in &deploy_requests {
            let bytes = postcard::to_allocvec(req).expect("serialize");
            let decoded: ClientRpcRequest =
                postcard::from_bytes(&bytes).unwrap_or_else(|e| panic!("Failed to deserialize {name}: {e}"));
            assert_eq!(decoded.variant_name(), *name);
        }
    }

    /// Verify deploy variants have correct required_app.
    #[test]
    fn test_deploy_required_app() {
        assert_eq!(
            ClientRpcRequest::ClusterDeploy {
                artifact: String::new(),
                strategy: "rolling".into(),
                max_concurrent: 1,
                health_timeout_secs: 120,
                expected_binary: None,
            }
            .required_app(),
            Some("deploy")
        );
        assert_eq!(ClientRpcRequest::ClusterDeployStatus.required_app(), Some("deploy"));
        assert_eq!(ClientRpcRequest::ClusterRollback.required_app(), Some("deploy"));
        assert_eq!(
            ClientRpcRequest::NodeUpgrade {
                deploy_id: String::new(),
                artifact: String::new(),
                expected_binary: None,
            }
            .required_app(),
            Some("deploy")
        );
        assert_eq!(
            ClientRpcRequest::NodeRollback {
                deploy_id: String::new(),
            }
            .required_app(),
            Some("deploy")
        );
    }

    #[test]
    fn test_request_required_app_metadata_is_unique_and_known() {
        let known_variant_names: std::collections::HashSet<&'static str> =
            messages::request_metadata::REQUEST_VARIANT_NAMES.iter().copied().collect();
        let mut seen = std::collections::HashSet::new();

        for variant_group in messages::request_metadata::request_required_app_variant_groups() {
            for &variant_name in *variant_group {
                assert!(
                    known_variant_names.contains(variant_name),
                    "required_app mapping references unknown request variant {variant_name}"
                );
                assert!(seen.insert(variant_name), "duplicate request metadata entry for {variant_name}");
            }
        }
    }

    const APP_REQUEST_PREFIX_CONTRACTS: &[(&str, &[&str])] = &[
        ("automerge", &["Automerge"]),
        ("calendar", &["Calendar"]),
        ("ci", &["Ci"]),
        ("contacts", &["Contacts", "Net"]),
        ("deploy", &["ClusterDeploy", "ClusterRollback", "NodeRollback", "NodeUpgrade"]),
        ("forge", &[
            "FederateRepository",
            "Federation",
            "Forge",
            "GetDiscoveredCluster",
            "GetFederationStatus",
            "GitBridge",
            "Gossip",
            "ListDiscoveredClusters",
            "ListFederatedRepositories",
            "StartGossip",
            "StopGossip",
            "TrustCluster",
            "UntrustCluster",
        ]),
        ("hooks", &["Hook"]),
        ("jobs", &["Job", "Worker"]),
        ("secrets", &["Secrets"]),
        ("snix", &["Cache", "NixCache", "Snix"]),
        ("sql", &["ExecuteSql"]),
    ];

    #[test]
    fn test_app_request_routing_tables_match_prefix_contracts() {
        for &(app, prefixes) in APP_REQUEST_PREFIX_CONTRACTS {
            let expected: std::collections::BTreeSet<&'static str> = messages::request_metadata::REQUEST_VARIANT_NAMES
                .iter()
                .copied()
                .filter(|variant_name| prefixes.iter().any(|prefix| variant_name.starts_with(prefix)))
                .collect();
            let actual: std::collections::BTreeSet<&'static str> = messages::request_metadata::REQUEST_VARIANT_NAMES
                .iter()
                .copied()
                .filter(|variant_name| {
                    messages::request_metadata::request_required_app_for_variant_name(variant_name) == Some(app)
                })
                .collect();

            assert_eq!(
                expected, actual,
                "request metadata drift for app `{app}`: prefix-owned variants must match required_app routing"
            );
        }
    }

    #[test]
    fn test_app_request_routing_contract_is_documented() {
        let architecture_doc = include_str!("../../../docs/developer-guide/architecture.md");

        assert!(architecture_doc.contains("Routing metadata rule"));
        assert!(architecture_doc.contains("request_metadata_apps/"));
        assert!(architecture_doc.contains("test_app_request_routing_tables_match_prefix_contracts"));

        for &(app, prefixes) in APP_REQUEST_PREFIX_CONTRACTS {
            let table_row =
                architecture_doc.lines().find(|line| line.starts_with(&format!("| `{app}` |"))).unwrap_or_else(|| {
                    panic!("architecture doc must describe routing metadata prefix contract for app `{app}`")
                });

            for prefix in prefixes {
                assert!(
                    table_row.contains(&format!("`{prefix}*`")),
                    "architecture doc must list `{prefix}*` in the routing metadata prefix row for app `{app}`"
                );
            }
        }
    }

    #[test]
    fn test_request_metadata_lookup_matches_sample_requests() {
        let mut cases = vec![
            (ClientRpcRequest::GetHealth, "GetHealth", None),
            (ClientRpcRequest::Ping, "Ping", None),
            (
                ClientRpcRequest::ClusterDeploy {
                    artifact: String::new(),
                    strategy: "rolling".into(),
                    max_concurrent: 1,
                    health_timeout_secs: 120,
                    expected_binary: None,
                },
                "ClusterDeploy",
                Some("deploy"),
            ),
            (
                ClientRpcRequest::SecretsKvRead {
                    mount: "secret".into(),
                    path: "db/password".into(),
                    version: None,
                },
                "SecretsKvRead",
                Some("secrets"),
            ),
            (ClientRpcRequest::HookList, "HookList", Some("hooks")),
            (
                ClientRpcRequest::NetLookup {
                    name: "alice-service".into(),
                },
                "NetLookup",
                Some("contacts"),
            ),
            (
                ClientRpcRequest::FederationGitListRefs {
                    origin_key: "a".repeat(52),
                    repo_id: "bb".repeat(32),
                    origin_addr_hint: None,
                },
                "FederationGitListRefs",
                Some("forge"),
            ),
        ];
        #[cfg(feature = "ci")]
        cases.push((ClientRpcRequest::CacheMigrationStatus, "CacheMigrationStatus", Some("snix")));
        #[cfg(feature = "ci")]
        cases.push((ClientRpcRequest::CiGetRunReceipt { run_id: "run-1".into() }, "CiGetRunReceipt", Some("ci")));
        #[cfg(feature = "automerge")]
        cases.push((
            ClientRpcRequest::AutomergeCreate {
                document_id: Some("doc-1".into()),
                namespace: None,
                title: None,
                description: None,
                tags: Vec::new(),
            },
            "AutomergeCreate",
            Some("automerge"),
        ));

        for (request, expected_name, expected_app) in cases {
            assert_eq!(request.variant_name(), expected_name);
            assert_eq!(request.required_app(), expected_app);
        }
    }

    /// Verify deploy variants have correct variant_name.
    #[test]
    fn test_deploy_variant_names() {
        assert_eq!(
            ClientRpcRequest::ClusterDeploy {
                artifact: String::new(),
                strategy: "rolling".into(),
                max_concurrent: 1,
                health_timeout_secs: 120,
                expected_binary: None,
            }
            .variant_name(),
            "ClusterDeploy"
        );
        assert_eq!(ClientRpcRequest::ClusterDeployStatus.variant_name(), "ClusterDeployStatus");
        assert_eq!(ClientRpcRequest::ClusterRollback.variant_name(), "ClusterRollback");
        assert_eq!(
            ClientRpcRequest::NodeUpgrade {
                deploy_id: String::new(),
                artifact: String::new(),
                expected_binary: None,
            }
            .variant_name(),
            "NodeUpgrade"
        );
        assert_eq!(
            ClientRpcRequest::NodeRollback {
                deploy_id: String::new(),
            }
            .variant_name(),
            "NodeRollback"
        );
    }

    /// Verify deploy operations require cluster admin authorization.
    #[cfg(feature = "auth")]
    #[test]
    fn test_deploy_to_operation() {
        use aspen_auth_core::Operation;

        // Mutating deploy ops require cluster admin auth
        let deploy = ClientRpcRequest::ClusterDeploy {
            artifact: String::new(),
            strategy: "rolling".into(),
            max_concurrent: 1,
            health_timeout_secs: 120,
            expected_binary: None,
        };
        match deploy.to_operation() {
            Some(Operation::ClusterAdmin { action }) => {
                assert_eq!(action, "cluster_operation");
            }
            other => panic!("Expected ClusterAdmin for ClusterDeploy, got {other:?}"),
        }

        // Rollback requires cluster admin
        match ClientRpcRequest::ClusterRollback.to_operation() {
            Some(Operation::ClusterAdmin { action }) => {
                assert_eq!(action, "cluster_operation");
            }
            other => panic!("Expected ClusterAdmin for ClusterRollback, got {other:?}"),
        }

        // Status is read-only, no auth required
        assert!(ClientRpcRequest::ClusterDeployStatus.to_operation().is_none());
    }

    /// Pin the postcard discriminant of critical request variants.
    #[test]
    fn test_request_discriminant_golden_table() {
        let golden: Vec<(ClientRpcRequest, u32, &str)> = vec![
            (ClientRpcRequest::GetHealth, 0, "GetHealth"),
            (ClientRpcRequest::Ping, 14, "Ping"),
            (ClientRpcRequest::ReadKey { key: String::new() }, 7, "ReadKey"),
            (
                ClientRpcRequest::WriteKey {
                    key: String::new(),
                    value: vec![],
                },
                8,
                "WriteKey",
            ),
        ];

        assert_eq!(golden.len(), 4, "golden request table must pin four critical variants");
        assert!(
            golden.iter().all(|(_, expected, name)| *expected < 128 && !name.is_empty()),
            "golden request table entries must stay single-byte discriminants with stable names"
        );

        for (variant, expected, name) in &golden {
            let actual = discriminant_of(variant);
            assert_eq!(
                actual, *expected,
                "GOLDEN DISCRIMINANT MISMATCH: {name} expected {expected}, got {actual}. \
                 Did you add/remove/reorder variants BEFORE {name}?"
            );
        }
    }

    // =========================================================================
    // Variant ordering enforcement (fix #3)
    //
    // Regression: PluginReloadResult was placed AFTER the automerge
    // #[cfg(feature)] block. Since it was non-gated, its discriminant
    // shifted when automerge was toggled. This test ensures no non-gated
    // variant appears after the feature-gated section.
    // =========================================================================

    /// Verify that PluginReloadResult (non-gated) appears before AutomergeCreateResult (gated).
    ///
    /// We check this by comparing their postcard discriminants: the non-gated
    /// variant must have a LOWER discriminant than any gated variant.
    #[test]
    fn test_plugin_reload_result_before_feature_gated_variants() {
        let plugin = ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
            is_success: true,
            plugin_count: 0,
            error: None,
            message: String::new(),
        });
        let plugin_discriminant = discriminant_of(&plugin);
        assert!(plugin_discriminant > 0, "PluginReloadResult discriminant must remain non-zero");

        let automerge = ClientRpcResponse::AutomergeCreateResult(automerge::AutomergeCreateResultResponse {
            is_success: true,
            document_id: None,
            error: None,
        });
        let automerge_discriminant = discriminant_of(&automerge);

        assert!(
            plugin_discriminant < automerge_discriminant,
            "PluginReloadResult (discriminant {}) must appear BEFORE \
             AutomergeCreateResult (discriminant {}). Non-gated variants \
             must not be placed after feature-gated variants.",
            plugin_discriminant,
            automerge_discriminant,
        );
    }

    /// The `#[cfg(feature = "ci")]` CacheMigration variants must come before
    /// the `#[cfg(feature = "automerge")]` Automerge variants to avoid
    /// discriminant interleaving when one feature is on and the other off.
    #[test]
    fn test_feature_gated_variants_are_grouped_by_feature() {
        let ci_variant = ClientRpcResponse::CacheMigrationStartResult(ci::CacheMigrationStartResultResponse {
            started: true,
            status: None,
            error: None,
        });
        let ci_discriminant = discriminant_of(&ci_variant);
        assert!(ci_discriminant > 0, "CI-gated discriminant must remain non-zero");

        let am_variant = ClientRpcResponse::AutomergeCreateResult(automerge::AutomergeCreateResultResponse {
            is_success: true,
            document_id: None,
            error: None,
        });
        let am_discriminant = discriminant_of(&am_variant);

        // CI-gated variants must all have lower discriminants than automerge-gated
        assert!(
            ci_discriminant < am_discriminant,
            "CI gated variants (discriminant {}) must precede automerge \
             gated variants (discriminant {})",
            ci_discriminant,
            am_discriminant,
        );
    }

    // =========================================================================
    // Original tests
    // =========================================================================

    #[test]
    fn test_constants_are_bounded() {
        const MAX_REASONABLE_CLIENT_MESSAGE_SIZE_BYTES: usize = 16_777_216;
        const MAX_REASONABLE_CLUSTER_NODES: usize = 256;

        assert!(MAX_CLIENT_MESSAGE_SIZE > 0);
        assert!(MAX_CLIENT_MESSAGE_SIZE <= MAX_REASONABLE_CLIENT_MESSAGE_SIZE_BYTES);
        assert!(MAX_CLUSTER_NODES > 0);
        assert!(MAX_CLUSTER_NODES <= MAX_REASONABLE_CLUSTER_NODES);
        assert!(MAX_CLIENT_CONNECTIONS > 0);
        assert!(DEFAULT_GIT_CHUNK_SIZE_BYTES <= MAX_GIT_CHUNK_SIZE_BYTES);
    }

    #[test]
    fn test_client_alpn_is_correct() {
        assert_eq!(CLIENT_ALPN, b"aspen-client");
    }

    #[test]
    fn test_get_health_variant_name() {
        let req = ClientRpcRequest::GetHealth;
        assert_eq!(req.variant_name(), "GetHealth");
    }

    #[test]
    fn test_get_health_domain() {
        let req = ClientRpcRequest::GetHealth;
        // GetHealth is a core request, no forge domain
        assert_ne!(req.variant_name(), "");
    }

    #[test]
    fn test_get_health_roundtrip_json() {
        let req = ClientRpcRequest::GetHealth;
        let json = serde_json::to_string(&req).unwrap();
        let decoded: ClientRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.variant_name(), "GetHealth");
    }

    #[test]
    fn test_git_bridge_probe_objects_request_roundtrip() {
        let req = ClientRpcRequest::GitBridgeProbeObjects {
            repo_id: "repo1".into(),
            sha1s: vec!["a".repeat(40), "b".repeat(40)],
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: ClientRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.variant_name(), "GitBridgeProbeObjects");
    }

    #[test]
    fn test_git_bridge_probe_objects_variant_name() {
        let req = ClientRpcRequest::GitBridgeProbeObjects {
            repo_id: "repo1".into(),
            sha1s: vec![],
        };
        assert_eq!(req.variant_name(), "GitBridgeProbeObjects");
    }

    #[test]
    fn test_git_bridge_probe_objects_response_roundtrip() {
        let resp = ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
            is_success: true,
            known_sha1s: vec!["c".repeat(40)],
            error: None,
        });
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        if let ClientRpcResponse::GitBridgeProbeObjects(r) = decoded {
            assert!(r.is_success);
            assert_eq!(r.known_sha1s.len(), 1);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn test_authenticated_request_unauthenticated() {
        let req = AuthenticatedRequest::unauthenticated(ClientRpcRequest::GetHealth);
        assert!(req.token.is_none());
        assert_eq!(req.proxy_hops, 0);
        assert_eq!(req.request.variant_name(), "GetHealth");
    }

    #[test]
    fn test_authenticated_request_from() {
        let auth: AuthenticatedRequest = ClientRpcRequest::GetHealth.into();
        assert!(auth.token.is_none());
        assert_eq!(auth.proxy_hops, 0);
    }

    #[test]
    fn test_kv_write_variant_name() {
        let req = ClientRpcRequest::WriteKey {
            key: "test".into(),
            value: "val".into(),
        };
        assert_eq!(req.variant_name(), "WriteKey");
    }

    #[test]
    fn test_kv_read_variant_name() {
        let req = ClientRpcRequest::ReadKey { key: "test".into() };
        assert_eq!(req.variant_name(), "ReadKey");
    }

    #[test]
    fn test_response_error_roundtrip() {
        let resp = ClientRpcResponse::error("NOT_LEADER", "try another node");
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        if let ClientRpcResponse::Error(e) = decoded {
            assert_eq!(e.code, "NOT_LEADER");
            assert_eq!(e.message, "try another node");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn test_response_pong_roundtrip() {
        let resp = ClientRpcResponse::Pong;
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ClientRpcResponse::Pong));
    }

    #[test]
    fn test_response_leader_roundtrip() {
        let resp = ClientRpcResponse::Leader(Some(1));
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        if let ClientRpcResponse::Leader(id) = decoded {
            assert_eq!(id, Some(1));
        } else {
            panic!("wrong variant");
        }
    }
}

// =========================================================================
// Comprehensive method tests for ClientRpcRequest and ClientRpcResponse
// =========================================================================

/// Test that variant_name() returns non-empty strings for all core request variants.
#[test]
fn test_variant_name_core_requests() {
    assert_eq!(ClientRpcRequest::GetHealth.variant_name(), "GetHealth");
    assert_eq!(ClientRpcRequest::Ping.variant_name(), "Ping");
    assert_eq!(ClientRpcRequest::GetLeader.variant_name(), "GetLeader");
    assert_eq!(ClientRpcRequest::GetNodeInfo.variant_name(), "GetNodeInfo");
    assert_eq!(ClientRpcRequest::InitCluster.variant_name(), "InitCluster");
    assert_eq!(ClientRpcRequest::TriggerSnapshot.variant_name(), "TriggerSnapshot");

    assert_eq!(ClientRpcRequest::ReadKey { key: "k".into() }.variant_name(), "ReadKey");
    assert_eq!(
        ClientRpcRequest::WriteKey {
            key: "k".into(),
            value: vec![],
        }
        .variant_name(),
        "WriteKey"
    );
    assert_eq!(ClientRpcRequest::DeleteKey { key: "k".into() }.variant_name(), "DeleteKey");
}

/// Test that required_app() returns correct values for representative variants.
#[test]
fn test_required_app_returns_correct_domain() {
    // Core operations return None
    assert_eq!(ClientRpcRequest::GetHealth.required_app(), None);
    assert_eq!(ClientRpcRequest::ReadKey { key: "k".into() }.required_app(), None);
    assert_eq!(ClientRpcRequest::Ping.required_app(), None);

    // Forge operations return Some("forge")
    assert_eq!(
        ClientRpcRequest::ForgeCreateRepo {
            name: "repo".into(),
            description: None,
            default_branch: None,
        }
        .required_app(),
        Some("forge")
    );

    // CI operations return Some("ci")
    assert_eq!(
        ClientRpcRequest::CiTriggerPipeline {
            repo_id: "r".into(),
            ref_name: "main".into(),
            commit_hash: None,
        }
        .required_app(),
        Some("ci")
    );

    // Secrets operations return Some("secrets")
    assert_eq!(
        ClientRpcRequest::SecretsKvRead {
            mount: "secret".into(),
            path: "db/pass".into(),
            version: None,
        }
        .required_app(),
        Some("secrets")
    );

    // Jobs operations return Some("jobs")
    assert_eq!(
        ClientRpcRequest::JobSubmit {
            job_type: "build".into(),
            payload: "{}".into(),
            priority: None,
            timeout_ms: None,
            max_retries: None,
            retry_delay_ms: None,
            schedule: None,
            tags: vec![],
        }
        .required_app(),
        Some("jobs")
    );

    // Hooks operations return Some("hooks")
    assert_eq!(ClientRpcRequest::HookList.required_app(), Some("hooks"));
}

/// Test to_operation() for KV operations.
#[cfg(feature = "auth")]
#[test]
fn test_to_operation_kv_operations() {
    use aspen_auth_core::Operation;

    let read = ClientRpcRequest::ReadKey { key: "mykey".into() };
    match read.to_operation() {
        Some(Operation::Read { key }) => {
            assert_eq!(key, "mykey");
        }
        other => panic!("Expected Read operation for ReadKey, got {other:?}"),
    }

    let write = ClientRpcRequest::WriteKey {
        key: "mykey".into(),
        value: vec![1, 2, 3],
    };
    match write.to_operation() {
        Some(Operation::Write { key, value }) => {
            assert_eq!(key, "mykey");
            assert_eq!(value, vec![1, 2, 3]);
        }
        other => panic!("Expected Write operation for WriteKey, got {other:?}"),
    }

    let delete = ClientRpcRequest::DeleteKey { key: "mykey".into() };
    match delete.to_operation() {
        Some(Operation::Write { key, value }) => {
            assert_eq!(key, "mykey");
            assert_eq!(value, Vec::<u8>::new());
        }
        other => panic!("Expected Write operation for DeleteKey, got {other:?}"),
    }
}

/// Test to_operation() for cluster admin operations.
#[cfg(feature = "auth")]
#[test]
fn test_to_operation_cluster_admin() {
    use aspen_auth_core::Operation;

    let init = ClientRpcRequest::InitCluster;
    match init.to_operation() {
        Some(Operation::ClusterAdmin { action }) => {
            assert_eq!(action, "cluster_operation");
        }
        other => panic!("Expected ClusterAdmin for InitCluster, got {other:?}"),
    }

    let snapshot = ClientRpcRequest::TriggerSnapshot;
    match snapshot.to_operation() {
        Some(Operation::ClusterAdmin { action }) => {
            assert_eq!(action, "cluster_operation");
        }
        other => panic!("Expected ClusterAdmin for TriggerSnapshot, got {other:?}"),
    }
}

/// Test to_operation() returns None for operations that don't require auth.
#[cfg(feature = "auth")]
#[test]
fn test_to_operation_none_for_public_operations() {
    assert!(ClientRpcRequest::GetHealth.to_operation().is_none());
    assert!(ClientRpcRequest::Ping.to_operation().is_none());
    assert!(ClientRpcRequest::GetLeader.to_operation().is_none());
    assert!(ClientRpcRequest::GetNodeInfo.to_operation().is_none());
}

/// Test postcard serialization round-trip for batch operations.
#[test]
fn test_batch_operations_postcard_roundtrip() {
    use messages::batch::BatchCondition;
    use messages::batch::BatchWriteOperation;

    // BatchRead
    let req = ClientRpcRequest::BatchRead {
        keys: vec!["k1".into(), "k2".into(), "k3".into()],
    };
    let bytes = postcard::to_allocvec(&req).expect("serialize BatchRead");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize BatchRead");
    assert_eq!(decoded.variant_name(), "BatchRead");

    // BatchWrite
    let req = ClientRpcRequest::BatchWrite {
        operations: vec![
            BatchWriteOperation::Set {
                key: "k1".into(),
                value: vec![1, 2],
            },
            BatchWriteOperation::Delete { key: "k2".into() },
        ],
    };
    let bytes = postcard::to_allocvec(&req).expect("serialize BatchWrite");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize BatchWrite");
    assert_eq!(decoded.variant_name(), "BatchWrite");

    // ConditionalBatchWrite
    let req = ClientRpcRequest::ConditionalBatchWrite {
        conditions: vec![BatchCondition::KeyExists { key: "k1".into() }],
        operations: vec![BatchWriteOperation::Set {
            key: "k2".into(),
            value: vec![3, 4],
        }],
    };
    let bytes = postcard::to_allocvec(&req).expect("serialize ConditionalBatchWrite");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize ConditionalBatchWrite");
    assert_eq!(decoded.variant_name(), "ConditionalBatchWrite");
}

/// Test postcard serialization round-trip for watch operations.
#[test]
fn test_watch_operations_postcard_roundtrip() {
    let req = ClientRpcRequest::WatchCreate {
        prefix: "user:".into(),
        start_index: 0,
        should_include_prev_value: true,
    };
    let bytes = postcard::to_allocvec(&req).expect("serialize WatchCreate");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize WatchCreate");
    assert_eq!(decoded.variant_name(), "WatchCreate");

    let req = ClientRpcRequest::WatchCancel { watch_id: 123 };
    let bytes = postcard::to_allocvec(&req).expect("serialize WatchCancel");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize WatchCancel");
    assert_eq!(decoded.variant_name(), "WatchCancel");
}

/// Test postcard serialization round-trip for lease operations.
#[test]
fn test_lease_operations_postcard_roundtrip() {
    let req = ClientRpcRequest::LeaseGrant {
        ttl_seconds: 60,
        lease_id: Some(789),
    };
    let bytes = postcard::to_allocvec(&req).expect("serialize LeaseGrant");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize LeaseGrant");
    assert_eq!(decoded.variant_name(), "LeaseGrant");

    let req = ClientRpcRequest::LeaseRevoke { lease_id: 789 };
    let bytes = postcard::to_allocvec(&req).expect("serialize LeaseRevoke");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize LeaseRevoke");
    assert_eq!(decoded.variant_name(), "LeaseRevoke");
}

/// Test postcard serialization round-trip for coordination primitives.
#[test]
fn test_coordination_operations_postcard_roundtrip() {
    // Lock operations
    let req = ClientRpcRequest::LockAcquire {
        key: "lock1".into(),
        holder_id: "h1".into(),
        ttl_ms: 1000,
        timeout_ms: 5000,
    };
    let bytes = postcard::to_allocvec(&req).expect("serialize LockAcquire");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize LockAcquire");
    assert_eq!(decoded.variant_name(), "LockAcquire");

    // Counter operations
    let req = ClientRpcRequest::CounterIncrement { key: "cnt".into() };
    let bytes = postcard::to_allocvec(&req).expect("serialize CounterIncrement");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize CounterIncrement");
    assert_eq!(decoded.variant_name(), "CounterIncrement");

    // Sequence operations
    let req = ClientRpcRequest::SequenceNext { key: "seq".into() };
    let bytes = postcard::to_allocvec(&req).expect("serialize SequenceNext");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize SequenceNext");
    assert_eq!(decoded.variant_name(), "SequenceNext");
}

/// Test postcard serialization round-trip for blob operations.
#[test]
fn test_blob_operations_postcard_roundtrip() {
    let req = ClientRpcRequest::AddBlob {
        data: vec![1, 2, 3, 4, 5],
        tag: Some("important".into()),
    };
    let bytes = postcard::to_allocvec(&req).expect("serialize AddBlob");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize AddBlob");
    assert_eq!(decoded.variant_name(), "AddBlob");

    let req = ClientRpcRequest::GetBlob { hash: "a".repeat(64) };
    let bytes = postcard::to_allocvec(&req).expect("serialize GetBlob");
    let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("deserialize GetBlob");
    assert_eq!(decoded.variant_name(), "GetBlob");
}

/// Test ErrorResponse creation and fields.
#[test]
fn test_error_response_creation_and_fields() {
    let err = ClientRpcResponse::error("NOT_FOUND", "Key does not exist");
    match err {
        ClientRpcResponse::Error(e) => {
            assert_eq!(e.code, "NOT_FOUND");
            assert_eq!(e.message, "Key does not exist");
        }
        _ => panic!("Expected Error variant"),
    }

    // Test with different codes
    let err = ClientRpcResponse::error("NOT_LEADER", "Node is not the leader");
    match err {
        ClientRpcResponse::Error(e) => {
            assert_eq!(e.code, "NOT_LEADER");
            assert!(e.message.contains("leader"));
        }
        _ => panic!("Expected Error variant"),
    }
}

/// Test AuthenticatedRequest with proxy hops.
#[test]
fn test_authenticated_request_with_proxy_hops() {
    let req = AuthenticatedRequest::with_proxy_hops(ClientRpcRequest::Ping, None, 2);
    assert_eq!(req.proxy_hops, 2);
    assert!(req.token.is_none());
    assert_eq!(req.request.variant_name(), "Ping");

    // Test serialization with proxy hops
    let bytes = postcard::to_allocvec(&req).expect("serialize with hops");
    let decoded: AuthenticatedRequest = postcard::from_bytes(&bytes).expect("deserialize with hops");
    assert_eq!(decoded.proxy_hops, 2);
    assert_eq!(decoded.request.variant_name(), "Ping");
}

/// Test CapabilityUnavailableResponse.
#[test]
fn test_capability_unavailable_response() {
    let resp = ClientRpcResponse::CapabilityUnavailable(CapabilityUnavailableResponse {
        required_app: "forge".into(),
        message: "Forge app not loaded on this cluster".into(),
        hints: vec![CapabilityHint {
            cluster_key: "abc123".into(),
            name: "forge-cluster".into(),
            app_version: Some("1.0.0".into()),
        }],
    });

    let bytes = postcard::to_allocvec(&resp).expect("serialize");
    let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match decoded {
        ClientRpcResponse::CapabilityUnavailable(cap) => {
            assert_eq!(cap.required_app, "forge");
            assert_eq!(cap.hints.len(), 1);
            assert_eq!(cap.hints[0].cluster_key, "abc123");
        }
        _ => panic!("Expected CapabilityUnavailable variant"),
    }
}
