use alloc::format;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Hook operations (read-only metadata access)
        ClientRpcRequest::HookList => Some(Some(Operation::HooksRead {
            resource: "hook:".to_string(),
        })),
        ClientRpcRequest::HookGetMetrics { handler_name } => Some(Some(Operation::HooksRead {
            resource: match handler_name.as_deref() {
                Some(handler_name) => format!("hook:{handler_name}:metrics"),
                None => "hook:metrics".to_string(),
            },
        })),
        ClientRpcRequest::HookTrigger { event_type, .. } => Some(Some(Operation::HooksWrite {
            resource: format!("event:{event_type}"),
        })),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use aspen_auth_core::Capability;

    use crate::messages::ClientRpcRequest;

    fn op(request: &ClientRpcRequest) -> aspen_auth_core::Operation {
        super::to_operation(request).flatten().expect("protected hook request")
    }

    #[test]
    fn hook_requests_use_hook_capabilities_not_generic_prefixes() {
        let read = op(&ClientRpcRequest::HookList);
        let write = op(&ClientRpcRequest::HookTrigger {
            event_type: "write_committed".to_string(),
            payload_json: "{}".to_string(),
        });

        assert!(
            Capability::HooksRead {
                resource_prefix: "hook:".to_string(),
            }
            .authorizes(&read)
        );
        assert!(
            Capability::HooksWrite {
                resource_prefix: "event:".to_string(),
            }
            .authorizes(&write)
        );
    }

    #[test]
    fn generic_hook_prefixes_do_not_authorize_hook_requests() {
        let generic_read = Capability::Read {
            prefix: "_hooks:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "_hooks:".to_string(),
        };
        let read = op(&ClientRpcRequest::HookList);
        let write = op(&ClientRpcRequest::HookTrigger {
            event_type: "write_committed".to_string(),
            payload_json: "{}".to_string(),
        });

        assert!(!generic_read.authorizes(&read));
        assert!(!generic_write.authorizes(&write));
    }
}
