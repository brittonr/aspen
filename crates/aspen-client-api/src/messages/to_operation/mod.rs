mod automerge_ops;
mod batch_ops;
mod blob_ops;
mod ci_ops;
mod cluster_ops;
mod coordination_ops;
mod dns_ops;
mod docs_ops;
mod forge_ops;
mod hooks_ops;
mod jobs_ops;
mod kv_ops;
mod lease_ops;
mod pijul_ops;
mod secrets_ops;
mod sql_ops;
mod watch_ops;

use aspen_auth::Operation;

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
        .or_else(|| dns_ops::to_operation(request))
        .or_else(|| forge_ops::to_operation(request))
        .or_else(|| hooks_ops::to_operation(request))
        .or_else(|| ci_ops::to_operation(request))
        .or_else(|| secrets_ops::to_operation(request))
        .or_else(|| jobs_ops::to_operation(request))
        .or_else(|| pijul_ops::to_operation(request))
        .or_else(|| automerge_ops::to_operation(request))
        // Flatten: Option<Option<Operation>> -> Option<Operation>
        .flatten()
}
