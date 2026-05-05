use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Lease operations
        ClientRpcRequest::LeaseGrant { .. }
        | ClientRpcRequest::LeaseRevoke { .. }
        | ClientRpcRequest::LeaseKeepalive { .. } => Some(Some(Operation::CoordinationWrite {
            resource: "lease:".to_string(),
        })),
        ClientRpcRequest::LeaseTimeToLive { .. } | ClientRpcRequest::LeaseList => {
            Some(Some(Operation::CoordinationRead {
                resource: "lease:".to_string(),
            }))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use aspen_auth_core::Capability;

    use super::*;

    #[test]
    fn lease_requests_use_domain_specific_capabilities() {
        let read = to_operation(&ClientRpcRequest::LeaseList).flatten().unwrap();
        let write = to_operation(&ClientRpcRequest::LeaseGrant {
            ttl_seconds: 30,
            lease_id: None,
        })
        .flatten()
        .unwrap();

        assert!(matches!(&read, Operation::CoordinationRead { resource } if resource == "lease:"));
        assert!(matches!(&write, Operation::CoordinationWrite { resource } if resource == "lease:"));
        assert!(
            Capability::CoordinationRead {
                resource_prefix: "lease:".to_string(),
            }
            .authorizes(&read)
        );
        assert!(
            Capability::CoordinationWrite {
                resource_prefix: "lease:".to_string(),
            }
            .authorizes(&write)
        );
        assert!(
            !Capability::Read {
                prefix: "_lease:".to_string(),
            }
            .authorizes(&read)
        );
        assert!(
            !Capability::Write {
                prefix: "_lease:".to_string(),
            }
            .authorizes(&write)
        );
    }
}
