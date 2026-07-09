type IoValue = preserves::IOValue;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type Path = std::path::Path;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA;
const EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA;
const CHAIN_RECEIPT_SCHEMA: &str = crate::preserves_rail::IROH_CHAIN_EXCHANGE_RECEIPT_SCHEMA;
const REPRO_RECEIPT_SCHEMA: &str = crate::preserves_rail::IROH_REPRO_EXCHANGE_RECEIPT_SCHEMA;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn content_ref_hex(value: &str) -> Result<&str> {
    crate::preserves_rail::content_ref_hex(value)
}

fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
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

const MAX_CHAIN_BUNDLE_ARTIFACTS: usize = 100_000;
const MAX_CHAIN_BUNDLE_CHECKPOINTS: usize = 10_000;

pub type CapabilityExchangeRoot = crate::local_store::ExchangeStoreRoot;

pub fn open_capability_exchange_root(root: &Path) -> Result<CapabilityExchangeRoot> {
    crate::local_store::ExchangeStoreRoot::open(root)
}

const _: () = assert!(MAX_CHAIN_BUNDLE_ARTIFACTS <= 1_000_000);
const _: () = assert!(MAX_CHAIN_BUNDLE_CHECKPOINTS <= 100_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repro {
    pub ticket: String,
    pub bundle_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSegment {
    pub ticket: String,
    pub bundle_ref: String,
    pub chain: crate::evidence_chain::ChainScope,
    pub anchor_ref: Option<String>,
    pub head_ref: Option<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChainSegmentBundle {
    bundle_ref: String,
    chain: crate::evidence_chain::ChainScope,
    anchor_ref: Option<String>,
    head_ref: Option<String>,
    artifacts: Vec<ChainBundleArtifact>,
    verify_receipt_refs: Vec<String>,
    checkpoint_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChainBundleArtifact {
    kind: String,
    artifact_ref: String,
    value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedChainVerifyReceipt {
    decision: String,
    chain: crate::evidence_chain::ChainScope,
    anchor_ref: Option<String>,
    expected_head: Option<String>,
    discovered_heads: Vec<String>,
    verified_links: Vec<String>,
    payload_refs: Vec<String>,
    predicate_refs: Vec<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct FetchBundleInput<'a> {
    pub root: &'a Path,
    pub ticket: &'a str,
    pub expected_bundle_ref: Option<&'a str>,
    pub peer: &'a str,
    pub out: Option<&'a Path>,
    pub ledger_root: Option<&'a Path>,
}

#[derive(Debug, Clone, Copy)]
pub struct PublishChainSegmentInput<'a> {
    pub iroh_root: &'a Path,
    pub ledger_root: &'a Path,
    pub chain: &'a crate::evidence_chain::ChainScope,
    pub anchor_ref: Option<&'a str>,
    pub expected_head: Option<&'a str>,
    pub node: &'a str,
    pub fork_policy: crate::evidence_chain::ChainForkPolicy,
}

#[derive(Debug, Clone, Copy)]
pub struct FetchChainSegmentInput<'a> {
    pub iroh_root: &'a Path,
    pub ticket: &'a str,
    pub expected_bundle_ref: Option<&'a str>,
    pub peer: &'a str,
    pub ledger_root: &'a Path,
    pub fork_policy: crate::evidence_chain::ChainForkPolicy,
}

struct ChainSegmentBundleValueInput<'a> {
    chain: &'a crate::evidence_chain::ChainScope,
    anchor_ref: Option<&'a str>,
    head_ref: Option<&'a str>,
    artifacts: &'a [IoValue],
    verify_receipt_refs: &'a [String],
    checkpoint_refs: &'a [String],
}

struct ValidateChainBundleInput<'a> {
    chain: &'a crate::evidence_chain::ChainScope,
    anchor_ref: Option<&'a str>,
    head_ref: Option<&'a str>,
    artifacts: &'a [ChainBundleArtifact],
    verify_receipt_refs: &'a [String],
    checkpoint_refs: &'a [String],
    fork_policy: crate::evidence_chain::ChainForkPolicy,
}

struct ChainReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    node: &'a str,
    peer: &'a str,
    ticket: &'a str,
    bundle_ref: &'a str,
    verify_receipt_refs: &'a [String],
    checkpoint_refs: &'a [String],
}

struct ReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    node: &'a str,
    peer: &'a str,
    ticket: &'a str,
    bundle_ref: &'a str,
    verify_ref: Option<&'a str>,
}

pub fn publish_bundle(root: &Path, bundle: &IoValue, node: &str) -> Result<Repro> {
    fs::create_dir_all(root.join("blobs")).map_err(MoltenError::from)?;
    let verify_receipt = crate::harness::repro_verify_receipt_value(bundle)?;
    let bundle_ref = canonical_hash(bundle)?;
    let bytes = canonical_bytes(bundle)?;
    let blob_ref = content_ref_from_bytes(&bytes);
    if blob_ref != bundle_ref {
        return Err(MoltenError::invalid_harness("Iroh publish bundle blob ref does not match bundle ref"));
    }
    fs::write(blob_path(root, &bundle_ref)?, bytes).map_err(MoltenError::from)?;
    let ticket = format!("iroh-local:{bundle_ref}");
    let verify_ref = canonical_hash(&verify_receipt)?;
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: "publish",
        decision: "pass",
        node,
        peer: "local",
        ticket: &ticket,
        bundle_ref: &bundle_ref,
        verify_ref: Some(&verify_ref),
    });
    Ok(Repro {
        ticket,
        bundle_ref,
        receipt_value,
    })
}

pub fn fetch_bundle(input: &FetchBundleInput<'_>) -> Result<Repro> {
    let advertised_ref = input.ticket.strip_prefix("iroh-local:").ok_or_else(|| {
        MoltenError::invalid_harness("unsupported Iroh repro ticket; expected iroh-local:<bundle-ref>")
    })?;
    if let Some(expected_bundle_ref) = input.expected_bundle_ref
        && expected_bundle_ref != advertised_ref
    {
        return Err(MoltenError::invalid_harness(format!(
            "Iroh repro ticket advertises bundle {advertised_ref}, expected {expected_bundle_ref}"
        )));
    }
    let bytes = fs::read(blob_path(input.root, advertised_ref)?).map_err(MoltenError::from)?;
    let bundle = parse_canonical_bytes(&bytes)?;
    let bundle_ref = canonical_hash(&bundle)?;
    if bundle_ref != advertised_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Iroh fetched blob content hashes to {bundle_ref}, expected advertised bundle {advertised_ref}"
        )));
    }
    let verify_receipt = crate::harness::repro_verify_receipt_value(&bundle)?;
    if let Some(out) = input.out {
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(MoltenError::from)?;
        }
        fs::write(out, crate::preserves_rail::to_text(&bundle)?).map_err(MoltenError::from)?;
    }
    if let Some(ledger_root) = input.ledger_root {
        crate::ledger::import_artifact(ledger_root, &bundle)?;
        crate::ledger::import_artifact(ledger_root, &verify_receipt)?;
    }
    let verify_ref = canonical_hash(&verify_receipt)?;
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: "fetch",
        decision: "pass",
        node: "local",
        peer: input.peer,
        ticket: input.ticket,
        bundle_ref: &bundle_ref,
        verify_ref: Some(&verify_ref),
    });
    Ok(Repro {
        ticket: input.ticket.to_string(),
        bundle_ref,
        receipt_value,
    })
}
