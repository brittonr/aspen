use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    to_operation_kv_transit(request).or_else(|| to_operation_pki_nix_cache(request))
}

struct SecretPath<'a> {
    mount: &'a str,
    path: &'a str,
}

struct SecretKey<'a> {
    mount: &'a str,
    name: &'a str,
}

fn secret_resource(resource: SecretPath<'_>) -> Operation {
    Operation::SecretsRead {
        mount: resource.mount.to_string(),
        path: resource.path.to_string(),
    }
}

fn secret_write(resource: SecretPath<'_>) -> Operation {
    Operation::SecretsWrite {
        mount: resource.mount.to_string(),
        path: resource.path.to_string(),
    }
}

fn secret_delete(resource: SecretPath<'_>) -> Operation {
    Operation::SecretsDelete {
        mount: resource.mount.to_string(),
        path: resource.path.to_string(),
    }
}

fn secret_list(resource: SecretPath<'_>) -> Operation {
    Operation::SecretsList {
        mount: resource.mount.to_string(),
        path: resource.path.to_string(),
    }
}

fn scoped_secret_name(key: SecretKey<'_>) -> String {
    let mount = key.mount;
    let name = key.name;
    format!("{mount}:{name}")
}

fn to_operation_kv_transit(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::SecretsKvRead { mount, path, .. } | ClientRpcRequest::SecretsKvMetadata { mount, path } => {
            Some(Some(secret_resource(SecretPath { mount, path })))
        }
        ClientRpcRequest::SecretsKvList { mount, path } => Some(Some(secret_list(SecretPath { mount, path }))),

        ClientRpcRequest::SecretsKvWrite { mount, path, .. }
        | ClientRpcRequest::SecretsKvUndelete { mount, path, .. }
        | ClientRpcRequest::SecretsKvUpdateMetadata { mount, path, .. } => {
            Some(Some(secret_write(SecretPath { mount, path })))
        }
        ClientRpcRequest::SecretsKvDelete { mount, path, .. }
        | ClientRpcRequest::SecretsKvDestroy { mount, path, .. }
        | ClientRpcRequest::SecretsKvDeleteMetadata { mount, path } => {
            Some(Some(secret_delete(SecretPath { mount, path })))
        }

        ClientRpcRequest::SecretsTransitListKeys { mount } => Some(Some(secret_list(SecretPath { mount, path: "" }))),
        ClientRpcRequest::SecretsTransitCreateKey { mount, name, .. }
        | ClientRpcRequest::SecretsTransitRotateKey { mount, name } => Some(Some(Operation::TransitKeyManage {
            key_name: scoped_secret_name(SecretKey { mount, name }),
        })),
        ClientRpcRequest::SecretsTransitEncrypt { mount, name, .. } => Some(Some(Operation::TransitEncrypt {
            key_name: scoped_secret_name(SecretKey { mount, name }),
        })),
        ClientRpcRequest::SecretsTransitDecrypt { mount, name, .. }
        | ClientRpcRequest::SecretsTransitRewrap { mount, name, .. }
        | ClientRpcRequest::SecretsTransitDatakey { mount, name, .. } => Some(Some(Operation::TransitDecrypt {
            key_name: scoped_secret_name(SecretKey { mount, name }),
        })),
        ClientRpcRequest::SecretsTransitSign { mount, name, .. } => Some(Some(Operation::TransitSign {
            key_name: scoped_secret_name(SecretKey { mount, name }),
        })),
        ClientRpcRequest::SecretsTransitVerify { mount, name, .. } => Some(Some(Operation::TransitVerify {
            key_name: scoped_secret_name(SecretKey { mount, name }),
        })),

        _ => None,
    }
}

fn to_operation_pki_nix_cache(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::SecretsPkiGetCrl { .. } | ClientRpcRequest::SecretsPkiListCerts { .. } => {
            Some(Some(Operation::PkiReadCa))
        }
        ClientRpcRequest::SecretsPkiListRoles { mount } => {
            Some(Some(secret_list(SecretPath { mount, path: "roles/" })))
        }
        ClientRpcRequest::SecretsPkiGetRole { mount, name } => {
            Some(Some(secret_resource(SecretPath { mount, path: name })))
        }
        ClientRpcRequest::SecretsPkiGenerateRoot { .. }
        | ClientRpcRequest::SecretsPkiGenerateIntermediate { .. }
        | ClientRpcRequest::SecretsPkiSetSignedIntermediate { .. }
        | ClientRpcRequest::SecretsPkiCreateRole { .. } => Some(Some(Operation::PkiManage)),
        ClientRpcRequest::SecretsPkiIssue { mount, role, .. } => Some(Some(Operation::PkiIssue {
            role: scoped_secret_name(SecretKey { mount, name: role }),
        })),
        ClientRpcRequest::SecretsPkiRevoke { .. } => Some(Some(Operation::PkiRevoke)),

        ClientRpcRequest::SecretsNixCacheGetPublicKey { mount, cache_name } => Some(Some(Operation::TransitVerify {
            key_name: scoped_secret_name(SecretKey {
                mount,
                name: cache_name,
            }),
        })),
        ClientRpcRequest::SecretsNixCacheListKeys { mount } => Some(Some(secret_list(SecretPath { mount, path: "" }))),
        ClientRpcRequest::SecretsNixCacheCreateKey { mount, cache_name }
        | ClientRpcRequest::SecretsNixCacheRotateKey { mount, cache_name }
        | ClientRpcRequest::SecretsNixCacheDeleteKey { mount, cache_name } => Some(Some(Operation::TransitKeyManage {
            key_name: scoped_secret_name(SecretKey {
                mount,
                name: cache_name,
            }),
        })),

        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use alloc::vec;

    use aspen_auth_core::Capability;

    use super::*;

    fn operation_for(request: &ClientRpcRequest) -> Operation {
        to_operation(request).flatten().expect("request should require auth")
    }

    fn assert_not_generic_data_authorized(operation: &Operation) {
        for capability in [
            Capability::Read {
                prefix: "_secrets:".to_string(),
            },
            Capability::Write {
                prefix: "_secrets:".to_string(),
            },
            Capability::Full {
                prefix: "_secrets:".to_string(),
            },
        ] {
            assert!(!capability.authorizes(operation));
        }
    }

    #[test]
    fn secrets_kv_requests_use_secrets_capabilities_not_generic_prefixes() {
        let read = operation_for(&ClientRpcRequest::SecretsKvRead {
            mount: "secret".to_string(),
            path: "apps/api".to_string(),
            version: None,
        });
        assert!(matches!(&read, Operation::SecretsRead { mount, path } if mount == "secret" && path == "apps/api"));
        assert_not_generic_data_authorized(&read);
        assert!(
            Capability::SecretsRead {
                mount: "secret".to_string(),
                prefix: "apps/".to_string(),
            }
            .authorizes(&read)
        );

        let list = operation_for(&ClientRpcRequest::SecretsKvList {
            mount: "secret".to_string(),
            path: "apps/".to_string(),
        });
        assert!(matches!(&list, Operation::SecretsList { mount, path } if mount == "secret" && path == "apps/"));
        assert_not_generic_data_authorized(&list);
        assert!(
            Capability::SecretsList {
                mount: "secret".to_string(),
                prefix: "apps/".to_string(),
            }
            .authorizes(&list)
        );

        let write = operation_for(&ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "apps/api".to_string(),
            data: BTreeMap::new(),
            cas: None,
        });
        assert!(matches!(&write, Operation::SecretsWrite { mount, path } if mount == "secret" && path == "apps/api"));
        assert_not_generic_data_authorized(&write);
        assert!(
            Capability::SecretsWrite {
                mount: "secret".to_string(),
                prefix: "apps/".to_string(),
            }
            .authorizes(&write)
        );

        let delete = operation_for(&ClientRpcRequest::SecretsKvDeleteMetadata {
            mount: "secret".to_string(),
            path: "apps/api".to_string(),
        });
        assert!(matches!(&delete, Operation::SecretsDelete { mount, path } if mount == "secret" && path == "apps/api"));
        assert_not_generic_data_authorized(&delete);
        assert!(
            Capability::SecretsDelete {
                mount: "secret".to_string(),
                prefix: "apps/".to_string(),
            }
            .authorizes(&delete)
        );
    }

    #[test]
    fn transit_and_nix_cache_requests_use_transit_capabilities() {
        let encrypt = operation_for(&ClientRpcRequest::SecretsTransitEncrypt {
            mount: "transit".to_string(),
            name: "api-key".to_string(),
            plaintext: vec![1, 2, 3],
            context: None,
        });
        assert!(matches!(&encrypt, Operation::TransitEncrypt { key_name } if key_name == "transit:api-key"));
        assert_not_generic_data_authorized(&encrypt);
        assert!(
            Capability::TransitEncrypt {
                key_prefix: "transit:api".to_string(),
            }
            .authorizes(&encrypt)
        );

        let datakey = operation_for(&ClientRpcRequest::SecretsTransitDatakey {
            mount: "transit".to_string(),
            name: "api-key".to_string(),
            key_type: "wrapped".to_string(),
        });
        assert!(matches!(&datakey, Operation::TransitDecrypt { key_name } if key_name == "transit:api-key"));
        assert_not_generic_data_authorized(&datakey);
        assert!(
            Capability::TransitDecrypt {
                key_prefix: "transit:api".to_string(),
            }
            .authorizes(&datakey)
        );

        let rotate = operation_for(&ClientRpcRequest::SecretsNixCacheRotateKey {
            mount: "nix-cache".to_string(),
            cache_name: "cache.example".to_string(),
        });
        assert!(matches!(&rotate, Operation::TransitKeyManage { key_name } if key_name == "nix-cache:cache.example"));
        assert_not_generic_data_authorized(&rotate);
        assert!(
            Capability::TransitKeyManage {
                key_prefix: "nix-cache:cache".to_string(),
            }
            .authorizes(&rotate)
        );
    }

    #[test]
    fn pki_requests_use_pki_capabilities_not_generic_prefixes() {
        let issue = operation_for(&ClientRpcRequest::SecretsPkiIssue {
            mount: "pki".to_string(),
            role: "web".to_string(),
            common_name: "service.internal".to_string(),
            alt_names: vec!["service.internal".to_string()],
            ttl_days: Some(30),
        });
        assert!(matches!(&issue, Operation::PkiIssue { role } if role == "pki:web"));
        assert_not_generic_data_authorized(&issue);
        assert!(
            Capability::PkiIssue {
                role_prefix: "pki:w".to_string(),
            }
            .authorizes(&issue)
        );

        let create_role = operation_for(&ClientRpcRequest::SecretsPkiCreateRole {
            mount: "pki".to_string(),
            name: "web".to_string(),
            allowed_domains: vec!["service.internal".to_string()],
            max_ttl_days: 30,
            allow_bare_domains: true,
            allow_wildcards: false,
            allow_subdomains: true,
        });
        assert!(matches!(create_role, Operation::PkiManage));

        let revoke = operation_for(&ClientRpcRequest::SecretsPkiRevoke {
            mount: "pki".to_string(),
            serial: "01:02".to_string(),
        });
        assert!(matches!(revoke, Operation::PkiRevoke));
    }
}
