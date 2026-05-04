use alloc::string::String;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;
use super::super::forge::JjNativeOperation;

const FORGE_AUTH_KEY: &str = "_forge:";

struct FederationResourceParts<'a> {
    origin_or_repo: &'a str,
    repo_id: &'a str,
}

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

fn federation_resource_id(parts: FederationResourceParts<'_>) -> String {
    if parts.origin_or_repo.is_empty() {
        parts.repo_id.to_string()
    } else {
        alloc::format!("{}:{}", parts.origin_or_repo, parts.repo_id)
    }
}

fn federation_pull_operation(fed_id: impl Into<String>) -> Operation {
    Operation::FederationPull { fed_id: fed_id.into() }
}

fn federation_push_operation(fed_id: impl Into<String>) -> Operation {
    Operation::FederationPush { fed_id: fed_id.into() }
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
        // Nostr challenge/verification bootstrap authentication and therefore
        // must remain callable before a capability token exists.
        ClientRpcRequest::NostrAuthChallenge { .. } | ClientRpcRequest::NostrAuthVerify { .. } => Some(None),
        ClientRpcRequest::ForgeJjNative { request } => Some(Some(jj_native_auth_operation(&request.operation))),
        _ => forge_write_operation_for(request)
            .or_else(|| federation_operation_for(request))
            .or_else(|| forge_read_operation_for(request)),
    }
}

fn forge_write_operation_for(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
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
        | ClientRpcRequest::ForgeCreateDiscussion { .. }
        | ClientRpcRequest::ForgeReplyDiscussion { .. }
        | ClientRpcRequest::ForgeLockDiscussion { .. }
        | ClientRpcRequest::ForgeUnlockDiscussion { .. }
        | ClientRpcRequest::ForgeCloseDiscussion { .. }
        | ClientRpcRequest::ForgeReopenDiscussion { .. }
        | ClientRpcRequest::ForgeForkRepo { .. }
        | ClientRpcRequest::ForgeSetMirror { .. }
        | ClientRpcRequest::ForgeDisableMirror { .. }
        | ClientRpcRequest::TrustCluster { .. }
        | ClientRpcRequest::UntrustCluster { .. }
        | ClientRpcRequest::FederateRepository { .. }
        | ClientRpcRequest::FederationGrant { .. }
        | ClientRpcRequest::GitBridgePush { .. }
        | ClientRpcRequest::GitBridgePushStart { .. }
        | ClientRpcRequest::GitBridgePushChunk { .. }
        | ClientRpcRequest::GitBridgePushComplete { .. } => Some(Some(forge_write_operation())),
        _ => None,
    }
}

fn federation_operation_for(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::FederationSyncPeer { peer_node_id, .. } => {
            Some(Some(federation_pull_operation(peer_node_id.clone())))
        }
        ClientRpcRequest::FederationFetchRefs { fed_id, .. } => Some(Some(federation_pull_operation(fed_id.clone()))),
        ClientRpcRequest::FederationGitListRefs {
            origin_key, repo_id, ..
        }
        | ClientRpcRequest::FederationGitFetch {
            origin_key, repo_id, ..
        } => Some(Some(federation_pull_operation(federation_resource_id(FederationResourceParts {
            origin_or_repo: origin_key,
            repo_id,
        })))),
        ClientRpcRequest::FederationPull {
            mirror_repo_id,
            repo_id,
            ..
        } => Some(Some(federation_pull_operation(
            repo_id.as_ref().or(mirror_repo_id.as_ref()).cloned().unwrap_or_default(),
        ))),
        ClientRpcRequest::FederationPush { repo_id, .. } | ClientRpcRequest::FederationBidiSync { repo_id, .. } => {
            Some(Some(federation_push_operation(repo_id.clone())))
        }
        ClientRpcRequest::ForgeFetchFederated { federated_id, .. } => {
            Some(Some(federation_pull_operation(federated_id.clone())))
        }
        _ => None,
    }
}

fn forge_read_operation_for(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::NostrGetProfile { .. }
        | ClientRpcRequest::ForgeGetRepo { .. }
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
        | ClientRpcRequest::ForgeListDiscussions { .. }
        | ClientRpcRequest::ForgeGetDiscussion { .. }
        | ClientRpcRequest::ForgeGetMirrorStatus { .. }
        | ClientRpcRequest::ForgeDiffCommits { .. }
        | ClientRpcRequest::ForgeDiffRefs { .. }
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
    use aspen_auth_core::Capability;

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

    #[test]
    fn federation_read_requests_require_federation_pull_scope() {
        let fetch_refs = operation_for(&ClientRpcRequest::FederationFetchRefs {
            peer_node_id: "peer".to_string(),
            peer_addr: None,
            fed_id: "origin:repo-a".to_string(),
        });
        assert!(matches!(fetch_refs, Operation::FederationPull { fed_id } if fed_id == "origin:repo-a"));

        let git_list_refs = operation_for(&ClientRpcRequest::FederationGitListRefs {
            origin_key: "origin".to_string(),
            repo_id: "repo-list".to_string(),
            origin_addr_hint: None,
        });
        assert!(matches!(git_list_refs, Operation::FederationPull { fed_id } if fed_id == "origin:repo-list"));

        let git_fetch = operation_for(&ClientRpcRequest::FederationGitFetch {
            origin_key: "origin".to_string(),
            repo_id: "repo-b".to_string(),
            want: Vec::new(),
            have: Vec::new(),
            origin_addr_hint: None,
        });
        assert!(matches!(git_fetch, Operation::FederationPull { fed_id } if fed_id == "origin:repo-b"));

        let pull = operation_for(&ClientRpcRequest::FederationPull {
            mirror_repo_id: None,
            peer_node_id: Some("peer".to_string()),
            peer_addr: None,
            repo_id: Some("repo-c".to_string()),
        });
        assert!(matches!(pull, Operation::FederationPull { fed_id } if fed_id == "repo-c"));

        let sync_peer = operation_for(&ClientRpcRequest::FederationSyncPeer {
            peer_node_id: "peer-node".to_string(),
            peer_addr: None,
        });
        assert!(matches!(sync_peer, Operation::FederationPull { fed_id } if fed_id == "peer-node"));

        let fetch_federated = operation_for(&ClientRpcRequest::ForgeFetchFederated {
            federated_id: "origin:repo-d".to_string(),
            remote_cluster: "origin".to_string(),
        });
        assert!(matches!(fetch_federated, Operation::FederationPull { fed_id } if fed_id == "origin:repo-d"));
    }

    #[test]
    fn federation_write_requests_require_federation_push_scope() {
        let push = operation_for(&ClientRpcRequest::FederationPush {
            peer_node_id: "peer".to_string(),
            peer_addr: None,
            repo_id: "repo-a".to_string(),
        });
        assert!(matches!(push, Operation::FederationPush { fed_id } if fed_id == "repo-a"));

        let bidi = operation_for(&ClientRpcRequest::FederationBidiSync {
            peer_node_id: "peer".to_string(),
            peer_addr: None,
            repo_id: "repo-b".to_string(),
            push_wins: false,
        });
        assert!(matches!(bidi, Operation::FederationPush { fed_id } if fed_id == "repo-b"));
    }

    #[test]
    fn generic_forge_or_kv_scopes_do_not_authorize_federation_sync_operations() {
        let forge_write = Capability::Write {
            prefix: FORGE_AUTH_KEY.to_string(),
        };
        let kv_full = Capability::Full { prefix: String::new() };
        let federation_pull = Operation::FederationPull {
            fed_id: "origin:repo".to_string(),
        };
        let federation_push = Operation::FederationPush {
            fed_id: "origin:repo".to_string(),
        };

        assert!(!forge_write.authorizes(&federation_pull));
        assert!(!forge_write.authorizes(&federation_push));
        assert!(!kv_full.authorizes(&federation_pull));
        assert!(!kv_full.authorizes(&federation_push));
    }
}
