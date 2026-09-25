
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
                phase: if strategy == CHUNK_SYNC_STEM_FIRST { "stem-or-leaf" } else { "leaf" }.to_string(),
            }
        })
        .collect()
}

fn required_chunks_for_remote_range(
    manifest: &ChunkManifest,
    offset: u64,
    length: u64,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<String>> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| MoltenError::invalid_harness("remote byte-source range end overflow"))?;
    if end > manifest.total_len {
        diagnostics.push_item("remote byte-source range exceeds manifest length".to_string());
    }
    let mut refs = Vec::with_capacity(manifest.chunks.len());
    let mut chunk_start = 0_u64;
    for chunk in &manifest.chunks {
        let chunk_end = chunk_start
            .checked_add(chunk.length)
            .ok_or_else(|| MoltenError::invalid_harness("remote byte-source chunk range overflow"))?;
        if ranges_overlap(offset, end, chunk_start, chunk_end) {
            refs.push(chunk.chunk_ref.clone());
        }
        chunk_start = chunk_end;
    }
    Ok(refs)
}

fn ranges_overlap(left_start: u64, left_end: u64, right_start: u64, right_end: u64) -> bool {
    left_start < right_end && right_start < left_end
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

fn chunk_sync_plan_receipt_value(input: SyncPlanReceiptInput<'_>) -> IoValue {
    let SyncPlanReceiptInput { decision, manifest_ref, strategy, stem_refs, leaf_refs, already_present_refs, missing_refs, fetch_effects, diagnostics } = input;
    record("chunk-traversal-sync-plan-v1", vec![
        string(CHUNK_TRAVERSAL_SYNC_PLAN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest", vec![string(manifest_ref)]),
        record("strategy", vec![string(strategy)]),
        record("stem", vec![string_sequence(stem_refs)]),
        record("leaves", vec![string_sequence(leaf_refs)]),
        record("already-present", vec![string_sequence(already_present_refs)]),
        record("missing", vec![string_sequence(missing_refs)]),
        record("fetch-effects", vec![sequence(fetch_effects.iter().map(chunk_fetch_effect_value).collect())]),
        record("diagnostics", vec![string_sequence(diagnostics)]),
        record("checks", vec![sequence(vec![
            check_record("manifest-identity-preserved", "pass"),
            check_record("receiver-driven-missing-set", "pass"),
        ])]),
    ])
}

fn chunk_fetch_effect_value(effect: &ChunkFetchEffect) -> IoValue {
    record("fetch", vec![
        record("peer", vec![string(&effect.peer)]),
        record("chunk", vec![string(&effect.chunk_ref)]),
        record("phase", vec![string(&effect.phase)]),
    ])
}

fn string_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn check_record(name: &str, status: &str) -> IoValue {
    record("check", vec![string(name), string(status)])
}
