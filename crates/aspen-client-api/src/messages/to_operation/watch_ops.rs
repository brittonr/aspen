use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;
use super::scan_operation_for_prefix;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Watch operations
        ClientRpcRequest::WatchCreate { prefix, .. } => Some(Some(scan_operation_for_prefix(prefix))),
        ClientRpcRequest::WatchCancel { .. } | ClientRpcRequest::WatchStatus { .. } => Some(None),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use aspen_auth_core::Capability;

    use super::*;

    #[test]
    fn snix_watch_prefix_requires_snix_read() {
        let request = ClientRpcRequest::WatchCreate {
            prefix: "snix:pathinfo:".to_string(),
            start_index: 0,
            should_include_prev_value: false,
        };

        let Some(Some(operation)) = to_operation(&request) else {
            panic!("snix watch should require authorization");
        };

        assert!(matches!(&operation, Operation::SnixRead { resource } if resource == "pathinfo:"));
        assert!(
            !Capability::Read {
                prefix: "snix:".to_string()
            }
            .authorizes(&operation)
        );
        assert!(
            Capability::SnixRead {
                resource_prefix: "pathinfo:".to_string(),
            }
            .authorizes(&operation)
        );
    }

    #[test]
    fn broad_snix_watch_prefix_requires_admin() {
        let request = ClientRpcRequest::WatchCreate {
            prefix: "snix:".to_string(),
            start_index: 0,
            should_include_prev_value: false,
        };

        let Some(Some(operation)) = to_operation(&request) else {
            panic!("broad snix watch should require authorization");
        };

        assert!(matches!(&operation, Operation::ClusterAdmin { action } if action == "reserved_snix_scan"));
        assert!(
            !Capability::Read {
                prefix: "snix:".to_string()
            }
            .authorizes(&operation)
        );
        assert!(Capability::ClusterAdmin.authorizes(&operation));
    }
}
