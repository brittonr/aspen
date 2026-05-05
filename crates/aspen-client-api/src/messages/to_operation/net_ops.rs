use alloc::format;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

const NET_LOOKUP_AUTHORIZATION_PORT: u16 = 0;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::NetPublish { name, .. } => Some(Some(Operation::NetPublish {
            service: name.to_string(),
        })),
        ClientRpcRequest::NetUnpublish { name, .. } => Some(Some(Operation::NetUnpublish {
            service: name.to_string(),
        })),
        ClientRpcRequest::NetLookup { name, .. } => Some(Some(Operation::NetConnect {
            service: name.to_string(),
            port: NET_LOOKUP_AUTHORIZATION_PORT,
        })),
        ClientRpcRequest::NetList { tag_filter } => Some(Some(Operation::NetAdmin {
            action: match tag_filter {
                Some(tag) => format!("list_services:{tag}"),
                None => "list_services".to_string(),
            },
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use aspen_auth_core::Capability;

    use super::*;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).flatten().expect("request should require auth")
    }

    #[test]
    fn net_registry_requests_use_net_capabilities_not_sys_prefixes() {
        let publish = operation_for(&ClientRpcRequest::NetPublish {
            name: "prod/api".to_string(),
            endpoint_id: "node-a".to_string(),
            port: 443,
            proto: "tcp".to_string(),
            tags: vec!["prod".to_string()],
        });
        assert!(matches!(publish, Operation::NetPublish { service } if service == "prod/api"));

        let unpublish = operation_for(&ClientRpcRequest::NetUnpublish {
            name: "prod/api".to_string(),
        });
        assert!(matches!(unpublish, Operation::NetUnpublish { service } if service == "prod/api"));

        let lookup = operation_for(&ClientRpcRequest::NetLookup {
            name: "prod/api".to_string(),
        });
        assert!(
            matches!(lookup, Operation::NetConnect { service, port } if service == "prod/api" && port == NET_LOOKUP_AUTHORIZATION_PORT)
        );

        let list = operation_for(&ClientRpcRequest::NetList {
            tag_filter: Some("prod".to_string()),
        });
        assert!(matches!(list, Operation::NetAdmin { action } if action == "list_services:prod"));
    }

    #[test]
    fn generic_sys_prefixes_do_not_authorize_net_registry_requests() {
        let generic_read = Capability::Read {
            prefix: "/_sys/net/svc/".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "/_sys/net/svc/".to_string(),
        };
        let publish_capability = Capability::NetPublish {
            service_prefix: "prod/".to_string(),
        };
        let connect_capability = Capability::NetConnect {
            service_prefix: "prod/".to_string(),
        };
        let admin_capability = Capability::NetAdmin;

        let publish = operation_for(&ClientRpcRequest::NetPublish {
            name: "prod/api".to_string(),
            endpoint_id: "node-a".to_string(),
            port: 443,
            proto: "tcp".to_string(),
            tags: vec!["prod".to_string()],
        });
        assert!(!generic_write.authorizes(&publish));
        assert!(publish_capability.authorizes(&publish));

        let lookup = operation_for(&ClientRpcRequest::NetLookup {
            name: "prod/api".to_string(),
        });
        assert!(!generic_read.authorizes(&lookup));
        assert!(connect_capability.authorizes(&lookup));

        let list = operation_for(&ClientRpcRequest::NetList { tag_filter: None });
        assert!(!generic_read.authorizes(&list));
        assert!(admin_capability.authorizes(&list));
    }
}
