use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Automerge write operations
        #[cfg(feature = "automerge")]
        ClientRpcRequest::AutomergeCreate { .. }
        | ClientRpcRequest::AutomergeSave { .. }
        | ClientRpcRequest::AutomergeDelete { .. }
        | ClientRpcRequest::AutomergeApplyChanges { .. }
        | ClientRpcRequest::AutomergeMerge { .. }
        | ClientRpcRequest::AutomergeReceiveSyncMessage { .. } => Some(Some(Operation::Write {
            key: "_automerge:".to_string(),
            value: vec![],
        })),

        // Automerge read operations
        #[cfg(feature = "automerge")]
        ClientRpcRequest::AutomergeGet { .. }
        | ClientRpcRequest::AutomergeList { .. }
        | ClientRpcRequest::AutomergeGetMetadata { .. }
        | ClientRpcRequest::AutomergeExists { .. }
        | ClientRpcRequest::AutomergeGenerateSyncMessage { .. } => Some(Some(Operation::Read {
            key: "_automerge:".to_string(),
        })),

        _ => None,
    }
}
