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

use aspen_auth_core::Operation;

use super::ClientRpcRequest;

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

    fn request_variant_names() -> Vec<&'static str> {
        let mut in_request_enum = false;
        let mut variants = Vec::new();

        for line in REQUEST_SOURCE.lines() {
            if line.trim() == "pub enum ClientRpcRequest {" {
                in_request_enum = true;
                continue;
            }
            if in_request_enum && line == "}" {
                break;
            }
            if !in_request_enum || !line.starts_with("    ") || line.starts_with("        ") {
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
                variants.push(name);
            }
        }

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
}
