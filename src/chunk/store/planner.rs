//! Receiver-driven traversal sync plans and response validation.

const CHUNK_TRAVERSAL_SYNC_PLAN_SCHEMA: &str = "molten.chunk-store.traversal-sync-plan.v1";
const CHUNK_TRAVERSAL_SYNC_RECEIPT_SCHEMA: &str = "molten.chunk-store.traversal-sync-receipt.v1";
const CHUNK_SYNC_STEM_FIRST: &str = "stem-first";
const CHUNK_SYNC_LEAF_ONLY: &str = "leaf-only";
pub(super) const CHUNK_SYNC_PARTITIONED_LEAF: &str = "partitioned-leaf";
const CHUNK_SYNC_RESUMABLE_MISSING: &str = "resumable-missing";
pub(super) const CHUNK_SYNC_DIAGNOSTIC_CAPACITY: usize = 12;
const MIN_CHUNK_SYNC_PEERS: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkFetchEffect {
    pub peer: String,
    pub chunk_ref: String,
    pub phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkTraversalSyncInput<'a> {
    pub manifest: &'a crate::blocks::ChunkManifest,
    pub verified_present_refs: &'a [String],
    pub candidate_peers: &'a [String],
    pub strategy: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkTraversalSyncPlan {
    pub decision: String,
    pub manifest_ref: String,
    pub stem_refs: Vec<String>,
    pub leaf_refs: Vec<String>,
    pub already_present_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub fetch_effects: Vec<ChunkFetchEffect>,
    pub diagnostics: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSyncResponseInput<'a> {
    pub plan: &'a ChunkTraversalSyncPlan,
    pub manifest_ref: &'a str,
    pub returned_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSyncResponseReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

pub fn plan_chunk_traversal_sync(input: &ChunkTraversalSyncInput<'_>) -> crate::error::Result<ChunkTraversalSyncPlan> {
    validate_chunk_manifest_identity(input.manifest)?;
    validate_chunk_sync_strategy(input.strategy)?;
    validate_chunk_refs(input.verified_present_refs, "verified present chunk ref")?;
    if input.candidate_peers.len() < MIN_CHUNK_SYNC_PEERS {
        return Err(crate::error::Failure::invalid_harness(
            "chunk traversal sync requires at least one candidate peer",
        ));
    }
    for peer in input.candidate_peers {
        if peer.trim().is_empty() {
            return Err(crate::error::Failure::invalid_harness("chunk traversal sync peer must not be empty"));
        }
    }
    let present = input.verified_present_refs.iter().collect::<std::collections::BTreeSet<_>>();
    let stem_refs = chunk_sync_stem_refs(input.manifest);
    let leaf_refs = input.manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect::<Vec<_>>();
    let planned_refs = chunk_sync_strategy_refs(input.strategy, &stem_refs, &leaf_refs);
    let already_present_refs =
        planned_refs.iter().filter(|reference| present.contains(reference)).cloned().collect::<Vec<_>>();
    let missing_refs =
        planned_refs.iter().filter(|reference| !present.contains(reference)).cloned().collect::<Vec<_>>();
    let fetch_effects = partition_chunk_fetches(&missing_refs, input.candidate_peers, input.strategy);
    let diagnostics = Vec::with_capacity(CHUNK_SYNC_DIAGNOSTIC_CAPACITY);
    let receipt_value = chunk_sync_plan_receipt_value(SyncPlanReceiptInput {
        decision: "pass",
        manifest_ref: &input.manifest.manifest_ref,
        strategy: input.strategy,
        stem_refs: &stem_refs,
        leaf_refs: &leaf_refs,
        already_present_refs: &already_present_refs,
        missing_refs: &missing_refs,
        fetch_effects: &fetch_effects,
        diagnostics: &diagnostics,
    });
    Ok(ChunkTraversalSyncPlan {
        decision: "pass".to_string(),
        manifest_ref: input.manifest.manifest_ref.clone(),
        stem_refs,
        leaf_refs,
        already_present_refs,
        missing_refs,
        fetch_effects,
        diagnostics,
        receipt_value,
    })
}

pub fn validate_chunk_sync_response(
    input: &ChunkSyncResponseInput<'_>,
) -> crate::error::Result<ChunkSyncResponseReceipt> {
    crate::preserves_rail::validate_content_ref(input.manifest_ref)?;
    validate_chunk_refs(input.returned_refs, "returned chunk ref")?;
    let mut diagnostics = Vec::with_capacity(CHUNK_SYNC_DIAGNOSTIC_CAPACITY);
    if input.manifest_ref != input.plan.manifest_ref {
        diagnostics.push(format!(
            "chunk traversal response manifest drift {} does not match {}",
            input.manifest_ref, input.plan.manifest_ref
        ));
    }
    let requested = input.plan.missing_refs.iter().collect::<std::collections::BTreeSet<_>>();
    for reference in input.returned_refs {
        if !requested.contains(reference) {
            diagnostics.push(format!("chunk traversal response returned unexpected chunk {reference}"));
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = crate::preserves_rail::record("chunk-traversal-sync-receipt-v1", vec![
        crate::preserves_rail::string(CHUNK_TRAVERSAL_SYNC_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(&input.plan.manifest_ref)]),
        crate::preserves_rail::record("returned-manifest", vec![crate::preserves_rail::string(input.manifest_ref)]),
        crate::preserves_rail::record("requested", vec![crate::blocks::wire::encode::string_sequence(
            &input.plan.missing_refs,
        )]),
        crate::preserves_rail::record("returned", vec![crate::blocks::wire::encode::string_sequence(
            input.returned_refs,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::blocks::wire::encode::string_sequence(&diagnostics)]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::blocks::wire::encode::check_record(
                "manifest-identity-preserved",
                if diagnostics.is_empty() { "pass" } else { "fail" },
            ),
            crate::blocks::wire::encode::check_record(
                "unexpected-chunks-not-indexed",
                if diagnostics.is_empty() { "pass" } else { "fail" },
            ),
        ])]),
    ]);
    Ok(ChunkSyncResponseReceipt {
        decision: decision.to_string(),
        diagnostics,
        receipt_value,
    })
}

pub(super) fn validate_chunk_manifest_identity(manifest: &crate::blocks::ChunkManifest) -> crate::error::Result<()> {
    crate::bounded::ensure_count_at_most(
        manifest.chunks.len(),
        crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
        "chunk manifest chunks",
    )?;
    crate::preserves_rail::validate_content_ref(&manifest.manifest_ref)?;
    crate::preserves_rail::validate_content_ref(&manifest.metadata_ref)?;
    crate::preserves_rail::validate_content_ref(&manifest.root_ref)?;
    for chunk in &manifest.chunks {
        crate::preserves_rail::validate_content_ref(&chunk.chunk_ref)?;
    }
    Ok(())
}

pub(super) fn validate_chunk_refs(refs: &[String], label: &str) -> crate::error::Result<()> {
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
            crate::error::Failure::invalid_harness(format!(
                "expected canonical content ref for {label}, got {reference}: {error}"
            ))
        })?;
    }
    Ok(())
}

fn validate_chunk_sync_strategy(strategy: &str) -> crate::error::Result<()> {
    match strategy {
        CHUNK_SYNC_STEM_FIRST | CHUNK_SYNC_LEAF_ONLY | CHUNK_SYNC_PARTITIONED_LEAF | CHUNK_SYNC_RESUMABLE_MISSING => {
            Ok(())
        }
        _ => Err(crate::error::Failure::invalid_harness(format!(
            "unsupported chunk traversal sync strategy {strategy}"
        ))),
    }
}

fn chunk_sync_stem_refs(manifest: &crate::blocks::ChunkManifest) -> Vec<String> {
    [manifest.metadata_ref.clone(), manifest.root_ref.clone()]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn chunk_sync_strategy_refs(strategy: &str, stem_refs: &[String], leaf_refs: &[String]) -> Vec<String> {
    match strategy {
        CHUNK_SYNC_STEM_FIRST => stem_refs.iter().chain(leaf_refs.iter()).cloned().collect(),
        CHUNK_SYNC_LEAF_ONLY | CHUNK_SYNC_PARTITIONED_LEAF | CHUNK_SYNC_RESUMABLE_MISSING => leaf_refs.to_vec(),
        _ => Vec::new(),
    }
}

fn partition_chunk_fetches(chunk_refs: &[String], peers: &[String], strategy: &str) -> Vec<ChunkFetchEffect> {
    chunk_refs
        .iter()
        .enumerate()
        .map(|(index, chunk_ref)| {
            let peer_index = if strategy == CHUNK_SYNC_PARTITIONED_LEAF {
                index.checked_rem(peers.len()).map_or(0, |remainder| remainder)
            } else {
                0
            };
            ChunkFetchEffect {
                peer: peers[peer_index].clone(),
                chunk_ref: chunk_ref.clone(),
                phase: if strategy == CHUNK_SYNC_STEM_FIRST {
                    "stem-or-leaf"
                } else {
                    "leaf"
                }
                .to_string(),
            }
        })
        .collect()
}

struct SyncPlanReceiptInput<'a> {
    decision: &'a str,
    manifest_ref: &'a str,
    strategy: &'a str,
    stem_refs: &'a [String],
    leaf_refs: &'a [String],
    already_present_refs: &'a [String],
    missing_refs: &'a [String],
    fetch_effects: &'a [ChunkFetchEffect],
    diagnostics: &'a [String],
}

fn chunk_sync_plan_receipt_value(input: SyncPlanReceiptInput<'_>) -> preserves::IOValue {
    let SyncPlanReceiptInput {
        decision,
        manifest_ref,
        strategy,
        stem_refs,
        leaf_refs,
        already_present_refs,
        missing_refs,
        fetch_effects,
        diagnostics,
    } = input;
    crate::preserves_rail::record("chunk-traversal-sync-plan-v1", vec![
        crate::preserves_rail::string(CHUNK_TRAVERSAL_SYNC_PLAN_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(manifest_ref)]),
        crate::preserves_rail::record("strategy", vec![crate::preserves_rail::string(strategy)]),
        crate::preserves_rail::record("stem", vec![crate::blocks::wire::encode::string_sequence(stem_refs)]),
        crate::preserves_rail::record("leaves", vec![crate::blocks::wire::encode::string_sequence(leaf_refs)]),
        crate::preserves_rail::record("already-present", vec![crate::blocks::wire::encode::string_sequence(
            already_present_refs,
        )]),
        crate::preserves_rail::record("missing", vec![crate::blocks::wire::encode::string_sequence(missing_refs)]),
        crate::preserves_rail::record("fetch-effects", vec![crate::preserves_rail::sequence(
            fetch_effects.iter().map(chunk_fetch_effect_value).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::blocks::wire::encode::string_sequence(diagnostics)]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::blocks::wire::encode::check_record("manifest-identity-preserved", "pass"),
            crate::blocks::wire::encode::check_record("receiver-driven-missing-set", "pass"),
        ])]),
    ])
}

fn chunk_fetch_effect_value(effect: &ChunkFetchEffect) -> preserves::IOValue {
    crate::preserves_rail::record("fetch", vec![
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(&effect.peer)]),
        crate::preserves_rail::record("chunk", vec![crate::preserves_rail::string(&effect.chunk_ref)]),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(&effect.phase)]),
    ])
}
