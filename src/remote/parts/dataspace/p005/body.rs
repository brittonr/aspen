
const TRAVERSAL_DESCRIPTOR_SCHEMA: &str = "molten.remote-sync.traversal-descriptor.v1";
const TRAVERSAL_PLAN_RECEIPT_SCHEMA: &str = "molten.remote-sync.traversal-plan-receipt.v1";
const TRAVERSAL_RESPONSE_RECEIPT_SCHEMA: &str = "molten.remote-sync.traversal-response-receipt.v1";
const EXTERNAL_DIGEST_MAPPING_RECEIPT_SCHEMA: &str = "molten.remote-sync.external-digest-mapping-receipt.v1";
const TRAVERSAL_ARTIFACT_CLOSURE: &str = "artifact-closure";
const TRAVERSAL_CHUNK_MANIFEST: &str = "chunk-manifest";
const TRAVERSAL_JOB_OUTPUTS: &str = "job-dag-outputs";
const TRAVERSAL_SEQUENCE: &str = "sequence";
const TRAVERSAL_POLICY_DEFINED: &str = "policy-defined";
const TRAVERSAL_ORDER_LEXICOGRAPHIC: &str = "lexicographic";
const INLINE_POLICY_METADATA_ONLY: &str = "metadata-only";
const INLINE_POLICY_NONE: &str = "none";
const INLINE_POLICY_ALL: &str = "all";
const INLINE_POLICY_STEM_ONLY: &str = "stem-only";
const EXTERNAL_DIGEST_CID_SHA2_256: &str = "cid-sha2-256";
const EXTERNAL_DIGEST_CID_SHA2_512: &str = "cid-sha2-512";
const EXTERNAL_DIGEST_BLAKE3: &str = "blake3";
const TRAVERSAL_DIAGNOSTIC_CAPACITY: usize = 12;
const MIN_TRAVERSAL_BOUND: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalDescriptor {
    pub traversal_kind: String,
    pub root_refs: Vec<String>,
    pub visited_refs: Vec<String>,
    pub order: String,
    pub filters: Vec<String>,
    pub inline_policy: String,
    pub resource_bound: u64,
    pub replay_bound: u64,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalInventorySummary {
    pub verified_refs: Vec<String>,
    pub chunk_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalPlan {
    pub decision: String,
    pub descriptor_ref: String,
    pub local_inventory_ref: String,
    pub selected_refs: Vec<String>,
    pub already_present_refs: Vec<String>,
    pub fetch_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub replayable: bool,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalResponseInput<'a> {
    pub plan: &'a TraversalPlan,
    pub response_refs: &'a [String],
    pub inline_data_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalResponseReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDigestMappingInput<'a> {
    pub algorithm: &'a str,
    pub external_digest: &'a str,
    pub bytes: &'a [u8],
    pub expected_content_ref: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDigestMappingReceipt {
    pub decision: String,
    pub content_ref: String,
    pub external_digest: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

pub fn traversal_descriptor_value(descriptor: &TraversalDescriptor) -> Result<IoValue> {
    validate_traversal_descriptor(descriptor)?;
    Ok(record("remote-sync-traversal-descriptor-v1", vec![
        string(TRAVERSAL_DESCRIPTOR_SCHEMA),
        record("kind", vec![string(&descriptor.traversal_kind)]),
        record("roots", vec![string_sequence(&descriptor.root_refs)]),
        record("visited", vec![string_sequence(&descriptor.visited_refs)]),
        record("order", vec![string(&descriptor.order)]),
        record("filters", vec![string_sequence(&descriptor.filters)]),
        record("inline-policy", vec![string(&descriptor.inline_policy)]),
        record("resource-bound", vec![string(descriptor.resource_bound.to_string())]),
        record("replay-bound", vec![string(descriptor.replay_bound.to_string())]),
        record("policy", vec![string_sequence(&descriptor.policy_refs)]),
        record("evidence", vec![string_sequence(&descriptor.evidence_refs)]),
    ]))
}

pub fn plan_traversal(descriptor: &TraversalDescriptor, inventory: &LocalInventorySummary) -> Result<TraversalPlan> {
    let mut diagnostics = Vec::with_capacity(TRAVERSAL_DIAGNOSTIC_CAPACITY);
    collect_traversal_descriptor_diagnostics(descriptor, &mut diagnostics)?;
    validate_inventory_summary(inventory)?;
    let selected_refs = selected_traversal_refs(descriptor);
    let present = inventory_ref_set(inventory);
    let already_present_refs = selected_refs
        .iter()
        .filter(|reference| present.contains(*reference))
        .cloned()
        .collect::<Vec<_>>();
    let fetch_refs = selected_refs
        .iter()
        .filter(|reference| !present.contains(*reference))
        .cloned()
        .collect::<Vec<_>>();
    let descriptor_value = traversal_descriptor_value(descriptor).unwrap_or_else(|_| {
        record("remote-sync-traversal-descriptor-invalid-v1", vec![string(TRAVERSAL_DESCRIPTOR_SCHEMA)])
    });
    let descriptor_ref = canonical_hash(&descriptor_value)?;
    let local_inventory_ref = inventory_summary_ref(inventory);
    let replayable = diagnostics.is_empty();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = traversal_plan_receipt_value(
        decision,
        &descriptor_ref,
        &local_inventory_ref,
        &selected_refs,
        &already_present_refs,
        &fetch_refs,
        &diagnostics,
        replayable,
    );
    Ok(TraversalPlan {
        decision: decision.to_string(),
        descriptor_ref,
        local_inventory_ref,
        selected_refs,
        already_present_refs,
        fetch_refs,
        diagnostics,
        replayable,
        receipt_value,
    })
}

pub fn validate_traversal_response(input: &TraversalResponseInput<'_>) -> Result<TraversalResponseReceipt> {
    validate_traversal_refs(input.response_refs, "traversal response ref")?;
    validate_traversal_refs(input.inline_data_refs, "traversal inline data ref")?;
    let mut diagnostics = Vec::with_capacity(TRAVERSAL_DIAGNOSTIC_CAPACITY);
    let expected = input.plan.fetch_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    for reference in input.response_refs {
        if !expected.contains(reference) {
            diagnostics.push(format!("traversal response returned unrequested ref {reference}"));
        }
    }
    for reference in &input.plan.fetch_refs {
        if !input.response_refs.iter().any(|response| response == reference) {
            diagnostics.push(format!("traversal response missing expected ref {reference}"));
        }
    }
    if input.response_refs != input.plan.fetch_refs.as_slice() {
        diagnostics.push("traversal response order does not match deterministic fetch order".to_string());
    }
    if !input.inline_data_refs.is_empty() && input.plan.decision != "pass" {
        diagnostics.push("inline data cannot repair a denied traversal plan".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = record("remote-sync-traversal-response-receipt-v1", vec![
        string(TRAVERSAL_RESPONSE_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("descriptor", vec![string(&input.plan.descriptor_ref)]),
        record("expected-fetch", vec![string_sequence(&input.plan.fetch_refs)]),
        record("response", vec![string_sequence(input.response_refs)]),
        record("inline-data", vec![string_sequence(input.inline_data_refs)]),
        record("diagnostics", vec![string_sequence(&diagnostics)]),
        record("checks", vec![sequence(vec![
            check_record("receiver-selected-refs", if diagnostics.is_empty() { "pass" } else { "fail" }),
            check_record("no-unrequested-ref-import", if diagnostics.is_empty() { "pass" } else { "fail" }),
        ])]),
    ]);
    Ok(TraversalResponseReceipt {
        decision: decision.to_string(),
        diagnostics,
        receipt_value,
    })
}

pub fn validate_external_digest_mapping(input: &ExternalDigestMappingInput<'_>) -> Result<ExternalDigestMappingReceipt> {
    validate_external_digest_algorithm(input.algorithm)?;
    crate::preserves_rail::validate_content_ref(input.expected_content_ref)?;
    validate_traversal_refs(input.evidence_refs, "external digest mapping evidence ref")?;
    let actual_content_ref = content_ref_from_bytes(input.bytes);
    let actual_external_digest = external_digest_for(input.algorithm, input.bytes);
    let mut diagnostics = Vec::with_capacity(TRAVERSAL_DIAGNOSTIC_CAPACITY);
    if actual_content_ref != input.expected_content_ref {
        diagnostics.push(format!(
            "fetched bytes hash to {actual_content_ref}, expected {}",
            input.expected_content_ref
        ));
    }
    if actual_external_digest != input.external_digest {
        diagnostics.push(format!(
            "external digest {actual_external_digest} does not match claimed {}",
            input.external_digest
        ));
    }
    if input.evidence_refs.is_empty() {
        diagnostics.push("external digest mapping requires evidence refs before compatibility metadata is admitted".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = record("remote-sync-external-digest-mapping-receipt-v1", vec![
        string(EXTERNAL_DIGEST_MAPPING_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("algorithm", vec![string(input.algorithm)]),
        record("external-digest", vec![string(input.external_digest)]),
        record("content-ref", vec![string(&actual_content_ref)]),
        record("expected-content-ref", vec![string(input.expected_content_ref)]),
        record("evidence", vec![string_sequence(input.evidence_refs)]),
        record("diagnostics", vec![string_sequence(&diagnostics)]),
        record("checks", vec![sequence(vec![
            check_record("external-digest-verified", if diagnostics.is_empty() { "pass" } else { "fail" }),
            check_record("molten-blake3-identity-verified", if diagnostics.is_empty() { "pass" } else { "fail" }),
        ])]),
    ]);
    Ok(ExternalDigestMappingReceipt {
        decision: decision.to_string(),
        content_ref: actual_content_ref,
        external_digest: actual_external_digest,
        diagnostics,
        receipt_value,
    })
}

pub fn external_digest_for(algorithm: &str, bytes: &[u8]) -> String {
    if algorithm == EXTERNAL_DIGEST_BLAKE3 {
        return content_ref_from_bytes(bytes);
    }
    let mut material = Vec::with_capacity(algorithm.len() + bytes.len());
    material.extend_from_slice(algorithm.as_bytes());
    material.extend_from_slice(bytes);
    content_ref_from_bytes(&material)
}

fn validate_traversal_descriptor(descriptor: &TraversalDescriptor) -> Result<()> {
    collect_traversal_descriptor_diagnostics(descriptor, &mut Vec::new()).and_then(|()| {
        validate_traversal_refs(&descriptor.root_refs, "traversal root ref")?;
        validate_traversal_refs(&descriptor.visited_refs, "traversal visited ref")?;
        validate_traversal_refs(&descriptor.policy_refs, "traversal policy ref")?;
        validate_traversal_refs(&descriptor.evidence_refs, "traversal evidence ref")
    })
}

fn collect_traversal_descriptor_diagnostics(
    descriptor: &TraversalDescriptor,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if !matches!(
        descriptor.traversal_kind.as_str(),
        TRAVERSAL_ARTIFACT_CLOSURE
            | TRAVERSAL_CHUNK_MANIFEST
            | TRAVERSAL_JOB_OUTPUTS
            | TRAVERSAL_SEQUENCE
            | TRAVERSAL_POLICY_DEFINED
    ) {
        diagnostics.push(format!("unsupported traversal kind {}", descriptor.traversal_kind));
    }
    if descriptor.root_refs.is_empty() {
        diagnostics.push("traversal descriptor requires at least one root ref".to_string());
    }
    validate_traversal_refs(&descriptor.root_refs, "traversal root ref")?;
    validate_traversal_refs(&descriptor.visited_refs, "traversal visited ref")?;
    if descriptor.order != TRAVERSAL_ORDER_LEXICOGRAPHIC {
        diagnostics.push(format!("traversal order {} is not deterministic", descriptor.order));
    }
    if !matches!(
        descriptor.inline_policy.as_str(),
        INLINE_POLICY_METADATA_ONLY | INLINE_POLICY_NONE | INLINE_POLICY_ALL | INLINE_POLICY_STEM_ONLY
    ) {
        diagnostics.push(format!("unsupported traversal inline policy {}", descriptor.inline_policy));
    }
    if descriptor.resource_bound < MIN_TRAVERSAL_BOUND || descriptor.replay_bound < MIN_TRAVERSAL_BOUND {
        diagnostics.push("traversal resource and replay bounds must be positive".to_string());
    }
    validate_traversal_refs(&descriptor.policy_refs, "traversal policy ref")?;
    validate_traversal_refs(&descriptor.evidence_refs, "traversal evidence ref")?;
    Ok(())
}

fn validate_inventory_summary(inventory: &LocalInventorySummary) -> Result<()> {
    validate_traversal_refs(&inventory.verified_refs, "inventory verified ref")?;
    validate_traversal_refs(&inventory.chunk_refs, "inventory chunk ref")
}

fn selected_traversal_refs(descriptor: &TraversalDescriptor) -> Vec<String> {
    let visited = descriptor.visited_refs.iter().collect::<std::collections::BTreeSet<_>>();
    descriptor
        .root_refs
        .iter()
        .filter(|reference| !visited.contains(reference))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn inventory_ref_set(inventory: &LocalInventorySummary) -> std::collections::BTreeSet<String> {
    inventory
        .verified_refs
        .iter()
        .chain(inventory.chunk_refs.iter())
        .cloned()
        .collect()
}

fn inventory_summary_ref(inventory: &LocalInventorySummary) -> String {
    let value = record("remote-sync-local-inventory-summary-v1", vec![
        record("verified", vec![string_sequence(&inventory.verified_refs)]),
        record("chunks", vec![string_sequence(&inventory.chunk_refs)]),
    ]);
    canonical_hash(&value).unwrap_or_else(|_| content_ref_from_bytes(b"invalid-local-inventory-summary"))
}

fn traversal_plan_receipt_value(
    decision: &str,
    descriptor_ref: &str,
    local_inventory_ref: &str,
    selected_refs: &[String],
    already_present_refs: &[String],
    fetch_refs: &[String],
    diagnostics: &[String],
    replayable: bool,
) -> IoValue {
    record("remote-sync-traversal-plan-receipt-v1", vec![
        string(TRAVERSAL_PLAN_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("descriptor", vec![string(descriptor_ref)]),
        record("local-inventory", vec![string(local_inventory_ref)]),
        record("selected", vec![string_sequence(selected_refs)]),
        record("already-present", vec![string_sequence(already_present_refs)]),
        record("fetch", vec![string_sequence(fetch_refs)]),
        record("diagnostics", vec![string_sequence(diagnostics)]),
        record("replayable", vec![string(replayable.to_string())]),
        record("checks", vec![sequence(vec![
            check_record("deterministic-order", if replayable { "pass" } else { "fail" }),
            check_record("receiver-driven-missing-set", if replayable { "pass" } else { "fail" }),
        ])]),
    ])
}

fn validate_traversal_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("expected canonical content ref for {label}, got {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_external_digest_algorithm(algorithm: &str) -> Result<()> {
    match algorithm {
        EXTERNAL_DIGEST_CID_SHA2_256 | EXTERNAL_DIGEST_CID_SHA2_512 | EXTERNAL_DIGEST_BLAKE3 => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported external digest algorithm {algorithm}"))),
    }
}

fn string_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn check_record(name: &str, status: &str) -> IoValue {
    record("check", vec![string(name), string(status)])
}
