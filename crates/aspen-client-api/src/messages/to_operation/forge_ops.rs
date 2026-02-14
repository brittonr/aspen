use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Forge write operations
        ClientRpcRequest::ForgeCreateRepo { .. }
        | ClientRpcRequest::ForgeStoreBlob { .. }
        | ClientRpcRequest::ForgeCreateTree { .. }
        | ClientRpcRequest::ForgeCommit { .. }
        | ClientRpcRequest::ForgeSetRef { .. }
        | ClientRpcRequest::ForgeDeleteRef { .. }
        | ClientRpcRequest::ForgeCasRef { .. }
        | ClientRpcRequest::ForgeCreateIssue { .. }
        | ClientRpcRequest::ForgeCommentIssue { .. }
        | ClientRpcRequest::ForgeCloseIssue { .. }
        | ClientRpcRequest::ForgeReopenIssue { .. }
        | ClientRpcRequest::ForgeCreatePatch { .. }
        | ClientRpcRequest::ForgeUpdatePatch { .. }
        | ClientRpcRequest::ForgeApprovePatch { .. }
        | ClientRpcRequest::ForgeMergePatch { .. }
        | ClientRpcRequest::ForgeClosePatch { .. }
        | ClientRpcRequest::TrustCluster { .. }
        | ClientRpcRequest::UntrustCluster { .. }
        | ClientRpcRequest::FederateRepository { .. }
        | ClientRpcRequest::ForgeFetchFederated { .. }
        | ClientRpcRequest::GitBridgePush { .. }
        | ClientRpcRequest::GitBridgePushStart { .. }
        | ClientRpcRequest::GitBridgePushChunk { .. }
        | ClientRpcRequest::GitBridgePushComplete { .. } => Some(Some(Operation::Write {
            key: "_forge:".to_string(),
            value: vec![],
        })),

        // Forge read operations
        ClientRpcRequest::ForgeGetRepo { .. }
        | ClientRpcRequest::ForgeListRepos { .. }
        | ClientRpcRequest::ForgeGetBlob { .. }
        | ClientRpcRequest::ForgeGetTree { .. }
        | ClientRpcRequest::ForgeGetCommit { .. }
        | ClientRpcRequest::ForgeLog { .. }
        | ClientRpcRequest::ForgeGetRef { .. }
        | ClientRpcRequest::ForgeListBranches { .. }
        | ClientRpcRequest::ForgeListTags { .. }
        | ClientRpcRequest::ForgeListIssues { .. }
        | ClientRpcRequest::ForgeGetIssue { .. }
        | ClientRpcRequest::ForgeListPatches { .. }
        | ClientRpcRequest::ForgeGetPatch { .. }
        | ClientRpcRequest::ForgeGetDelegateKey { .. }
        | ClientRpcRequest::GitBridgeListRefs { .. }
        | ClientRpcRequest::GitBridgeFetch { .. } => Some(Some(Operation::Read {
            key: "_forge:".to_string(),
        })),

        _ => None,
    }
}
