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

pub fn publish_chain_segment(input: &PublishChainSegmentInput<'_>) -> Result<ChainSegment> {
    fs::create_dir_all(input.iroh_root.join("blobs")).map_err(MoltenError::from)?;
    let bundle = build_chain_segment_bundle_value(
        input.ledger_root,
        input.chain,
        input.anchor_ref,
        input.expected_head,
        input.fork_policy,
    )?;
    let parsed = parse_chain_segment_bundle(&bundle, input.fork_policy)?;
    let bytes = canonical_bytes(&bundle)?;
    let blob_ref = content_ref_from_bytes(&bytes);
    if blob_ref != parsed.bundle_ref {
        return Err(MoltenError::invalid_harness("Iroh publish chain bundle blob ref does not match bundle ref"));
    }
    fs::write(blob_path(input.iroh_root, &parsed.bundle_ref)?, bytes).map_err(MoltenError::from)?;
    let ticket = format!("iroh-local-chain:{}", parsed.bundle_ref);
    let receipt_value = chain_receipt_value(&ChainReceiptValueInput {
        operation: "publish",
        decision: "pass",
        node: input.node,
        peer: "local",
        ticket: &ticket,
        bundle_ref: &parsed.bundle_ref,
        verify_receipt_refs: &parsed.verify_receipt_refs,
        checkpoint_refs: &parsed.checkpoint_refs,
    });
    Ok(ChainSegment {
        ticket,
        bundle_ref: parsed.bundle_ref,
        chain: parsed.chain,
        anchor_ref: parsed.anchor_ref,
        head_ref: parsed.head_ref,
        receipt_value,
    })
}

pub fn fetch_chain_segment(input: &FetchChainSegmentInput<'_>) -> Result<ChainSegment> {
    let advertised_ref = input.ticket.strip_prefix("iroh-local-chain:").ok_or_else(|| {
        MoltenError::invalid_harness("unsupported Iroh chain ticket; expected iroh-local-chain:<bundle-ref>")
    })?;
    if let Some(expected_bundle_ref) = input.expected_bundle_ref
        && expected_bundle_ref != advertised_ref
    {
        return Err(MoltenError::invalid_harness(format!(
            "Iroh chain ticket advertises bundle {advertised_ref}, expected {expected_bundle_ref}"
        )));
    }
    let bytes = fs::read(blob_path(input.iroh_root, advertised_ref)?).map_err(MoltenError::from)?;
    let bundle = parse_canonical_bytes(&bytes)?;
    let parsed = parse_chain_segment_bundle(&bundle, input.fork_policy)?;
    if parsed.bundle_ref != advertised_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Iroh fetched chain bundle hashes to {}, expected advertised bundle {advertised_ref}",
            parsed.bundle_ref
        )));
    }
    for artifact in &parsed.artifacts {
        crate::ledger::import_artifact(input.ledger_root, &artifact.value)?;
    }
    crate::ledger::import_artifact(input.ledger_root, &bundle)?;
    let receipt_value = chain_receipt_value(&ChainReceiptValueInput {
        operation: "fetch",
        decision: "pass",
        node: "local",
        peer: input.peer,
        ticket: input.ticket,
        bundle_ref: &parsed.bundle_ref,
        verify_receipt_refs: &parsed.verify_receipt_refs,
        checkpoint_refs: &parsed.checkpoint_refs,
    });
    crate::ledger::import_artifact(input.ledger_root, &receipt_value)?;
    Ok(ChainSegment {
        ticket: input.ticket.to_string(),
        bundle_ref: parsed.bundle_ref,
        chain: parsed.chain,
        anchor_ref: parsed.anchor_ref,
        head_ref: parsed.head_ref,
        receipt_value,
    })
}

fn build_chain_segment_bundle_value(
    ledger_root: &Path,
    chain: &crate::evidence_chain::ChainScope,
    anchor_ref: Option<&str>,
    expected_head: Option<&str>,
    fork_policy: crate::evidence_chain::ChainForkPolicy,
) -> Result<IoValue> {
    let verified = crate::evidence_chain::verify_chain_segment_with_policy(
        ledger_root,
        chain,
        anchor_ref,
        expected_head,
        fork_policy,
    )?;
    let index = crate::evidence_chain::build_chain_index(ledger_root)?;
    let mut artifacts = OrderedMap::<String, IoValue>::new();
    for link_ref in &verified.verified_links {
        add_ledger_artifact(ledger_root, &mut artifacts, link_ref)?;
        if let Some(link) = index.links_by_ref.get(link_ref) {
            add_ledger_artifact(ledger_root, &mut artifacts, &link.payload.artifact_ref)?;
        }
    }
    for predicate_ref in &verified.predicate_receipt_refs {
        add_ledger_artifact(ledger_root, &mut artifacts, predicate_ref)?;
    }
    add_ledger_artifact(ledger_root, &mut artifacts, &verified.receipt_ref)?;
    for fork_ref in index.fork_evidence_for_chain(chain) {
        add_ledger_artifact(ledger_root, &mut artifacts, &fork_ref)?;
    }
    for anchor_ref in index.anchors_for_chain(chain) {
        add_ledger_artifact(ledger_root, &mut artifacts, &anchor_ref)?;
    }
    let mut checkpoint_refs = Vec::new();
    for checkpoint_ref in index.checkpoints_for_chain(chain) {
        let checkpoint_value = crate::ledger::read_artifact(ledger_root, &checkpoint_ref)?;
        let checkpoint = crate::evidence_chain::parse_chain_checkpoint(&checkpoint_value)?;
        add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint_ref)?;
        for checkpoint_link_ref in [&checkpoint.anchor_link_ref, &checkpoint.head_ref] {
            add_ledger_artifact(ledger_root, &mut artifacts, checkpoint_link_ref)?;
            let checkpoint_link_value = crate::ledger::read_artifact(ledger_root, checkpoint_link_ref)?;
            let checkpoint_link = crate::evidence_chain::parse_chain_link(&checkpoint_link_value)?;
            add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint_link.payload.artifact_ref)?;
        }
        add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint.verify_receipt_ref)?;
        add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint.range_predicate_ref)?;
        push_bounded(
            &mut checkpoint_refs,
            checkpoint_ref,
            MAX_CHAIN_BUNDLE_CHECKPOINTS,
            "chain bundle checkpoint refs",
        )?;
    }
    let artifact_values = artifacts
        .iter()
        .map(|(artifact_ref, value)| {
            record("artifact", vec![
                string(crate::ledger::artifact_kind(value)),
                string(artifact_ref),
                value.clone(),
            ])
        })
        .collect::<Vec<_>>();
    Ok(chain_segment_bundle_value(&ChainSegmentBundleValueInput {
        chain,
        anchor_ref,
        head_ref: expected_head,
        artifacts: &artifact_values,
        verify_receipt_refs: std::slice::from_ref(&verified.receipt_ref),
        checkpoint_refs: &checkpoint_refs,
    }))
}

fn add_ledger_artifact(root: &Path, artifacts: &mut OrderedMap<String, IoValue>, artifact_ref: &str) -> Result<()> {
    if !artifacts.contains_key(artifact_ref) {
        artifacts.insert(artifact_ref.to_string(), crate::ledger::read_artifact(root, artifact_ref)?);
    }
    Ok(())
}

fn chain_segment_bundle_value(input: &ChainSegmentBundleValueInput<'_>) -> IoValue {
    record("chain-segment-bundle-v1", vec![
        string(EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA),
        chain_scope_value(input.chain),
        record("anchor", vec![optional_ref_value(input.anchor_ref)]),
        record("head", vec![optional_ref_value(input.head_ref)]),
        record("artifacts", vec![sequence(input.artifacts.to_vec())]),
        record("verify-receipts", vec![sequence(input.verify_receipt_refs.iter().map(string).collect())]),
        record("checkpoints", vec![sequence(input.checkpoint_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("content-addressed-artifacts"), string("pass")]),
            record("check", vec![string("chain-segment-verified"), string("pass")]),
            record("check", vec![string("predicate-receipts-bound"), string("pass")]),
            record("check", vec![string("checkpoint-binding"), string("pass")]),
            record("check", vec![string("transport-does-not-grant-trust"), string("pass")]),
        ])]),
    ])
}

fn parse_chain_segment_bundle(
    value: &IoValue,
    fork_policy: crate::evidence_chain::ChainForkPolicy,
) -> Result<ChainSegmentBundle> {
    let bundle = value
        .collect_simple_record("chain-segment-bundle-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-segment-bundle-v1 ...>"))?;
    require_schema(&bundle[0], EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA, "chain segment bundle schema")?;
    let chain = parse_chain_scope(&bundle[1])?;
    let anchor_ref = parse_optional_ref_field(&bundle[2], "anchor")?;
    let head_ref = parse_optional_ref_field(&bundle[3], "head")?;
    let artifacts = parse_chain_bundle_artifacts(&bundle[4])?;
    let verify_receipt_refs = parse_ref_sequence_field(&bundle[5], "verify-receipts")?;
    let checkpoint_refs = parse_ref_sequence_field(&bundle[6], "checkpoints")?;
    let checks = parse_check_names(&bundle[7])?;
    require_check(&checks, "content-addressed-artifacts")?;
    require_check(&checks, "chain-segment-verified")?;
    require_check(&checks, "predicate-receipts-bound")?;
    require_check(&checks, "transport-does-not-grant-trust")?;
    validate_chain_bundle(ValidateChainBundleInput {
        chain: &chain,
        anchor_ref: anchor_ref.as_deref(),
        head_ref: head_ref.as_deref(),
        artifacts: &artifacts,
        verify_receipt_refs: &verify_receipt_refs,
        checkpoint_refs: &checkpoint_refs,
        fork_policy,
    })?;
    Ok(ChainSegmentBundle {
        bundle_ref: canonical_hash(value)?,
        chain,
        anchor_ref,
        head_ref,
        artifacts,
        verify_receipt_refs,
        checkpoint_refs,
    })
}

struct Parts<'a> {
    artifacts: OrderedMap<String, &'a ChainBundleArtifact>,
    links: OrderedMap<String, crate::evidence_chain::ChainLink>,
    predicates: OrderedMap<String, crate::evidence_chain::ChainPredicateReceipt>,
    has_forks: bool,
}

struct PrimaryCheck<'a> {
    primary: &'a ParsedChainVerifyReceipt,
    chain: &'a crate::evidence_chain::ChainScope,
    anchor_ref: Option<&'a str>,
    head_ref: Option<&'a str>,
    fork_policy: crate::evidence_chain::ChainForkPolicy,
    has_forks: bool,
}

fn validate_chain_bundle(input: ValidateChainBundleInput<'_>) -> Result<()> {
    let parts = parsed_parts(input.artifacts)?;
    anchors(input.chain, input.artifacts)?;
    let receipts = receipts(input.verify_receipt_refs, &parts.artifacts)?;
    let primary = primary(&receipts)?;
    check_primary(PrimaryCheck {
        primary,
        chain: input.chain,
        anchor_ref: input.anchor_ref,
        head_ref: input.head_ref,
        fork_policy: input.fork_policy,
        has_forks: parts.has_forks,
    })?;
    validate_chain_links(&parts.links, primary, input.anchor_ref, input.head_ref)?;
    payloads(primary, &parts.artifacts)?;
    predicates(primary, &parts.predicates)?;
    let checkpoints = checkpoints(input.checkpoint_refs, &parts.artifacts)?;
    validate_bundle_checkpoints(input.chain, &checkpoints, &parts.artifacts, &parts.links, &parts.predicates)?;
    Ok(())
}

fn parsed_parts(artifacts: &[ChainBundleArtifact]) -> Result<Parts<'_>> {
    let parts = artifacts
        .iter()
        .map(|artifact| (artifact.artifact_ref.clone(), artifact))
        .collect::<OrderedMap<_, _>>();
    let links = artifacts
        .iter()
        .filter(|artifact| artifact.kind == "chain-link")
        .map(|artifact| {
            crate::evidence_chain::parse_chain_link(&artifact.value).map(|link| (link.link_ref.clone(), link))
        })
        .collect::<Result<OrderedMap<_, _>>>()?;
    let predicates = artifacts
        .iter()
        .filter(|artifact| artifact.kind == "chain-predicate-receipt")
        .map(|artifact| {
            crate::evidence_chain::parse_chain_predicate_receipt(&artifact.value)
                .map(|receipt| (receipt.receipt_ref.clone(), receipt))
        })
        .collect::<Result<OrderedMap<_, _>>>()?;
    let mut has_forks = false;
    for artifact in artifacts.iter().filter(|artifact| artifact.kind == "chain-fork-evidence") {
        crate::evidence_chain::parse_chain_fork_evidence(&artifact.value)?;
        has_forks = true;
    }
    Ok(Parts {
        artifacts: parts,
        links,
        predicates,
        has_forks,
    })
}

fn anchors(chain: &crate::evidence_chain::ChainScope, artifacts: &[ChainBundleArtifact]) -> Result<()> {
    for artifact in artifacts.iter().filter(|artifact| artifact.kind == "chain-anchor") {
        let anchor = crate::evidence_chain::parse_chain_anchor(&artifact.value)?;
        if anchor.chain != *chain {
            return Err(MoltenError::invalid_harness("chain bundle anchor belongs to a different chain"));
        }
    }
    Ok(())
}

fn receipts(
    refs: &[String],
    artifacts: &OrderedMap<String, &ChainBundleArtifact>,
) -> Result<Vec<ParsedChainVerifyReceipt>> {
    refs.iter()
        .map(|verify_ref| {
            let artifact = artifacts.get(verify_ref).ok_or_else(|| {
                MoltenError::invalid_harness(format!("chain bundle missing verify receipt {verify_ref}"))
            })?;
            parse_chain_verify_receipt_value(&artifact.value)
        })
        .collect()
}

fn primary(receipts: &[ParsedChainVerifyReceipt]) -> Result<&ParsedChainVerifyReceipt> {
    receipts
        .first()
        .ok_or_else(|| MoltenError::invalid_harness("chain bundle must contain a verify receipt"))
}

fn check_primary(input: PrimaryCheck<'_>) -> Result<()> {
    let primary = input.primary;
    if primary.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "chain bundle verify receipt decision must be pass, got {}",
            primary.decision
        )));
    }
    if &primary.chain != input.chain
        || primary.anchor_ref.as_deref() != input.anchor_ref
        || primary.expected_head.as_deref() != input.head_ref
    {
        return Err(MoltenError::invalid_harness("chain bundle verify receipt does not match requested chain range"));
    }
    let has_fork_diagnostics = primary
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.as_str(), "fork" | "sequence-conflict"));
    if has_fork_diagnostics && input.fork_policy == crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks {
        return Err(MoltenError::invalid_harness(
            "chain bundle contains fork diagnostics rejected by production policy",
        ));
    }
    if has_fork_diagnostics && !input.has_forks {
        return Err(MoltenError::invalid_harness("chain bundle fork diagnostics require fork evidence artifacts"));
    }
    Ok(())
}

fn payloads(verify: &ParsedChainVerifyReceipt, artifacts: &OrderedMap<String, &ChainBundleArtifact>) -> Result<()> {
    for payload_ref in &verify.payload_refs {
        if !artifacts.contains_key(payload_ref) {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle missing payload artifact {payload_ref} named by verify receipt"
            )));
        }
    }
    Ok(())
}

fn predicates(
    verify: &ParsedChainVerifyReceipt,
    predicates: &OrderedMap<String, crate::evidence_chain::ChainPredicateReceipt>,
) -> Result<()> {
    for predicate_ref in &verify.predicate_refs {
        let Some(predicate) = predicates.get(predicate_ref) else {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle missing predicate receipt {predicate_ref} named by verify receipt"
            )));
        };
        if predicate.decision == "fail" {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle predicate receipt {predicate_ref} has failing decision"
            )));
        }
    }
    Ok(())
}

fn checkpoints(
    refs: &[String],
    artifacts: &OrderedMap<String, &ChainBundleArtifact>,
) -> Result<Vec<crate::evidence_chain::ChainCheckpoint>> {
    refs.iter()
        .map(|checkpoint_ref| {
            let artifact = artifacts.get(checkpoint_ref).ok_or_else(|| {
                MoltenError::invalid_harness(format!("chain bundle missing checkpoint {checkpoint_ref}"))
            })?;
            crate::evidence_chain::parse_chain_checkpoint(&artifact.value)
        })
        .collect()
}

fn validate_chain_links(
    links: &OrderedMap<String, crate::evidence_chain::ChainLink>,
    verify: &ParsedChainVerifyReceipt,
    anchor_ref: Option<&str>,
    head_ref: Option<&str>,
) -> Result<()> {
    if verify.verified_links.is_empty() {
        return Err(MoltenError::invalid_harness("chain bundle verify receipt contains no verified links"));
    }
    if let Some(anchor_ref) = anchor_ref
        && verify.verified_links.first().map(String::as_str) != Some(anchor_ref)
    {
        return Err(MoltenError::invalid_harness("chain bundle segment does not begin at requested anchor"));
    }
    if let Some(head_ref) = head_ref
        && verify.verified_links.last().map(String::as_str) != Some(head_ref)
    {
        return Err(MoltenError::invalid_harness("chain bundle segment does not end at requested head"));
    }
    for (position, link_ref) in verify.verified_links.iter().enumerate() {
        let link = links
            .get(link_ref)
            .ok_or_else(|| MoltenError::invalid_harness(format!("chain bundle missing verified link {link_ref}")))?;
        if link.chain != verify.chain {
            return Err(MoltenError::invalid_harness("chain bundle link belongs to a different chain scope"));
        }
        if position == 0 && anchor_ref.is_none() {
            crate::evidence_chain::validate_genesis(link)?;
        }
        if position > 0 {
            let previous_ref = &verify.verified_links[position - 1];
            let previous = links.get(previous_ref).ok_or_else(|| {
                MoltenError::invalid_harness(format!("chain bundle missing previous link {previous_ref}"))
            })?;
            crate::evidence_chain::validate_append(previous, link)?;
        }
    }
    Ok(())
}

fn validate_bundle_checkpoints(
    chain: &crate::evidence_chain::ChainScope,
    checkpoints: &[crate::evidence_chain::ChainCheckpoint],
    artifacts: &OrderedMap<String, &ChainBundleArtifact>,
    links: &OrderedMap<String, crate::evidence_chain::ChainLink>,
    predicates: &OrderedMap<String, crate::evidence_chain::ChainPredicateReceipt>,
) -> Result<()> {
    for checkpoint in checkpoints {
        if &checkpoint.chain != chain {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint belongs to a different chain"));
        }
        if !links.contains_key(&checkpoint.anchor_link_ref) || !links.contains_key(&checkpoint.head_ref) {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint anchor/head links are unavailable"));
        }
        let Some(verify_artifact) = artifacts.get(&checkpoint.verify_receipt_ref) else {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint verify receipt is unavailable"));
        };
        let verify = parse_chain_verify_receipt_value(&verify_artifact.value)?;
        if verify.decision != "pass"
            || verify.anchor_ref.as_deref() != Some(&checkpoint.anchor_link_ref)
            || verify.expected_head.as_deref() != Some(&checkpoint.head_ref)
        {
            return Err(MoltenError::invalid_harness(
                "chain bundle checkpoint verify receipt does not bind checkpoint range",
            ));
        }
        let Some(predicate) = predicates.get(&checkpoint.range_predicate_ref) else {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint range predicate is unavailable"));
        };
        if predicate.predicate != crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE
            || predicate.decision != "pass"
        {
            return Err(MoltenError::invalid_harness(
                "chain bundle checkpoint range predicate must be a passing checkpoint coverage receipt",
            ));
        }
    }
    Ok(())
}

fn parse_chain_bundle_artifacts(value: &Value<IoValue>) -> Result<Vec<ChainBundleArtifact>> {
    let artifacts = value
        .collect_simple_record("artifacts", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifacts ...> field"))?;
    let sequence = artifacts[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain bundle artifacts must be a sequence"))?;
    let mut parsed = Vec::new();
    for item in sequence.iter() {
        let item = value_to_iovalue(item);
        let artifact = item
            .collect_simple_record("artifact", Some(3))
            .ok_or_else(|| MoltenError::invalid_harness("expected <artifact kind ref value> in chain bundle"))?;
        let kind = required_string(&artifact[0], "chain bundle artifact kind")?;
        let artifact_ref = required_ref(&artifact[1], "chain bundle artifact ref")?;
        let value = value_to_iovalue(&artifact[2]);
        let actual_ref = canonical_hash(&value)?;
        if actual_ref != artifact_ref {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle artifact ref mismatch: got {actual_ref}, expected {artifact_ref}"
            )));
        }
        let actual_kind = crate::ledger::artifact_kind(&value);
        if actual_kind != kind {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle artifact {artifact_ref} kind mismatch: declared {kind}, parsed {actual_kind}"
            )));
        }
        push_bounded(
            &mut parsed,
            ChainBundleArtifact {
                kind,
                artifact_ref,
                value,
            },
            MAX_CHAIN_BUNDLE_ARTIFACTS,
            "chain bundle artifacts",
        )?;
    }
    if parsed.is_empty() {
        return Err(MoltenError::invalid_harness("chain bundle must contain artifacts"));
    }
    Ok(parsed)
}

fn parse_chain_verify_receipt_value(value: &IoValue) -> Result<ParsedChainVerifyReceipt> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify receipt in chain bundle"))?;
    require_schema(&receipt[0], EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA, "chain verify receipt schema")?;
    let diagnostics = parse_diagnostic_kinds(&receipt[9])?;
    Ok(ParsedChainVerifyReceipt {
        decision: record_string(&receipt[1], "decision")?,
        chain: parse_chain_scope(&receipt[2])?,
        anchor_ref: parse_optional_ref_field(&receipt[3], "anchor")?,
        expected_head: parse_optional_ref_field(&receipt[4], "expected-head")?,
        discovered_heads: parse_ref_sequence_field(&receipt[5], "discovered-heads")?,
        verified_links: parse_ref_sequence_field(&receipt[6], "verified-links")?,
        payload_refs: parse_ref_sequence_field(&receipt[7], "payloads")?,
        predicate_refs: parse_ref_sequence_field(&receipt[8], "predicates")?,
        diagnostics,
    })
}

fn parse_diagnostic_kinds(value: &Value<IoValue>) -> Result<Vec<String>> {
    let diagnostics = value
        .collect_simple_record("diagnostics", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify diagnostics field"))?;
    let values = diagnostics[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain verify diagnostics must be a sequence"))?;
    values
        .iter()
        .map(|diagnostic| {
            let diagnostic = value_to_iovalue(diagnostic);
            let record = diagnostic
                .collect_simple_record("diagnostic", Some(3))
                .ok_or_else(|| MoltenError::invalid_harness("expected diagnostic record"))?;
            required_string(&record[0], "diagnostic kind")
        })
        .collect()
}

fn chain_receipt_value(input: &ChainReceiptValueInput<'_>) -> IoValue {
    let mut refs = vec![record("artifact-ref", vec![
        string("chain-segment-bundle"),
        string(input.bundle_ref),
    ])];
    refs.extend(
        input
            .verify_receipt_refs
            .iter()
            .map(|verify_ref| record("artifact-ref", vec![string("chain-verify-receipt"), string(verify_ref)])),
    );
    refs.extend(
        input
            .checkpoint_refs
            .iter()
            .map(|checkpoint_ref| record("artifact-ref", vec![string("chain-checkpoint"), string(checkpoint_ref)])),
    );
    record("iroh-chain-exchange-receipt-v1", vec![
        string(CHAIN_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("node", vec![string(input.node)]),
        record("peer", vec![string(input.peer)]),
        record("ticket", vec![string(input.ticket)]),
        record("artifact-refs", vec![sequence(refs)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("content-addressed-chain-bundle"), string("pass")]),
            record("check", vec![string("chain-segment-verified"), string("pass")]),
            record("check", vec![string("predicate-receipts-bound"), string("pass")]),
            record("check", vec![string("checkpoint-binding"), string("pass")]),
            record("check", vec![string("transport-does-not-grant-trust"), string("pass")]),
        ])]),
    ])
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn parse_chain_scope(value: &Value<IoValue>) -> Result<crate::evidence_chain::ChainScope> {
    let value = value_to_iovalue(value);
    let chain = value
        .collect_simple_record("chain", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain scope record"))?;
    Ok(crate::evidence_chain::ChainScope::new(
        record_string(&chain[0], "scope")?,
        record_string(&chain[1], "id")?,
        record_string(&chain[2], "epoch")?,
    ))
}

fn chain_scope_value(chain: &crate::evidence_chain::ChainScope) -> IoValue {
    record("chain", vec![
        record("scope", vec![string(&chain.scope)]),
        record("id", vec![string(&chain.id)]),
        record("epoch", vec![string(&chain.epoch)]),
    ])
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_field(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_ref(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <none> or <some ref> for {label}")))
    }
}

fn parse_ref_sequence_field(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    values.iter().map(|value| required_ref(value, label)).collect()
}

fn parse_check_names(value: &Value<IoValue>) -> Result<Vec<String>> {
    let record = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <checks ...> field"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected sequence for checks"))?;
    values
        .iter()
        .map(|value| {
            let value = value_to_iovalue(value);
            let check = value
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
            let name = required_string(&check[0], "check name")?;
            let status = required_string(&check[1], "check status")?;
            if status != "pass" {
                return Err(MoltenError::invalid_harness(format!("check {name} status is {status}")));
            }
            Ok(name)
        })
        .collect()
}

fn require_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("chain bundle missing {expected} check")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_string(&record[0], label)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    validate_content_ref(&reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })?;
    Ok(reference)
}

fn receipt_value(input: &ReceiptValueInput<'_>) -> IoValue {
    let mut refs = vec![record("artifact-ref", vec![string("bundle"), string(input.bundle_ref)])];
    if let Some(verify_ref) = input.verify_ref {
        refs.push(record("artifact-ref", vec![string("verify-receipt"), string(verify_ref)]));
    }
    record("iroh-repro-exchange-receipt-v1", vec![
        string(REPRO_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("node", vec![string(input.node)]),
        record("peer", vec![string(input.peer)]),
        record("ticket", vec![string(input.ticket)]),
        record("artifact-refs", vec![sequence(refs)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("content-addressed-bundle"), string("pass")]),
            record("check", vec![string("sealed-repro-verified"), string("pass")]),
            record("check", vec![string("transport-does-not-grant-trust"), string("pass")]),
        ])]),
    ])
}

fn blob_path(root: &Path, bundle_ref: &str) -> Result<std::path::PathBuf> {
    let hex = content_ref_hex(bundle_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("unsupported Iroh bundle ref {bundle_ref}: {error}")))?;
    Ok(root.join("blobs").join(format!("blake3_{hex}.bin")))
}

#[cfg(test)]
mod tests {
    use super::*;

    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    #[test]
    fn local_iroh_publish_fetch_verifies_bundle_refs() {
        let root = temp_dir("iroh");
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "iroh" 1
              <budget-v1 "molten.harness.budget.v1" <limits 8 2 32 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "assert" #f "ready">]>
              [<assert "a" "ready">]>"#,
        )
        .expect("parse suite");
        let run = crate::harness::run_suite_value(&suite).expect("run suite");
        let bundle = crate::harness::sealed_repro_bundle_value_with_command(&run.report_value, &["molten".to_string()])
            .expect("seal bundle");
        let published = publish_bundle(&root, &bundle, "node:local").expect("publish bundle");
        let fetched = fetch_bundle(&FetchBundleInput {
            root: &root,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:local",
            out: None,
            ledger_root: None,
        })
        .expect("fetch bundle");
        assert_eq!(published.bundle_ref, fetched.bundle_ref);
        let error = fetch_bundle(&FetchBundleInput {
            root: &root,
            ticket: &published.ticket,
            expected_bundle_ref: Some("blake3:deadbeef"),
            peer: "peer:local",
            out: None,
            ledger_root: None,
        })
        .expect_err("wrong advertised ref fails");
        assert!(error.to_string().contains("expected blake3:deadbeef"));
    }

    #[test]
    fn local_iroh_chain_segment_publish_fetch_imports_verified_artifacts() {
        let source = temp_dir("chain-source");
        let destination = temp_dir("chain-destination");
        let iroh = temp_dir("chain-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-a", "epoch-1");
        let genesis = append_test_link(&source, &chain, None, "payload-a");
        let second = append_test_link(&source, &chain, Some(&genesis), "payload-b");
        let policy_ref = ref_for("checkpoint-policy");
        crate::evidence_chain::publish_chain_anchor(
            &source,
            &chain,
            &genesis.link_ref,
            std::slice::from_ref(&policy_ref),
            &sample_producer(),
        )
        .expect("publish anchor");
        let verified = crate::evidence_chain::verify_chain_segment(
            &source,
            &chain,
            Some(&genesis.link_ref),
            Some(&second.link_ref),
        )
        .expect("verify range");
        crate::evidence_chain::accept_chain_checkpoint(&source, &crate::evidence_chain::ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&source, &verified),
            policy_refs: vec![policy_ref],
            membership_refs: vec![ref_for("membership")],
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept checkpoint");

        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: Some(&genesis.link_ref),
            expected_head: Some(&second.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("publish chain segment");
        let fetched = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("fetch chain segment");
        assert_eq!(published.bundle_ref, fetched.bundle_ref);
        let destination_index = crate::evidence_chain::build_chain_index(&destination).expect("destination index");
        assert_eq!(destination_index.heads_for_chain(&chain), vec![second.link_ref.clone()]);
        assert_eq!(destination_index.anchor_links_for_chain(&chain), vec![genesis.link_ref.clone()]);
        assert_eq!(destination_index.checkpoint_heads_for_chain(&chain), vec![second.link_ref.clone()]);
    }

    #[test]
    fn fetched_chain_segment_rejects_missing_checkpoint_predicate_artifact() {
        let source = temp_dir("chain-missing-predicate-source");
        let destination = temp_dir("chain-missing-predicate-destination");
        let iroh = temp_dir("chain-missing-predicate-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-missing-predicate", "epoch-1");
        let genesis = append_test_link(&source, &chain, None, "payload-a");
        let second = append_test_link(&source, &chain, Some(&genesis), "payload-b");
        let policy_ref = ref_for("checkpoint-policy");
        crate::evidence_chain::publish_chain_anchor(
            &source,
            &chain,
            &genesis.link_ref,
            std::slice::from_ref(&policy_ref),
            &sample_producer(),
        )
        .expect("publish anchor");
        let verified = crate::evidence_chain::verify_chain_segment(
            &source,
            &chain,
            Some(&genesis.link_ref),
            Some(&second.link_ref),
        )
        .expect("verify range");
        crate::evidence_chain::accept_chain_checkpoint(&source, &crate::evidence_chain::ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref,
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&source, &verified),
            policy_refs: vec![policy_ref],
            membership_refs: vec![ref_for("membership")],
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept checkpoint");
        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: None,
            expected_head: Some(&second.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("publish chain segment");
        let bundle_bytes = fs::read(blob_path(&iroh, &published.bundle_ref).expect("blob path")).expect("read bundle");
        let bundle = parse_canonical_bytes(&bundle_bytes).expect("parse bundle");
        let tampered = remove_bundle_artifacts_of_kind(&bundle, "chain-predicate-receipt");
        fs::write(
            blob_path(&iroh, &published.bundle_ref).expect("blob path"),
            canonical_bytes(&tampered).expect("canonical tampered bundle"),
        )
        .expect("write tampered bundle");
        let error = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect_err("missing predicate rejected");
        assert!(error.to_string().contains("predicate"));
    }

    #[test]
    fn fetched_chain_segment_rejects_tampered_bundle_bytes() {
        let source = temp_dir("chain-tamper-source");
        let destination = temp_dir("chain-tamper-destination");
        let iroh = temp_dir("chain-tamper-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-tamper", "epoch-1");
        let genesis = append_test_link(&source, &chain, None, "payload-a");
        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: None,
            expected_head: Some(&genesis.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("publish chain segment");
        fs::write(blob_path(&iroh, &published.bundle_ref).expect("blob path"), b"tampered").expect("tamper blob");
        let error = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect_err("tampered bundle rejected");
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn fetched_forked_chain_segment_requires_diagnostic_policy() {
        let source = temp_dir("chain-fork-source");
        let iroh = temp_dir("chain-fork-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-fork", "epoch-1");
        let genesis = import_test_link(&source, &chain, None, "payload-a");
        let first_child = import_test_link(&source, &chain, Some(&genesis), "payload-b");
        let _second_child = import_test_link(&source, &chain, Some(&genesis), "payload-c");
        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: None,
            expected_head: Some(&first_child.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RetainForkEvidence,
        })
        .expect("publish retained-fork segment");
        let production_destination = temp_dir("chain-fork-prod-destination");
        let production_error = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &production_destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect_err("production policy rejects fetched forks");
        assert!(production_error.to_string().contains("fork diagnostics"));
        let diagnostic_destination = temp_dir("chain-fork-diagnostic-destination");
        fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &diagnostic_destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RetainForkEvidence,
        })
        .expect("diagnostic policy retains fetched forks");
        let diagnostic_index =
            crate::evidence_chain::build_chain_index(&diagnostic_destination).expect("diagnostic index");
        assert!(diagnostic_index.heads_for_chain(&chain).contains(&first_child.link_ref));
        assert!(!diagnostic_index.fork_evidence_for_chain(&chain).is_empty());
    }

    fn remove_bundle_artifacts_of_kind(bundle: &IoValue, removed_kind: &str) -> IoValue {
        let fields = bundle.collect_simple_record("chain-segment-bundle-v1", Some(8)).expect("chain segment bundle");
        let artifacts_field = value_to_iovalue(&fields[4]);
        let artifacts = artifacts_field.collect_simple_record("artifacts", Some(1)).expect("artifacts field");
        let artifact_values = artifacts[0].collect_sequence().expect("artifact sequence");
        let filtered = artifact_values
            .iter()
            .filter_map(|artifact| {
                let artifact = value_to_iovalue(artifact);
                let artifact_record = artifact.collect_simple_record("artifact", Some(3)).expect("artifact record");
                let kind = required_string(&artifact_record[0], "artifact kind").expect("artifact kind");
                (kind != removed_kind).then_some(artifact)
            })
            .collect::<Vec<_>>();
        record("chain-segment-bundle-v1", vec![
            value_to_iovalue(&fields[0]),
            value_to_iovalue(&fields[1]),
            value_to_iovalue(&fields[2]),
            value_to_iovalue(&fields[3]),
            record("artifacts", vec![sequence(filtered)]),
            value_to_iovalue(&fields[5]),
            value_to_iovalue(&fields[6]),
            value_to_iovalue(&fields[7]),
        ])
    }

    fn append_test_link(
        root: &Path,
        chain: &crate::evidence_chain::ChainScope,
        previous: Option<&crate::evidence_chain::ChainLink>,
        payload_label: &str,
    ) -> crate::evidence_chain::ChainLink {
        let value = test_link_value(root, chain, previous, payload_label);
        let link = crate::evidence_chain::parse_chain_link(&value).expect("parse link");
        crate::evidence_chain::append_chain_link(root, &value).expect("append link");
        link
    }

    fn import_test_link(
        root: &Path,
        chain: &crate::evidence_chain::ChainScope,
        previous: Option<&crate::evidence_chain::ChainLink>,
        payload_label: &str,
    ) -> crate::evidence_chain::ChainLink {
        let value = test_link_value(root, chain, previous, payload_label);
        let link = crate::evidence_chain::parse_chain_link(&value).expect("parse link");
        crate::ledger::import_artifact(root, &value).expect("import raw link");
        link
    }

    fn test_link_value(
        root: &Path,
        chain: &crate::evidence_chain::ChainScope,
        previous: Option<&crate::evidence_chain::ChainLink>,
        payload_label: &str,
    ) -> IoValue {
        let payload = stored_payload(root, payload_label);
        match previous {
            Some(previous) => crate::evidence_chain::chain_link_value(&crate::evidence_chain::ChainLinkInput::append(
                previous,
                payload,
                Vec::new(),
                sample_producer(),
                ref_for(&format!("append-input-{payload_label}")),
            )),
            None => crate::evidence_chain::chain_link_value(&crate::evidence_chain::ChainLinkInput::genesis(
                chain.clone(),
                payload,
                Vec::new(),
                sample_producer(),
                ref_for(&format!("genesis-input-{payload_label}")),
            )),
        }
    }

    fn stored_payload(root: &Path, label: &str) -> crate::evidence_chain::ChainPayload {
        let artifact = record("test-payload", vec![string(label)]);
        let imported = crate::ledger::import_artifact(root, &artifact).expect("import payload");
        crate::evidence_chain::ChainPayload::new("test-payload", imported.artifact_ref, "molten.test.payload.v1")
    }

    fn checkpoint_range_predicate(root: &Path, verify: &crate::evidence_chain::ChainVerify) -> String {
        verify
            .predicate_receipt_refs
            .iter()
            .find(|predicate_ref| {
                let value = crate::ledger::read_artifact(root, predicate_ref).expect("read predicate receipt");
                crate::evidence_chain::parse_chain_predicate_receipt(&value)
                    .expect("parse predicate receipt")
                    .predicate
                    == crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE
            })
            .cloned()
            .expect("checkpoint range predicate ref")
    }

    fn checkpoint_checks() -> Vec<crate::evidence_chain::ChainCheck> {
        vec![
            crate::evidence_chain::ChainCheck::pass("raft-control-plane-command"),
            crate::evidence_chain::ChainCheck::pass("verified-range"),
            crate::evidence_chain::ChainCheck::pass("checkpoint-freshness"),
        ]
    }

    fn sample_producer() -> crate::evidence_chain::ChainProducer {
        crate::evidence_chain::ChainProducer::new("node:local", ref_for("producer-key"))
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
