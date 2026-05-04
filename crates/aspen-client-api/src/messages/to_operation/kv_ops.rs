use alloc::format;
use alloc::string::ToString;
use alloc::vec;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;
use super::scan_operation_for_prefix;
use super::snix_operation_for_read_key;
use super::snix_operation_for_write_key;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Key-value read operations. SNIX-reserved key prefixes are store metadata,
        // so even generic KV RPCs over those prefixes require SNIX-specific auth.
        ClientRpcRequest::ReadKey { key } | ClientRpcRequest::HashCheck { key, .. } => {
            Some(Some(snix_operation_for_read_key(key)))
        }
        ClientRpcRequest::ScanKeys { prefix, .. } => Some(Some(scan_operation_for_prefix(prefix))),
        ClientRpcRequest::GetVaultKeys { vault_name: key } => Some(Some(Operation::Read { key: key.clone() })),

        // Key-value write operations. SNIX-reserved key prefixes are store metadata,
        // so even generic KV RPCs over those prefixes require SNIX-specific auth.
        ClientRpcRequest::WriteKey { key, value } | ClientRpcRequest::WriteKeyWithLease { key, value, .. } => {
            Some(Some(snix_operation_for_write_key(key, value.clone())))
        }
        ClientRpcRequest::DeleteKey { key }
        | ClientRpcRequest::CompareAndSwapKey { key, .. }
        | ClientRpcRequest::CompareAndDeleteKey { key, .. } => Some(Some(snix_operation_for_write_key(key, vec![]))),

        // Index read operations
        ClientRpcRequest::IndexScan { .. } | ClientRpcRequest::IndexList => Some(Some(Operation::Read {
            key: "_sys:index:".to_string(),
        })),

        // Index write operations
        ClientRpcRequest::IndexCreate { name, .. } | ClientRpcRequest::IndexDrop { name, .. } => {
            Some(Some(Operation::Write {
                key: format!("_sys:index:{}", name),
                value: vec![],
            }))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use aspen_auth_core::Capability;

    use super::*;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).flatten().expect("request should require auth")
    }

    #[test]
    fn snix_prefixed_generic_kv_reads_require_snix_read() {
        let read = operation_for(&ClientRpcRequest::ReadKey {
            key: "snix:dir:abc".to_string(),
        });
        assert!(matches!(read, Operation::SnixRead { resource } if resource == "dir:abc"));

        let scan = operation_for(&ClientRpcRequest::ScanKeys {
            prefix: "snix:pathinfo:".to_string(),
            limit: Some(100),
            continuation_token: None,
        });
        assert!(matches!(scan, Operation::SnixRead { resource } if resource == "pathinfo:"));

        let hash_check = operation_for(&ClientRpcRequest::HashCheck {
            key: "snix:pathinfo:def".to_string(),
            expected_hash: [0; 32],
        });
        assert!(matches!(hash_check, Operation::SnixRead { resource } if resource == "pathinfo:def"));
    }

    #[test]
    fn snix_prefixed_generic_kv_writes_require_snix_write() {
        let write = operation_for(&ClientRpcRequest::WriteKey {
            key: "snix:dir:abc".to_string(),
            value: b"encoded-dir".to_vec(),
        });
        assert!(matches!(write, Operation::SnixWrite { resource } if resource == "dir:abc"));

        let delete = operation_for(&ClientRpcRequest::DeleteKey {
            key: "snix:pathinfo:def".to_string(),
        });
        assert!(matches!(delete, Operation::SnixWrite { resource } if resource == "pathinfo:def"));
    }

    #[test]
    fn generic_kv_scopes_do_not_authorize_snix_prefixed_scan() {
        let generic_full = Capability::Full {
            prefix: "snix:".to_string(),
        };
        let generic_read = Capability::Read {
            prefix: "snix:".to_string(),
        };
        let snix_read = Capability::SnixRead {
            resource_prefix: "pathinfo:".to_string(),
        };

        let operation = operation_for(&ClientRpcRequest::ScanKeys {
            prefix: "snix:pathinfo:".to_string(),
            limit: Some(100),
            continuation_token: None,
        });

        assert!(!generic_full.authorizes(&operation));
        assert!(!generic_read.authorizes(&operation));
        assert!(snix_read.authorizes(&operation));
    }

    #[test]
    fn generic_kv_operations_over_internal_prefixes_require_admin() {
        let requests = [
            ClientRpcRequest::ReadKey {
                key: "_ci:runs:run-1".to_string(),
            },
            ClientRpcRequest::HashCheck {
                key: "_secrets:kv:prod/db".to_string(),
                expected_hash: [0; 32],
            },
            ClientRpcRequest::WriteKey {
                key: "_lease:tenant".to_string(),
                value: alloc::vec![1],
            },
            ClientRpcRequest::DeleteKey {
                key: "__worker:worker-1:jobs".to_string(),
            },
            ClientRpcRequest::CompareAndSwapKey {
                key: "/_sys/net/svc/api".to_string(),
                expected: None,
                new_value: alloc::vec![1],
            },
            ClientRpcRequest::CompareAndDeleteKey {
                key: "_cache:narinfo:hash".to_string(),
                expected: alloc::vec![1],
            },
        ];

        for request in requests {
            let operation = operation_for(&request);
            assert!(
                matches!(&operation, Operation::ClusterAdmin { action } if action == "reserved_internal_kv"),
                "request {request:?} produced {operation:?}"
            );
            assert!(!Capability::Full { prefix: String::new() }.authorizes(&operation));
            assert!(!Capability::Read { prefix: String::new() }.authorizes(&operation));
            assert!(!Capability::Write { prefix: String::new() }.authorizes(&operation));
            assert!(Capability::ClusterAdmin.authorizes(&operation));
        }
    }

    #[test]
    fn broad_scans_overlapping_internal_namespaces_require_admin() {
        for prefix in ["", "_", "_ci", "_ci:", "_secrets:", "__worker:", "/_sys/"] {
            let operation = operation_for(&ClientRpcRequest::ScanKeys {
                prefix: prefix.to_string(),
                limit: Some(100),
                continuation_token: None,
            });

            assert!(
                matches!(&operation, Operation::ClusterAdmin { action } if action == "reserved_internal_scan"),
                "prefix {prefix:?} produced {operation:?}"
            );
            assert!(!Capability::Full { prefix: String::new() }.authorizes(&operation));
            assert!(!Capability::Read { prefix: String::new() }.authorizes(&operation));
            assert!(Capability::ClusterAdmin.authorizes(&operation));
        }
    }

    #[test]
    fn broad_scans_overlapping_snix_namespaces_require_admin() {
        for prefix in ["snix", "snix:", "snix:p", "snix:path"] {
            let operation = operation_for(&ClientRpcRequest::ScanKeys {
                prefix: prefix.to_string(),
                limit: Some(100),
                continuation_token: None,
            });

            assert!(
                matches!(&operation, Operation::ClusterAdmin { action } if action == "reserved_snix_scan"),
                "prefix {prefix:?} produced {operation:?}"
            );
            assert!(!Capability::Full { prefix: String::new() }.authorizes(&operation));
            assert!(!Capability::Read { prefix: String::new() }.authorizes(&operation));
            assert!(
                !Capability::SnixRead {
                    resource_prefix: String::new()
                }
                .authorizes(&operation)
            );
            assert!(Capability::ClusterAdmin.authorizes(&operation));
        }
    }
}
