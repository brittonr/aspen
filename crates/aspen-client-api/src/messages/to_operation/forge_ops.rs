use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;
use super::super::forge::JjNativeOperation;

const FORGE_AUTH_KEY: &str = "_forge:";

fn forge_read_operation() -> Operation {
    Operation::Read {
        key: FORGE_AUTH_KEY.to_string(),
    }
}

fn forge_write_operation() -> Operation {
    Operation::Write {
        key: FORGE_AUTH_KEY.to_string(),
        value: vec![],
    }
}

fn jj_native_auth_operation(operation: &JjNativeOperation) -> Operation {
    match operation {
        JjNativeOperation::Clone | JjNativeOperation::Fetch | JjNativeOperation::ResolveChangeId => {
            forge_read_operation()
        }
        JjNativeOperation::Push | JjNativeOperation::BookmarkSync => forge_write_operation(),
    }
}

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::ForgeJjNative { request } => Some(Some(jj_native_auth_operation(&request.operation))),

        // Forge write operations
        ClientRpcRequest::ForgeCreateRepo { .. }
        | ClientRpcRequest::ForgeCreateRepoWithBackends { .. }
        | ClientRpcRequest::ForgeDeleteRepo { .. }
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
        | ClientRpcRequest::FederationSyncPeer { .. }
        | ClientRpcRequest::FederationFetchRefs { .. }
        | ClientRpcRequest::FederationGitListRefs { .. }
        | ClientRpcRequest::FederationGitFetch { .. }
        | ClientRpcRequest::FederationPull { .. }
        | ClientRpcRequest::FederationPush { .. }
        | ClientRpcRequest::FederationBidiSync { .. }
        | ClientRpcRequest::FederationGrant { .. }
        | ClientRpcRequest::ForgeFetchFederated { .. }
        | ClientRpcRequest::GitBridgePush { .. }
        | ClientRpcRequest::GitBridgePushStart { .. }
        | ClientRpcRequest::GitBridgePushChunk { .. }
        | ClientRpcRequest::GitBridgePushComplete { .. } => Some(Some(forge_write_operation())),

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
        | ClientRpcRequest::ForgeCheckMerge { .. }
        | ClientRpcRequest::ForgeGetDelegateKey { .. }
        | ClientRpcRequest::GitBridgeListRefs { .. }
        | ClientRpcRequest::GitBridgeFetch { .. }
        | ClientRpcRequest::GitBridgeFetchStart { .. }
        | ClientRpcRequest::GitBridgeFetchChunk { .. }
        | ClientRpcRequest::GitBridgeFetchComplete { .. }
        | ClientRpcRequest::GitBridgeProbeObjects { .. }
        | ClientRpcRequest::FederationListTokens => Some(Some(forge_read_operation())),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::forge;
    use super::*;

    fn jj_request(operation: JjNativeOperation) -> ClientRpcRequest {
        ClientRpcRequest::ForgeJjNative {
            request: forge::JjNativeRequest {
                repo_id: "repo".to_string(),
                operation,
                transport_version: forge::JJ_TRANSPORT_VERSION_CURRENT,
                want_objects: Vec::new(),
                have_objects: Vec::new(),
                change_ids: Vec::new(),
                bookmark_mutations: Vec::new(),
            },
        }
    }

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).expect("forge request maps").expect("forge request requires auth")
    }

    #[test]
    fn jj_native_read_operations_require_forge_read_auth() {
        for operation in [
            JjNativeOperation::Clone,
            JjNativeOperation::Fetch,
            JjNativeOperation::ResolveChangeId,
        ] {
            let Operation::Read { key } = operation_for(&jj_request(operation)) else {
                panic!("expected read auth");
            };
            assert_eq!(key, FORGE_AUTH_KEY);
        }
    }

    #[test]
    fn jj_native_write_operations_require_forge_write_auth() {
        for operation in [JjNativeOperation::Push, JjNativeOperation::BookmarkSync] {
            let Operation::Write { key, value } = operation_for(&jj_request(operation)) else {
                panic!("expected write auth");
            };
            assert_eq!(key, FORGE_AUTH_KEY);
            assert!(value.is_empty());
        }
    }
}
