mod automerge_ops;
mod batch_ops;
mod blob_ops;
mod calendar_ops;
mod ci_ops;
mod cluster_ops;
mod contacts_ops;
mod coordination_ops;
mod deploy_ops;
mod docs_ops;
mod forge_ops;
mod hooks_ops;
mod jobs_ops;
mod kv_ops;
mod lease_ops;
mod net_ops;
mod observability_ops;
mod secrets_ops;
mod sql_ops;
mod watch_ops;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use aspen_auth_core::Operation;

use super::ClientRpcRequest;

const SNIX_DIRECTORY_KEY_PREFIX: &str = "snix:dir:";
const SNIX_PATHINFO_KEY_PREFIX: &str = "snix:pathinfo:";
const RESERVED_INTERNAL_KEY_PREFIXES: &[&str] = &[
    "_automerge:",
    "_barrier:",
    "_blob:",
    "_cache:",
    "_ci:",
    "_counter:",
    "_docs:",
    "_forge:",
    "_hooks:",
    "_jobs:",
    "_lease:",
    "_lock:",
    "_queue:",
    "_ratelimit:",
    "_rwlock:",
    "_secrets:",
    "_semaphore:",
    "_service:",
    "_sql:",
    "_sys:",
    "__worker:",
    "/_sys/",
];

fn snix_resource_from_kv_key(key: &str) -> Option<String> {
    key.strip_prefix(SNIX_DIRECTORY_KEY_PREFIX)
        .map(|digest| format!("dir:{digest}"))
        .or_else(|| key.strip_prefix(SNIX_PATHINFO_KEY_PREFIX).map(|digest| format!("pathinfo:{digest}")))
}

fn key_is_reserved_snix(key: &str) -> bool {
    snix_resource_from_kv_key(key).is_some()
}

fn key_is_reserved_internal(key: &str) -> bool {
    RESERVED_INTERNAL_KEY_PREFIXES.iter().any(|prefix| key.starts_with(prefix))
}

fn prefix_overlaps_reserved_internal(prefix: &str) -> bool {
    RESERVED_INTERNAL_KEY_PREFIXES
        .iter()
        .any(|reserved| reserved.starts_with(prefix) || prefix.starts_with(reserved))
}

fn reserved_internal_admin_operation(action: &str) -> Operation {
    Operation::ClusterAdmin {
        action: action.to_string(),
    }
}

fn reserved_snix_admin_operation(action: &str) -> Operation {
    Operation::ClusterAdmin {
        action: action.to_string(),
    }
}

fn snix_operation_for_read_key(key: &str) -> Operation {
    if key_is_reserved_internal(key) {
        return reserved_internal_admin_operation("reserved_internal_kv");
    }

    snix_resource_from_kv_key(key)
        .map_or_else(|| Operation::Read { key: key.to_string() }, |resource| Operation::SnixRead { resource })
}

fn scan_operation_for_prefix(prefix: &str) -> Operation {
    if prefix_overlaps_reserved_internal(prefix) {
        return reserved_internal_admin_operation("reserved_internal_scan");
    }

    if let Some(resource) = snix_resource_from_kv_key(prefix) {
        return Operation::SnixRead { resource };
    }

    if SNIX_DIRECTORY_KEY_PREFIX.starts_with(prefix) || SNIX_PATHINFO_KEY_PREFIX.starts_with(prefix) {
        return reserved_snix_admin_operation("reserved_snix_scan");
    }

    Operation::Read {
        key: prefix.to_string(),
    }
}

fn snix_operation_for_write_key(key: &str, value: Vec<u8>) -> Operation {
    if key_is_reserved_internal(key) {
        return reserved_internal_admin_operation("reserved_internal_kv");
    }

    snix_resource_from_kv_key(key).map_or_else(
        || Operation::Write {
            key: key.to_string(),
            value,
        },
        |resource| Operation::SnixWrite { resource },
    )
}

/// Convert the request to an authorization operation.
///
/// Returns None for operations that don't require authorization.
/// Each domain submodule returns `Option<Option<Operation>>`:
/// - `Some(Some(op))` = handled, authorization required
/// - `Some(None)` = handled, no authorization required
/// - `None` = not handled by this domain
pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Operation> {
    cluster_ops::to_operation(request)
        .or_else(|| kv_ops::to_operation(request))
        .or_else(|| batch_ops::to_operation(request))
        .or_else(|| coordination_ops::to_operation(request))
        .or_else(|| blob_ops::to_operation(request))
        .or_else(|| docs_ops::to_operation(request))
        .or_else(|| sql_ops::to_operation(request))
        .or_else(|| watch_ops::to_operation(request))
        .or_else(|| lease_ops::to_operation(request))
        .or_else(|| forge_ops::to_operation(request))
        .or_else(|| hooks_ops::to_operation(request))
        .or_else(|| ci_ops::to_operation(request))
        .or_else(|| secrets_ops::to_operation(request))
        .or_else(|| jobs_ops::to_operation(request))
        .or_else(|| automerge_ops::to_operation(request))
        .or_else(|| observability_ops::to_operation(request))
        .or_else(|| net_ops::to_operation(request))
        .or_else(|| contacts_ops::to_operation(request))
        .or_else(|| calendar_ops::to_operation(request))
        .or_else(|| deploy_ops::to_operation(request))
        // Flatten: Option<Option<Operation>> -> Option<Operation>
        .flatten()
}

#[cfg(test)]
mod tests {
    const REQUEST_SOURCE: &str = include_str!("../mod.rs");
    const AUTHORIZATION_SOURCES: &[&str] = &[
        include_str!("automerge_ops.rs"),
        include_str!("batch_ops.rs"),
        include_str!("blob_ops.rs"),
        include_str!("calendar_ops.rs"),
        include_str!("ci_ops.rs"),
        include_str!("cluster_ops.rs"),
        include_str!("contacts_ops.rs"),
        include_str!("coordination_ops.rs"),
        include_str!("deploy_ops.rs"),
        include_str!("docs_ops.rs"),
        include_str!("forge_ops.rs"),
        include_str!("hooks_ops.rs"),
        include_str!("jobs_ops.rs"),
        include_str!("kv_ops.rs"),
        include_str!("lease_ops.rs"),
        include_str!("net_ops.rs"),
        include_str!("observability_ops.rs"),
        include_str!("secrets_ops.rs"),
        include_str!("sql_ops.rs"),
        include_str!("watch_ops.rs"),
    ];

    const MAX_CLIENT_REQUEST_VARIANTS: usize = 512;

    fn request_variant_names() -> Vec<&'static str> {
        let mut is_in_request_enum = false;
        let mut variants = Vec::with_capacity(MAX_CLIENT_REQUEST_VARIANTS);

        for line in REQUEST_SOURCE.lines() {
            if line.trim() == "pub enum ClientRpcRequest {" {
                is_in_request_enum = true;
                continue;
            }
            if is_in_request_enum && line == "}" {
                break;
            }
            if !is_in_request_enum || !line.starts_with("    ") || line.starts_with("        ") {
                continue;
            }

            let trimmed = line.trim_start();
            let Some(first) = trimmed.chars().next() else {
                continue;
            };
            if !first.is_ascii_uppercase() {
                continue;
            }
            let name_len = trimmed.bytes().take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_').count();
            if name_len == 0 {
                continue;
            }
            let name = &trimmed[..name_len];
            if !variants.contains(&name) {
                assert!(variants.len() < MAX_CLIENT_REQUEST_VARIANTS);
                variants.push(name);
            }
        }

        assert!(is_in_request_enum);
        assert!(!variants.is_empty());
        variants
    }

    fn public_no_auth_variant_names() -> Vec<String> {
        let mut variants = Vec::with_capacity(32);

        for source in AUTHORIZATION_SOURCES {
            let mut arm = String::new();
            for line in source.lines() {
                let trimmed = line.trim_start();
                let is_request_arm = trimmed.starts_with("ClientRpcRequest::");
                if is_request_arm {
                    arm.clear();
                }
                if is_request_arm || (!arm.is_empty() && trimmed.starts_with('|')) {
                    arm.push_str(trimmed);
                    arm.push(' ');
                }
                if !arm.is_empty() && trimmed.contains("=> Some(None)") {
                    arm.push_str(trimmed);
                    let mut rest = arm.as_str();
                    while let Some(index) = rest.find("ClientRpcRequest::") {
                        let start = index.checked_add("ClientRpcRequest::".len());
                        assert!(start.is_some());
                        let candidate = &rest[start.unwrap_or(rest.len())..];
                        let name_len =
                            candidate.bytes().take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_').count();
                        assert!(name_len > 0);
                        let name = candidate[..name_len].to_string();
                        if !variants.contains(&name) {
                            assert!(variants.len() < 32);
                            variants.push(name);
                        }
                        rest = &candidate[name_len..];
                    }
                    arm.clear();
                }
            }
        }

        variants.sort_unstable();
        variants
    }

    #[test]
    fn every_client_request_variant_has_authorization_classification() {
        let missing: Vec<_> = request_variant_names()
            .into_iter()
            .filter(|variant| {
                let needle = alloc::format!("ClientRpcRequest::{variant}");
                !AUTHORIZATION_SOURCES.iter().any(|source| source.contains(&needle))
            })
            .collect();

        assert!(
            missing.is_empty(),
            "ClientRpcRequest variants must be explicitly classified by messages/to_operation/*.rs; missing: {missing:?}"
        );
    }

    #[test]
    fn public_no_auth_request_variants_match_audited_allowlist() {
        let expected: Vec<String> = [
            "GetClientTicket",
            "GetClusterState",
            "GetClusterTicket",
            "GetClusterTicketCombined",
            "GetDiscoveredCluster",
            "GetDocsTicket",
            "GetFederationStatus",
            "GetHealth",
            "GetKeyOrigin",
            "GetLeader",
            "GetMetrics",
            "GetNetworkMetrics",
            "GetNodeInfo",
            "GetPeerClusterStatus",
            "GetRaftMetrics",
            "GetTopology",
            "ListDiscoveredClusters",
            "ListFederatedRepositories",
            "ListPeerClusters",
            "ListVaults",
            "NostrAuthChallenge",
            "NostrAuthVerify",
            "Ping",
            "WatchCancel",
            "WatchStatus",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let actual = public_no_auth_variant_names();
        assert_eq!(expected.len(), 25);
        assert_eq!(actual.len(), expected.len());
        assert_eq!(
            actual, expected,
            "public/no-auth ClientRpcRequest variants must be reviewed explicitly; update the audit evidence before changing this allowlist"
        );
    }
}
