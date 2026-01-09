# SOPS Phase 3: Secrets API Design

**Date**: 2026-01-09T16:30:00-05:00
**Status**: In Progress
**Author**: Claude (AI Assistant)

---

## Executive Summary

Phase 3 of SOPS integration adds client-facing RPC handlers and CLI commands for the three secrets engines (KV v2, Transit, PKI) implemented in Phase 2. This enables runtime secrets management through the standard Aspen client interface.

## Background

### Previous Phases

**Phase 1** (c39b8082): SOPS Foundation

- SOPS age decryption
- Bootstrap secrets loading
- SecretsProvider trait
- Capability system integration

**Phase 2** (4d927c55): Vault-compatible Engines

- KV v2: Versioned secrets with soft/hard delete
- Transit: Encryption-as-a-service
- PKI: Certificate authority
- SecretsBackend abstraction

### Current State

The secrets engines are fully implemented but not exposed via the client RPC API. They can only be used programmatically by code with direct access to the `KvStore`, `TransitStore`, and `PkiStore` traits.

## Design Goals

1. **Vault-compatible API patterns** - Follow HashiCorp Vault's API design for familiarity
2. **Capability-based authorization** - Require appropriate auth tokens for operations
3. **Tiger Style compliance** - Bounded operations, explicit errors, no panics
4. **Iroh-native transport** - Use existing ClientRpcRequest/Response patterns
5. **CLI integration** - Provide `aspen-cli secrets` subcommands

## Architecture

### RPC Protocol Extension

Add new `ClientRpcRequest` variants for secrets operations:

```rust
// KV v2 Secrets Engine
SecretsKvRead { mount: String, path: String, version: Option<u64> },
SecretsKvWrite { mount: String, path: String, data: HashMap<String, String>, cas: Option<u64> },
SecretsKvDelete { mount: String, path: String, versions: Vec<u64> },
SecretsKvDestroy { mount: String, path: String, versions: Vec<u64> },
SecretsKvUndelete { mount: String, path: String, versions: Vec<u64> },
SecretsKvList { mount: String, path: String },
SecretsKvMetadata { mount: String, path: String },
SecretsKvUpdateMetadata { mount: String, path: String, max_versions: Option<u32>, cas_required: Option<bool> },
SecretsKvDeleteMetadata { mount: String, path: String },

// Transit Secrets Engine
SecretsTransitCreateKey { mount: String, name: String, key_type: String },
SecretsTransitEncrypt { mount: String, name: String, plaintext: Vec<u8>, context: Option<Vec<u8>> },
SecretsTransitDecrypt { mount: String, name: String, ciphertext: String, context: Option<Vec<u8>> },
SecretsTransitSign { mount: String, name: String, data: Vec<u8> },
SecretsTransitVerify { mount: String, name: String, data: Vec<u8>, signature: String },
SecretsTransitRotateKey { mount: String, name: String },
SecretsTransitListKeys { mount: String },
SecretsTransitRewrap { mount: String, name: String, ciphertext: String, context: Option<Vec<u8>> },
SecretsTransitDatakey { mount: String, name: String, key_type: String },

// PKI Secrets Engine
SecretsPkiGenerateRoot { mount: String, common_name: String, ttl_days: Option<u32> },
SecretsPkiGenerateIntermediate { mount: String, common_name: String },
SecretsPkiSetSignedIntermediate { mount: String, certificate: String },
SecretsPkiCreateRole { mount: String, name: String, allowed_domains: Vec<String>, max_ttl_days: u32 },
SecretsPkiIssue { mount: String, role: String, common_name: String, alt_names: Vec<String>, ttl_days: Option<u32> },
SecretsPkiRevoke { mount: String, serial: String },
SecretsPkiGetCrl { mount: String },
SecretsPkiListCerts { mount: String },
```

### Response Types

```rust
// KV v2 Responses
SecretsKvReadResponse { data: HashMap<String, String>, metadata: VersionMetadata },
SecretsKvWriteResponse { version: u64 },
SecretsKvListResponse { keys: Vec<String> },
SecretsKvMetadataResponse { current_version: u64, versions: Vec<VersionInfo> },

// Transit Responses
SecretsTransitEncryptResponse { ciphertext: String },
SecretsTransitDecryptResponse { plaintext: Vec<u8> },
SecretsTransitSignResponse { signature: String },
SecretsTransitVerifyResponse { valid: bool },
SecretsTransitDatakeyResponse { plaintext: Option<Vec<u8>>, ciphertext: String },

// PKI Responses
SecretsPkiCertificateResponse { certificate: String, private_key: Option<String>, serial: String },
SecretsPkiCrlResponse { crl: String },
SecretsPkiListResponse { serials: Vec<String> },
```

### Handler Implementation

New `SecretsHandler` in `crates/aspen-rpc-handlers/src/handlers/secrets.rs`:

```rust
pub struct SecretsHandler;

impl RequestHandler for SecretsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request,
            ClientRpcRequest::SecretsKvRead { .. } |
            ClientRpcRequest::SecretsKvWrite { .. } |
            // ... all secrets variants
        )
    }

    async fn handle(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext)
        -> anyhow::Result<ClientRpcResponse>
    {
        // Dispatch to appropriate engine handler
    }
}
```

### Capability Requirements

| Operation | Required Capability |
| --------- | ------------------- |
| SecretsKvRead | SecretsRead |
| SecretsKvWrite | SecretsWrite |
| SecretsKvDelete/Destroy | SecretsWrite |
| SecretsTransitEncrypt | TransitEncrypt |
| SecretsTransitDecrypt | TransitDecrypt |
| SecretsTransitSign | TransitSign |
| SecretsTransitVerify | TransitVerify |
| SecretsPkiIssue | PkiIssue |
| SecretsPkiRevoke | PkiRevoke |
| SecretsPkiGenerateRoot | SecretsAdmin |

### CLI Commands

```bash
# KV v2
aspen-cli secrets kv get secret/myapp/config
aspen-cli secrets kv put secret/myapp/config username=admin password=secret
aspen-cli secrets kv delete secret/myapp/config --versions=1,2
aspen-cli secrets kv destroy secret/myapp/config --versions=1
aspen-cli secrets kv undelete secret/myapp/config --versions=1
aspen-cli secrets kv list secret/
aspen-cli secrets kv metadata get secret/myapp/config
aspen-cli secrets kv metadata delete secret/myapp/config

# Transit
aspen-cli secrets transit create-key mykey --type=aes256-gcm
aspen-cli secrets transit encrypt mykey --plaintext="hello world"
aspen-cli secrets transit decrypt mykey --ciphertext="vault:v1:..."
aspen-cli secrets transit sign mykey --data="message to sign"
aspen-cli secrets transit verify mykey --data="message" --signature="..."
aspen-cli secrets transit rotate mykey
aspen-cli secrets transit list

# PKI
aspen-cli secrets pki generate-root --common-name="My CA" --ttl=3650
aspen-cli secrets pki create-role web --allowed-domains=example.com --max-ttl=90
aspen-cli secrets pki issue web --common-name=web.example.com --alt-names=www.example.com
aspen-cli secrets pki revoke --serial=12:34:56:78
aspen-cli secrets pki crl
aspen-cli secrets pki list-certs
```

## Implementation Plan

### Files to Create/Modify

1. **aspen-client-api/src/messages.rs**
   - Add 30+ new `ClientRpcRequest` variants
   - Add corresponding response types

2. **aspen-rpc-handlers/src/handlers/secrets.rs** (NEW)
   - SecretsHandler implementation (~500 lines)

3. **aspen-rpc-handlers/src/handlers/mod.rs**
   - Add `secrets` module with feature gate

4. **aspen-rpc-handlers/src/registry.rs**
   - Register SecretsHandler

5. **aspen-cli/src/bin/aspen-cli/commands/secrets.rs** (NEW)
   - CLI subcommands (~800 lines)

6. **aspen-cli/src/bin/aspen-cli/commands/mod.rs**
   - Add `secrets` module

7. **aspen-cluster/src/bootstrap.rs**
   - Wire secrets service into node context

8. **Cargo.toml files**
   - Add `secrets` feature flag
   - Add aspen-secrets dependency where needed

### Tiger Style Resource Bounds

All operations inherit limits from aspen-secrets:

- MAX_SECRET_PATH_LENGTH = 512
- MAX_KV_SECRET_SIZE = 128KB
- MAX_VERSIONS_PER_SECRET = 100
- MAX_TRANSIT_KEY_VERSIONS = 100
- MAX_PKI_SANS = 100

### Error Handling

Follow existing error sanitization pattern:

- Internal errors return generic "secrets operation failed" messages
- Specific errors (not found, version conflict) return details
- Capability errors return "unauthorized" with required capability

## Security Considerations

1. **Authorization**: All secrets operations require appropriate capability tokens
2. **Audit logging**: Add tracing spans for secrets operations
3. **Error sanitization**: Don't leak secret values in error messages
4. **Path validation**: Prevent traversal attacks in secret paths

## Testing Strategy

1. Unit tests in `secrets.rs` handler
2. Integration test in `tests/secrets_integration_test.rs`
3. CLI tests via `scripts/kitty-secrets-test.sh`
4. Property-based tests for path validation

## Success Metrics

- All 30+ RPC variants implemented and tested
- CLI commands functional
- Integration with existing auth system
- No panics or unwraps in production code

## References

- [HashiCorp Vault KV v2 API](https://developer.hashicorp.com/vault/api-docs/secret/kv/kv-v2)
- [HashiCorp Vault Transit API](https://developer.hashicorp.com/vault/api-docs/secret/transit)
- [HashiCorp Vault PKI API](https://developer.hashicorp.com/vault/api-docs/secret/pki)
- [Rust secrecy crate](https://docs.rs/secrecy/latest/secrecy/)
- Phase 1 commit: c39b8082
- Phase 2 commit: 4d927c55
