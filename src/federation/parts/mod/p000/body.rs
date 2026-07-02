type BtreeSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type Result<T> = crate::error::Result<T>;
type RuntimeAssertion = crate::runtime::RuntimeAssertion;
type RuntimeValue = crate::runtime::RuntimeValue;
type Value<T> = preserves::Value<T>;

mod chunk_store {
    pub(super) fn fetch_iroh_blobs(
        iroh_root: &super::Path,
        dest_root: &super::Path,
        ticket: &str,
        expected_manifest_ref: Option<&str>,
        peer: &str,
    ) -> super::Result<crate::chunk_store::ChunkStoreIrohFetch> {
        crate::chunk_store::fetch_iroh_blobs(iroh_root, dest_root, ticket, expected_manifest_ref, peer)
    }

    #[cfg(test)]
    pub(super) fn publish_iroh_blobs(
        store_root: &super::Path,
        iroh_root: &super::Path,
        manifest_ref: &str,
        node: &str,
    ) -> super::Result<crate::chunk_store::ChunkStoreIrohPublish> {
        crate::chunk_store::publish_iroh_blobs(store_root, iroh_root, manifest_ref, node)
    }

    #[cfg(test)]
    pub(super) fn put_bytes(
        root: &super::Path,
        object_kind: &str,
        bytes: &[u8],
        chunk_size: u64,
    ) -> super::Result<crate::chunk_store::ChunkStorePut> {
        crate::chunk_store::put_bytes(root, object_kind, bytes, chunk_size)
    }

    #[cfg(test)]
    pub(super) fn read_object(
        root: &super::Path,
        manifest_ref: &str,
    ) -> super::Result<crate::chunk_store::ChunkStoreRead> {
        crate::chunk_store::read_object(root, manifest_ref)
    }
}

mod ledger {
    pub(super) fn artifact_kind(value: &super::IoValue) -> &'static str {
        crate::ledger::artifact_kind(value)
    }

    pub(super) fn import_artifact(
        root: &super::Path,
        artifact: &super::IoValue,
    ) -> super::Result<crate::ledger::Import> {
        crate::ledger::import_artifact(root, artifact)
    }

    pub(super) fn list_artifacts(root: &super::Path) -> super::Result<Vec<crate::ledger::Entry>> {
        crate::ledger::list_artifacts(root)
    }

    pub(super) fn read_artifact(root: &super::Path, artifact_ref: &str) -> super::Result<super::IoValue> {
        crate::ledger::read_artifact(root, artifact_ref)
    }
}

const ANNOUNCEMENT_SCHEMA: &str = crate::preserves_rail::FEDERATION_ANNOUNCEMENT_SCHEMA;
const INVENTORY_SCHEMA: &str = crate::preserves_rail::FEDERATION_INVENTORY_SCHEMA;
const RECEIPT_SCHEMA: &str = crate::preserves_rail::FEDERATION_RECEIPT_SCHEMA;
const SIGNATURE_ALGORITHM: &str = crate::evidence::SIGNATURE_ALGORITHM;

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const ANNOUNCEMENT_PURPOSE: &str = "federation-announcement";
pub const INVENTORY_PURPOSE: &str = "federation-inventory";
pub const RESOURCE_ARTIFACT: &str = "artifact";
pub const RESOURCE_CHUNK_MANIFEST: &str = "chunk-manifest";
pub const RESOURCE_CHUNK: &str = "chunk";
pub const RESOURCE_DOC_METADATA: &str = "doc-metadata";
pub const RESOURCE_CATALOG_METADATA: &str = "catalog-metadata";
pub const RESOURCE_RECEIPT: &str = "receipt";
pub const RESOURCE_PROVENANCE: &str = "provenance";
pub const RESOURCE_TRANSCRIPT: &str = "transcript";
pub const RESOURCE_PROTOCOL: &str = "protocol";
pub const RESOURCE_SCHEMA: &str = "schema";

const MAX_RESOURCES: usize = 4096;
const MAX_ASSERTIONS: usize = 8192;

const _: () = assert!(MAX_RESOURCES <= 100_000);
const _: () = assert!(MAX_ASSERTIONS <= 100_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub resource_type: String,
    pub resource_ref: String,
    pub schema: String,
    pub transport: String,
    pub source_peer: String,
}

impl Resource {
    pub fn new(
        resource_type: impl Into<String>,
        resource_ref: impl Into<String>,
        schema: impl Into<String>,
        transport: impl Into<String>,
        source_peer: impl Into<String>,
    ) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_ref: resource_ref.into(),
            schema: schema.into(),
            transport: transport.into(),
            source_peer: source_peer.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub announcement_ref: String,
    pub peer: String,
    pub resource: Resource,
    pub signer: String,
    pub trust_root: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    pub inventory_ref: String,
    pub peer: String,
    pub resources: Vec<Resource>,
    pub delegates: Vec<Delegate>,
    pub signer: String,
    pub trust_root: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delegate {
    pub delegate_ref: String,
    pub resource_ref: String,
    pub capability: String,
    pub signer: String,
    pub trust_root: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullPolicy {
    pub allowed_resource_types: Vec<String>,
    pub required_delegate_capability: Option<String>,
    pub delegate_trust_root: String,
    pub delegate_key: String,
    pub max_resources: usize,
    pub max_imports: usize,
}

impl PullPolicy {
    pub fn allow_all() -> Self {
        Self {
            allowed_resource_types: Vec::new(),
            required_delegate_capability: None,
            delegate_trust_root: String::new(),
            delegate_key: String::new(),
            max_resources: usize::MAX,
            max_imports: usize::MAX,
        }
    }

    pub fn allowed_types(allowed_resource_types: Vec<String>) -> Self {
        Self {
            allowed_resource_types,
            ..Self::allow_all()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pull {
    pub peer: String,
    pub imported_refs: Vec<String>,
    pub skipped_refs: Vec<String>,
    pub denied_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct AnnounceResourceInput<'a> {
    pub peer: &'a str,
    pub resource: &'a Resource,
    pub signer: &'a str,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct InventoryWithDelegatesInput<'a> {
    pub peer: &'a str,
    pub resources: &'a [Resource],
    pub delegates: &'a [Delegate],
    pub signer: &'a str,
    pub trust_root: &'a str,
    pub key: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct PullLedgerInventoryInput<'a> {
    pub source_root: &'a Path,
    pub dest_root: &'a Path,
    pub inventory_value: &'a IoValue,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub allowed_resource_types: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct PullLedgerInventoryPolicyInput<'a> {
    pub source_root: &'a Path,
    pub dest_root: &'a Path,
    pub inventory_value: &'a IoValue,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub policy: &'a PullPolicy,
}

#[derive(Debug, Clone, Copy)]
pub struct PullChunkManifestInput<'a> {
    pub iroh_root: &'a Path,
    pub dest_root: &'a Path,
    pub announcement_value: &'a IoValue,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub peer: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptValueInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub peer: &'a str,
    pub resources: &'a [Resource],
    pub imported_refs: &'a [String],
    pub skipped_refs: &'a [String],
    pub denied_refs: &'a [String],
}
