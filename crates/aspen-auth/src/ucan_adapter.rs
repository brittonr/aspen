//! Runtime UCAN adapter for Aspen capability documents.
//!
//! This module is the narrow shell boundary between Aspen's existing runtime auth
//! API and the sibling `ucan` crate. It deliberately does not change the legacy
//! `CapabilityToken` wire format yet; builder/verifier/RPC/CLI callers continue
//! to use the existing Aspen-facing types while this adapter validates the UCAN
//! resource/ability projection through the sibling implementation.

use aspen_auth_core::AuthError;
use aspen_auth_core::Capability;
use ucan::shell::CapabilityDocument;
use ucan::token::CapabilitySet;

const ASPEN_RESOURCE_SCHEME: &str = "aspen";

/// Convert a legacy Aspen capability into a validated sibling-UCAN capability
/// document.
///
/// The returned document uses the mapping recorded in
/// `openspec/changes/adopt-sibling-ucan-auth/evidence/i3-aspen-ucan-capability-mapping.md`.
/// UCAN validation is performed by `ucan::shell::CapabilityDocument::new`.
pub fn capability_to_ucan_document(capability: &Capability) -> Result<CapabilityDocument, AuthError> {
    let mapped = mapped_capability(capability);
    CapabilityDocument::new(mapped.resource, mapped.ability)
        .map_err(|error| AuthError::EncodingError(error.to_string()))
}

/// Convert a non-empty set of Aspen capabilities into the sibling UCAN
/// capability collection used by token issuance and authorization.
pub fn capabilities_to_ucan_set(capabilities: &[Capability]) -> Result<CapabilitySet, AuthError> {
    let documents = capabilities.iter().map(capability_to_ucan_document).collect::<Result<Vec<_>, _>>()?;
    CapabilitySet::new(documents).map_err(|error| AuthError::EncodingError(error.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MappedCapability {
    resource: String,
    ability: String,
}

fn mapped_capability(capability: &Capability) -> MappedCapability {
    match capability {
        Capability::Read { prefix } => scoped("kv", prefix, "kv/read"),
        Capability::Write { prefix } => scoped("kv", prefix, "kv/write"),
        Capability::Delete { prefix } => scoped("kv", prefix, "kv/delete"),
        Capability::Full { prefix } => scoped("kv", prefix, "kv/*"),
        Capability::Watch { prefix } => scoped("kv", prefix, "kv/watch"),
        Capability::ClusterAdmin => global("cluster", "cluster/admin"),
        Capability::Delegate => global("auth", "auth/delegate"),
        Capability::ShellExecute {
            command_pattern,
            working_dir,
        } => scoped("shell", &shell_scope(command_pattern, working_dir), "shell/execute"),
        Capability::SecretsRead { mount, prefix } => scoped("secrets", &mount_scope(mount, prefix), "secrets/read"),
        Capability::SecretsWrite { mount, prefix } => scoped("secrets", &mount_scope(mount, prefix), "secrets/write"),
        Capability::SecretsDelete { mount, prefix } => scoped("secrets", &mount_scope(mount, prefix), "secrets/delete"),
        Capability::SecretsList { mount, prefix } => scoped("secrets", &mount_scope(mount, prefix), "secrets/list"),
        Capability::SecretsFull { mount, prefix } => scoped("secrets", &mount_scope(mount, prefix), "secrets/*"),
        Capability::TransitEncrypt { key_prefix } => scoped("transit", key_prefix, "transit/encrypt"),
        Capability::TransitDecrypt { key_prefix } => scoped("transit", key_prefix, "transit/decrypt"),
        Capability::TransitSign { key_prefix } => scoped("transit", key_prefix, "transit/sign"),
        Capability::TransitVerify { key_prefix } => scoped("transit", key_prefix, "transit/verify"),
        Capability::TransitKeyManage { key_prefix } => scoped("transit", key_prefix, "transit/manage"),
        Capability::PkiIssue { role_prefix } => scoped("pki", role_prefix, "pki/issue"),
        Capability::PkiRevoke => global("pki", "pki/revoke"),
        Capability::PkiReadCa => global("pki", "pki/read-ca"),
        Capability::PkiManage => global("pki", "pki/manage"),
        Capability::SecretsAdmin => global("secrets", "secrets/admin"),
        Capability::NetConnect { service_prefix } => scoped("net", service_prefix, "net/connect"),
        Capability::NetPublish { service_prefix } => scoped("net", service_prefix, "net/publish"),
        Capability::NetAdmin => global("net", "net/admin"),
        Capability::CiRead { resource_prefix } => scoped("ci", resource_prefix, "ci/read"),
        Capability::CiWrite { resource_prefix } => scoped("ci", resource_prefix, "ci/write"),
        Capability::JobsRead { resource_prefix } => scoped("jobs", resource_prefix, "jobs/read"),
        Capability::JobsWrite { resource_prefix } => scoped("jobs", resource_prefix, "jobs/write"),
        Capability::BlobRead { resource_prefix } => scoped("blob", resource_prefix, "blob/read"),
        Capability::BlobWrite { resource_prefix } => scoped("blob", resource_prefix, "blob/write"),
        Capability::DocsRead { resource_prefix } => scoped("docs", resource_prefix, "docs/read"),
        Capability::DocsWrite { resource_prefix } => scoped("docs", resource_prefix, "docs/write"),
        Capability::HooksRead { resource_prefix } => scoped("hooks", resource_prefix, "hooks/read"),
        Capability::HooksWrite { resource_prefix } => scoped("hooks", resource_prefix, "hooks/write"),
        Capability::KvMetadataRead { resource_prefix } => scoped("kv-metadata", resource_prefix, "kv-metadata/read"),
        Capability::KvMetadataWrite { resource_prefix } => scoped("kv-metadata", resource_prefix, "kv-metadata/write"),
        Capability::CoordinationRead { resource_prefix } => {
            scoped("coordination", resource_prefix, "coordination/read")
        }
        Capability::CoordinationWrite { resource_prefix } => {
            scoped("coordination", resource_prefix, "coordination/write")
        }
        Capability::SqlRead { resource_prefix } => scoped("sql", resource_prefix, "sql/read"),
        Capability::ObservabilityRead { resource_prefix } => {
            scoped("observability", resource_prefix, "observability/read")
        }
        Capability::ObservabilityWrite { resource_prefix } => {
            scoped("observability", resource_prefix, "observability/write")
        }
        Capability::AutomergeRead { resource_prefix } => scoped("automerge", resource_prefix, "automerge/read"),
        Capability::AutomergeWrite { resource_prefix } => scoped("automerge", resource_prefix, "automerge/write"),
        Capability::FederationPull { repo_prefix } => scoped("federation", repo_prefix, "federation/pull"),
        Capability::FederationPush { repo_prefix } => scoped("federation", repo_prefix, "federation/push"),
        Capability::CacheRead { resource_prefix } => scoped("cache", resource_prefix, "cache/read"),
        Capability::CacheWrite { resource_prefix } => scoped("cache", resource_prefix, "cache/write"),
        Capability::SnixRead { resource_prefix } => scoped("snix", resource_prefix, "snix/read"),
        Capability::SnixWrite { resource_prefix } => scoped("snix", resource_prefix, "snix/write"),
    }
}

fn scoped(domain: &str, scope: &str, ability: &str) -> MappedCapability {
    MappedCapability {
        resource: format!("{ASPEN_RESOURCE_SCHEME}:{domain}:{scope}"),
        ability: ability.to_owned(),
    }
}

fn global(domain: &str, ability: &str) -> MappedCapability {
    scoped(domain, "", ability)
}

fn mount_scope(mount: &str, prefix: &str) -> String {
    format!("{mount}:{prefix}")
}

fn shell_scope(command_pattern: &str, working_dir: &Option<String>) -> String {
    match working_dir {
        Some(directory) => format!("{command_pattern}:{directory}"),
        None => command_pattern.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use aspen_auth_core::Capability;

    use super::capabilities_to_ucan_set;
    use super::capability_to_ucan_document;

    #[test]
    fn maps_kv_full_to_ucan_wildcard_ability() {
        let document = capability_to_ucan_document(&Capability::Full {
            prefix: "tenant-a/".to_owned(),
        })
        .expect("capability should map");

        assert_eq!(document.resource, "aspen:kv:tenant-a/");
        assert_eq!(document.ability, "kv/*");
    }

    #[test]
    fn maps_delegate_as_auth_boundary_marker() {
        let document = capability_to_ucan_document(&Capability::Delegate).expect("delegate should map");

        assert_eq!(document.resource, "aspen:auth:");
        assert_eq!(document.ability, "auth/delegate");
    }

    #[test]
    fn builds_sibling_ucan_capability_set() {
        let capabilities = [
            Capability::Read {
                prefix: "tenant-a/".to_owned(),
            },
            Capability::FederationPull {
                repo_prefix: "forge:org-a/".to_owned(),
            },
        ];

        let set = capabilities_to_ucan_set(&capabilities).expect("ucan set should validate");

        assert_eq!(set.as_slice().len(), 2);
        assert_eq!(set.as_slice()[0].ability, "kv/read");
        assert_eq!(set.as_slice()[1].resource, "aspen:federation:forge:org-a/");
    }

    #[test]
    fn rejects_empty_ucan_capability_set() {
        let error = capabilities_to_ucan_set(&[]).expect_err("empty UCAN sets are denied");

        assert!(matches!(error, aspen_auth_core::AuthError::EncodingError(_)));
    }
}
