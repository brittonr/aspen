use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Secrets KV v2 read operations
        ClientRpcRequest::SecretsKvRead { mount, path, .. }
        | ClientRpcRequest::SecretsKvList { mount, path }
        | ClientRpcRequest::SecretsKvMetadata { mount, path } => Some(Some(Operation::Read {
            key: format!("_secrets:{}:{}", mount, path),
        })),

        // Secrets KV v2 write operations
        ClientRpcRequest::SecretsKvWrite { mount, path, .. }
        | ClientRpcRequest::SecretsKvDelete { mount, path, .. }
        | ClientRpcRequest::SecretsKvDestroy { mount, path, .. }
        | ClientRpcRequest::SecretsKvUndelete { mount, path, .. }
        | ClientRpcRequest::SecretsKvUpdateMetadata { mount, path, .. }
        | ClientRpcRequest::SecretsKvDeleteMetadata { mount, path } => Some(Some(Operation::Write {
            key: format!("_secrets:{}:{}", mount, path),
            value: vec![],
        })),

        // Secrets Transit operations
        ClientRpcRequest::SecretsTransitListKeys { mount } => Some(Some(Operation::Read {
            key: format!("_secrets:{}:", mount),
        })),
        ClientRpcRequest::SecretsTransitCreateKey { mount, name, .. }
        | ClientRpcRequest::SecretsTransitRotateKey { mount, name } => Some(Some(Operation::Write {
            key: format!("_secrets:{}:{}", mount, name),
            value: vec![],
        })),
        ClientRpcRequest::SecretsTransitEncrypt { mount, name, .. }
        | ClientRpcRequest::SecretsTransitDecrypt { mount, name, .. }
        | ClientRpcRequest::SecretsTransitSign { mount, name, .. }
        | ClientRpcRequest::SecretsTransitVerify { mount, name, .. }
        | ClientRpcRequest::SecretsTransitRewrap { mount, name, .. }
        | ClientRpcRequest::SecretsTransitDatakey { mount, name, .. } => Some(Some(Operation::Read {
            key: format!("_secrets:{}:{}", mount, name),
        })),

        // Secrets PKI operations
        ClientRpcRequest::SecretsPkiGetCrl { mount }
        | ClientRpcRequest::SecretsPkiListCerts { mount }
        | ClientRpcRequest::SecretsPkiListRoles { mount } => Some(Some(Operation::Read {
            key: format!("_secrets:{}:", mount),
        })),
        ClientRpcRequest::SecretsPkiGetRole { mount, name } => Some(Some(Operation::Read {
            key: format!("_secrets:{}:{}", mount, name),
        })),
        ClientRpcRequest::SecretsPkiGenerateRoot { mount, .. }
        | ClientRpcRequest::SecretsPkiGenerateIntermediate { mount, .. }
        | ClientRpcRequest::SecretsPkiSetSignedIntermediate { mount, .. }
        | ClientRpcRequest::SecretsPkiCreateRole { mount, .. }
        | ClientRpcRequest::SecretsPkiIssue { mount, .. }
        | ClientRpcRequest::SecretsPkiRevoke { mount, .. } => Some(Some(Operation::Write {
            key: format!("_secrets:{}:", mount),
            value: vec![],
        })),

        // Nix cache signing key operations
        ClientRpcRequest::SecretsNixCacheGetPublicKey { mount, .. } => Some(Some(Operation::Read {
            key: format!("_secrets:{}:", mount),
        })),
        ClientRpcRequest::SecretsNixCacheListKeys { mount } => Some(Some(Operation::Read {
            key: format!("_secrets:{}:", mount),
        })),
        ClientRpcRequest::SecretsNixCacheCreateKey { mount, .. }
        | ClientRpcRequest::SecretsNixCacheRotateKey { mount, .. }
        | ClientRpcRequest::SecretsNixCacheDeleteKey { mount, .. } => Some(Some(Operation::Write {
            key: format!("_secrets:{}:", mount),
            value: vec![],
        })),

        _ => None,
    }
}
