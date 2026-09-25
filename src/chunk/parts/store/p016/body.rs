
const CHUNK_TRAVERSAL_SYNC_PLAN_SCHEMA: &str = "molten.chunk-store.traversal-sync-plan.v1";
const CHUNK_TRAVERSAL_SYNC_RECEIPT_SCHEMA: &str = "molten.chunk-store.traversal-sync-receipt.v1";
const REMOTE_BYTE_SOURCE_HINT_SCHEMA: &str = "molten.chunk-store.remote-byte-source-hint.v1";
const REMOTE_BYTE_SOURCE_READBACK_SCHEMA: &str = "molten.chunk-store.remote-byte-source-readback-receipt.v1";
const CHUNK_SYNC_STEM_FIRST: &str = "stem-first";
const CHUNK_SYNC_LEAF_ONLY: &str = "leaf-only";
const CHUNK_SYNC_PARTITIONED_LEAF: &str = "partitioned-leaf";
const CHUNK_SYNC_RESUMABLE_MISSING: &str = "resumable-missing";
const CHUNK_SYNC_DIAGNOSTIC_CAPACITY: usize = 12;
const MIN_CHUNK_SYNC_PEERS: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkFetchEffect {
    pub peer: String,
    pub chunk_ref: String,
    pub phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkTraversalSyncInput<'a> {
    pub manifest: &'a ChunkManifest,
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
    pub receipt_value: IoValue,
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
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteByteSourceHintInput<'a> {
    pub manifest_ref: &'a str,
    pub location: &'a str,
    pub outboard_ref: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteByteSourceHint {
    pub hint_ref: String,
    pub manifest_ref: String,
    pub location: String,
    pub outboard_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRangeReadbackInput<'a> {
    pub hint: &'a RemoteByteSourceHint,
    pub manifest: &'a ChunkManifest,
    pub offset: u64,
    pub length: u64,
    pub chunk_bytes: &'a [(String, Vec<u8>)],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRangeReadbackReceipt {
    pub decision: String,
    pub bytes: Vec<u8>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

pub fn plan_chunk_traversal_sync(input: &ChunkTraversalSyncInput<'_>) -> Result<ChunkTraversalSyncPlan> {
    validate_chunk_manifest_identity(input.manifest)?;
    validate_chunk_sync_strategy(input.strategy)?;
    validate_chunk_refs(input.verified_present_refs, "verified present chunk ref")?;
    if input.candidate_peers.len() < MIN_CHUNK_SYNC_PEERS {
        return Err(MoltenError::invalid_harness("chunk traversal sync requires at least one candidate peer"));
    }
    for peer in input.candidate_peers {
        if peer.trim().is_empty() {
            return Err(MoltenError::invalid_harness("chunk traversal sync peer must not be empty"));
        }
    }
    let present = input.verified_present_refs.iter().collect::<OrderedSet<_>>();
    let stem_refs = chunk_sync_stem_refs(input.manifest);
    let leaf_refs = input
        .manifest
        .chunks
        .iter()
        .map(|chunk| chunk.chunk_ref.clone())
        .collect::<Vec<_>>();
    let planned_refs = chunk_sync_strategy_refs(input.strategy, &stem_refs, &leaf_refs);
    let already_present_refs = planned_refs
        .iter()
        .filter(|reference| present.contains(reference))
        .cloned()
        .collect::<Vec<_>>();
    let missing_refs = planned_refs
        .iter()
        .filter(|reference| !present.contains(reference))
        .cloned()
        .collect::<Vec<_>>();
    let fetch_effects = partition_chunk_fetches(&missing_refs, input.candidate_peers, input.strategy);
    let diagnostics = Vec::with_capacity(CHUNK_SYNC_DIAGNOSTIC_CAPACITY);
    let receipt_value = chunk_sync_plan_receipt_value(SyncPlanReceiptInput { decision: "pass", manifest_ref: &input.manifest.manifest_ref, strategy: input.strategy, stem_refs: &stem_refs, leaf_refs: &leaf_refs, already_present_refs: &already_present_refs, missing_refs: &missing_refs, fetch_effects: &fetch_effects, diagnostics: &diagnostics });
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

pub fn validate_chunk_sync_response(input: &ChunkSyncResponseInput<'_>) -> Result<ChunkSyncResponseReceipt> {
    validate_content_ref(input.manifest_ref)?;
    validate_chunk_refs(input.returned_refs, "returned chunk ref")?;
    let mut diagnostics = Vec::with_capacity(CHUNK_SYNC_DIAGNOSTIC_CAPACITY);
    if input.manifest_ref != input.plan.manifest_ref {
        diagnostics.push(format!(
            "chunk traversal response manifest drift {} does not match {}",
            input.manifest_ref, input.plan.manifest_ref
        ));
    }
    let requested = input.plan.missing_refs.iter().collect::<OrderedSet<_>>();
    for reference in input.returned_refs {
        if !requested.contains(reference) {
            diagnostics.push(format!("chunk traversal response returned unexpected chunk {reference}"));
        }
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = record("chunk-traversal-sync-receipt-v1", vec![
        string(CHUNK_TRAVERSAL_SYNC_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest", vec![string(&input.plan.manifest_ref)]),
        record("returned-manifest", vec![string(input.manifest_ref)]),
        record("requested", vec![string_sequence(&input.plan.missing_refs)]),
        record("returned", vec![string_sequence(input.returned_refs)]),
        record("diagnostics", vec![string_sequence(&diagnostics)]),
        record("checks", vec![sequence(vec![
            check_record("manifest-identity-preserved", if diagnostics.is_empty() { "pass" } else { "fail" }),
            check_record("unexpected-chunks-not-indexed", if diagnostics.is_empty() { "pass" } else { "fail" }),
        ])]),
    ]);
    Ok(ChunkSyncResponseReceipt {
        decision: decision.to_string(),
        diagnostics,
        receipt_value,
    })
}

pub fn remote_byte_source_hint(input: &RemoteByteSourceHintInput<'_>) -> Result<RemoteByteSourceHint> {
    validate_content_ref(input.manifest_ref)?;
    validate_content_ref(input.outboard_ref)?;
    validate_chunk_refs(input.evidence_refs, "remote byte source evidence ref")?;
    validate_remote_location(input.location)?;
    let value = record("remote-byte-source-hint-v1", vec![
        string(REMOTE_BYTE_SOURCE_HINT_SCHEMA),
        record("manifest", vec![string(input.manifest_ref)]),
        record("location", vec![string(input.location)]),
        record("outboard", vec![string(input.outboard_ref)]),
        record("evidence", vec![string_sequence(input.evidence_refs)]),
        record("caveat", vec![string("remote byte-source hints are locations, not object identity or mutation authority")]),
    ]);
    let hint_ref = canonical_hash(&value)?;
    Ok(RemoteByteSourceHint {
        hint_ref,
        manifest_ref: input.manifest_ref.to_string(),
        location: input.location.to_string(),
        outboard_ref: input.outboard_ref.to_string(),
        value,
    })
}

pub fn verify_remote_range_readback(input: &RemoteRangeReadbackInput<'_>) -> Result<RemoteRangeReadbackReceipt> {
    validate_chunk_manifest_identity(input.manifest)?;
    let mut diagnostics = Vec::with_capacity(CHUNK_SYNC_DIAGNOSTIC_CAPACITY);
    if input.hint.manifest_ref != input.manifest.manifest_ref {
        diagnostics.push("remote byte-source hint manifest does not match requested manifest".to_string());
    }
    let required_chunks = required_chunks_for_remote_range(input.manifest, input.offset, input.length, &mut diagnostics)?;
    let supplied = input.chunk_bytes.iter().cloned().collect::<OrderedMap<_, _>>();
    let mut bytes = Vec::new();
    for chunk_ref in &required_chunks {
        match supplied.get(chunk_ref) {
            Some(chunk_bytes) => {
                let chunk_size = usize::try_from(input.manifest.chunk_size).map_err(|error| {
                    MoltenError::invalid_harness(format!("remote byte-source chunk size cannot fit usize: {error}"))
                })?;
                let actual_ref = hash_chunk(chunk_bytes, chunk_size);
                if actual_ref != *chunk_ref {
                    diagnostics.push(format!("remote byte-source chunk {chunk_ref} verified as {actual_ref}"));
                } else {
                    bytes.extend_from_slice(chunk_bytes);
                }
            }
            None => diagnostics.push(format!("remote byte-source missing required chunk {chunk_ref}")),
        }
    }
    if !diagnostics.is_empty() {
        bytes.clear();
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = record("remote-byte-source-readback-receipt-v1", vec![
        string(REMOTE_BYTE_SOURCE_READBACK_SCHEMA),
        record("decision", vec![string(decision)]),
        record("hint", vec![string(&input.hint.hint_ref)]),
        record("manifest", vec![string(&input.manifest.manifest_ref)]),
        record("offset", vec![string(input.offset.to_string())]),
        record("length", vec![string(input.length.to_string())]),
        record("required-chunks", vec![string_sequence(&required_chunks)]),
        record("diagnostics", vec![string_sequence(&diagnostics)]),
        record("checks", vec![sequence(vec![
            check_record("source-hint-not-identity", "pass"),
            check_record("range-verified-before-exposure", if diagnostics.is_empty() { "pass" } else { "fail" }),
        ])]),
    ]);
    Ok(RemoteRangeReadbackReceipt {
        decision: decision.to_string(),
        bytes,
        diagnostics,
        receipt_value,
    })
}

fn validate_chunk_manifest_identity(manifest: &ChunkManifest) -> Result<()> {
    validate_content_ref(&manifest.manifest_ref)?;
    validate_content_ref(&manifest.metadata_ref)?;
    validate_content_ref(&manifest.root_ref)?;
    for chunk in &manifest.chunks {
        validate_content_ref(&chunk.chunk_ref)?;
    }
    Ok(())
}

fn validate_chunk_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("expected canonical content ref for {label}, got {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_chunk_sync_strategy(strategy: &str) -> Result<()> {
    match strategy {
        CHUNK_SYNC_STEM_FIRST | CHUNK_SYNC_LEAF_ONLY | CHUNK_SYNC_PARTITIONED_LEAF | CHUNK_SYNC_RESUMABLE_MISSING => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported chunk traversal sync strategy {strategy}"))),
    }
}

fn validate_remote_location(location: &str) -> Result<()> {
    if location.starts_with("s3://") || location.starts_with("http://") || location.starts_with("https://") || location.starts_with("iroh://") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported remote byte-source location {location}")))
    }
}

fn chunk_sync_stem_refs(manifest: &ChunkManifest) -> Vec<String> {
    [manifest.metadata_ref.clone(), manifest.root_ref.clone()]
        .into_iter()
        .collect::<OrderedSet<_>>()
        .into_iter()
        .collect()
}
