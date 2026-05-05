use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;
use super::scan_operation_for_prefix;
use super::snix_operation_for_read_key;
use super::snix_operation_for_write_key;

fn vault_resource(vault_name: &str) -> String {
    format!("vault:{vault_name}")
}

fn index_resource(name: &str) -> String {
    format!("index:{name}")
}

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Key-value read operations. SNIX-reserved key prefixes are store metadata,
        // so even generic KV RPCs over those prefixes require SNIX-specific auth.
        ClientRpcRequest::ReadKey { key } | ClientRpcRequest::HashCheck { key, .. } => {
            Some(Some(snix_operation_for_read_key(key)))
        }
        ClientRpcRequest::ScanKeys { prefix, .. } => Some(Some(scan_operation_for_prefix(prefix))),
        ClientRpcRequest::GetVaultKeys { vault_name } => Some(Some(Operation::KvMetadataRead {
            resource: vault_resource(vault_name),
        })),

        // Key-value write operations. SNIX-reserved key prefixes are store metadata,
        // so even generic KV RPCs over those prefixes require SNIX-specific auth.
        ClientRpcRequest::WriteKey { key, value } | ClientRpcRequest::WriteKeyWithLease { key, value, .. } => {
            Some(Some(snix_operation_for_write_key(key, value.clone())))
        }
        ClientRpcRequest::DeleteKey { key }
        | ClientRpcRequest::CompareAndSwapKey { key, .. }
        | ClientRpcRequest::CompareAndDeleteKey { key, .. } => Some(Some(snix_operation_for_write_key(key, vec![]))),

        // Index metadata operations
        ClientRpcRequest::IndexScan { index_name, .. } => Some(Some(Operation::KvMetadataRead {
            resource: index_resource(index_name),
        })),
        ClientRpcRequest::IndexList => Some(Some(Operation::KvMetadataRead {
            resource: "index:".to_string(),
        })),
        ClientRpcRequest::IndexCreate { name, .. } | ClientRpcRequest::IndexDrop { name, .. } => {
            Some(Some(Operation::KvMetadataWrite {
                resource: index_resource(name),
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
    fn kv_metadata_requests_use_domain_specific_capabilities() {
        let vault = operation_for(&ClientRpcRequest::GetVaultKeys {
            vault_name: "prod".to_string(),
        });
        assert!(matches!(vault, Operation::KvMetadataRead { resource } if resource == "vault:prod"));

        let index_scan = operation_for(&ClientRpcRequest::IndexScan {
            index_name: "by-owner".to_string(),
            mode: "exact".to_string(),
            value: "team-a".to_string(),
            end_value: None,
            limit: Some(10),
        });
        assert!(matches!(index_scan, Operation::KvMetadataRead { resource } if resource == "index:by-owner"));

        let index_list = operation_for(&ClientRpcRequest::IndexList);
        assert!(matches!(index_list, Operation::KvMetadataRead { resource } if resource == "index:"));

        let index_create = operation_for(&ClientRpcRequest::IndexCreate {
            name: "by-owner".to_string(),
            field: "owner".to_string(),
            field_type: "string".to_string(),
            is_unique: false,
            should_index_nulls: false,
        });
        assert!(matches!(index_create, Operation::KvMetadataWrite { resource } if resource == "index:by-owner"));
    }

    #[test]
    fn generic_kv_metadata_prefixes_do_not_authorize_metadata_requests() {
        let generic_vault_read = Capability::Read {
            prefix: "prod".to_string(),
        };
        let generic_index_read = Capability::Read {
            prefix: "_sys:index:".to_string(),
        };
        let generic_index_write = Capability::Write {
            prefix: "_sys:index:".to_string(),
        };
        let metadata_read = Capability::KvMetadataRead {
            resource_prefix: String::new(),
        };
        let metadata_write = Capability::KvMetadataWrite {
            resource_prefix: "index:".to_string(),
        };

        let vault = operation_for(&ClientRpcRequest::GetVaultKeys {
            vault_name: "prod".to_string(),
        });
        assert!(!generic_vault_read.authorizes(&vault));
        assert!(metadata_read.authorizes(&vault));

        let index_list = operation_for(&ClientRpcRequest::IndexList);
        assert!(!generic_index_read.authorizes(&index_list));
        assert!(metadata_read.authorizes(&index_list));

        let index_drop = operation_for(&ClientRpcRequest::IndexDrop {
            name: "by-owner".to_string(),
        });
        assert!(!generic_index_write.authorizes(&index_drop));
        assert!(metadata_write.authorizes(&index_drop));
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
