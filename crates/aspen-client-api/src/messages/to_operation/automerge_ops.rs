use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

fn document_resource(document_id: &str) -> String {
    format!("doc:{document_id}")
}

fn create_resource(document_id: Option<&str>, namespace: Option<&str>) -> String {
    if let Some(document_id) = document_id {
        return document_resource(document_id);
    }
    if let Some(namespace) = namespace {
        return format!("namespace:{namespace}");
    }
    "document:".to_string()
}

fn list_resource(namespace: Option<&str>, tag: Option<&str>) -> String {
    if let Some(namespace) = namespace {
        return format!("namespace:{namespace}");
    }
    if let Some(tag) = tag {
        return format!("tag:{tag}");
    }
    "document:".to_string()
}

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Automerge write operations
        #[cfg(feature = "automerge")]
        ClientRpcRequest::AutomergeCreate {
            document_id, namespace, ..
        } => Some(Some(Operation::AutomergeWrite {
            resource: create_resource(document_id.as_deref(), namespace.as_deref()),
        })),
        #[cfg(feature = "automerge")]
        ClientRpcRequest::AutomergeSave { document_id, .. }
        | ClientRpcRequest::AutomergeDelete { document_id }
        | ClientRpcRequest::AutomergeApplyChanges { document_id, .. }
        | ClientRpcRequest::AutomergeReceiveSyncMessage { document_id, .. } => Some(Some(Operation::AutomergeWrite {
            resource: document_resource(document_id),
        })),
        #[cfg(feature = "automerge")]
        ClientRpcRequest::AutomergeMerge { target_document_id, .. } => Some(Some(Operation::AutomergeWrite {
            resource: document_resource(target_document_id),
        })),

        // Automerge read operations
        #[cfg(feature = "automerge")]
        ClientRpcRequest::AutomergeGet { document_id }
        | ClientRpcRequest::AutomergeGetMetadata { document_id }
        | ClientRpcRequest::AutomergeExists { document_id }
        | ClientRpcRequest::AutomergeGenerateSyncMessage { document_id, .. } => Some(Some(Operation::AutomergeRead {
            resource: document_resource(document_id),
        })),
        #[cfg(feature = "automerge")]
        ClientRpcRequest::AutomergeList { namespace, tag, .. } => Some(Some(Operation::AutomergeRead {
            resource: list_resource(namespace.as_deref(), tag.as_deref()),
        })),

        _ => None,
    }
}

#[cfg(all(test, feature = "automerge"))]
mod tests {
    use aspen_auth_core::Capability;

    use super::*;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).flatten().expect("request should require auth")
    }

    #[test]
    fn automerge_requests_use_domain_specific_capabilities() {
        let create = operation_for(&ClientRpcRequest::AutomergeCreate {
            document_id: Some("doc-1".to_string()),
            namespace: Some("team".to_string()),
            title: None,
            description: None,
            tags: Vec::new(),
        });
        assert!(matches!(create, Operation::AutomergeWrite { resource } if resource == "doc:doc-1"));

        let list = operation_for(&ClientRpcRequest::AutomergeList {
            namespace: Some("team".to_string()),
            tag: None,
            limit: None,
            continuation_token: None,
        });
        assert!(matches!(list, Operation::AutomergeRead { resource } if resource == "namespace:team"));

        let sync_read = operation_for(&ClientRpcRequest::AutomergeGenerateSyncMessage {
            document_id: "doc-1".to_string(),
            peer_id: "peer-1".to_string(),
            sync_state: None,
        });
        assert!(matches!(sync_read, Operation::AutomergeRead { resource } if resource == "doc:doc-1"));

        let sync_write = operation_for(&ClientRpcRequest::AutomergeReceiveSyncMessage {
            document_id: "doc-1".to_string(),
            peer_id: "peer-1".to_string(),
            message: "msg".to_string(),
            sync_state: None,
        });
        assert!(matches!(sync_write, Operation::AutomergeWrite { resource } if resource == "doc:doc-1"));
    }

    #[test]
    fn generic_automerge_prefixes_do_not_authorize_automerge_requests() {
        let generic_read = Capability::Read {
            prefix: "_automerge:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "_automerge:".to_string(),
        };
        let automerge_read = Capability::AutomergeRead {
            resource_prefix: "doc:".to_string(),
        };
        let automerge_write = Capability::AutomergeWrite {
            resource_prefix: "doc:".to_string(),
        };

        let get = operation_for(&ClientRpcRequest::AutomergeGet {
            document_id: "doc-1".to_string(),
        });
        assert!(!generic_read.authorizes(&get));
        assert!(automerge_read.authorizes(&get));

        let save = operation_for(&ClientRpcRequest::AutomergeSave {
            document_id: "doc-1".to_string(),
            document_bytes: "bytes".to_string(),
        });
        assert!(!generic_write.authorizes(&save));
        assert!(automerge_write.authorizes(&save));
    }
}
