
pub fn evaluate_chunk_availability(input: ChunkAvailabilityInput<'_>) -> ChunkAvailabilityDecision {
    let manifest_refs = input
        .manifest
        .chunks
        .iter()
        .map(|chunk| chunk.chunk_ref.clone())
        .collect::<OrderedSet<_>>();
    let available = ref_set(input.available_chunk_refs);
    let missing = ref_set(input.missing_chunk_refs);
    let indexed_available = ref_set(input.indexed_available_refs);
    let indexed_missing = ref_set(input.indexed_missing_refs);
    let partial_missing = ref_set(input.partial_fetch_missing_refs);
    let partial_fetched = ref_set(input.partial_fetch_fetched_refs);

    let mut diagnostics = Vec::with_capacity(CHUNK_AVAILABILITY_DIAGNOSTIC_CAPACITY);
    let union = available.union(&missing).cloned().collect::<OrderedSet<_>>();
    if union != manifest_refs || !available.is_disjoint(&missing) {
        diagnostics.push("availability-partition-mismatch".to_string());
    }
    if indexed_available != available || indexed_missing != missing {
        diagnostics.push("index-availability-mismatch".to_string());
    }
    if !missing.is_empty() {
        diagnostics.push("chunk-missing".to_string());
    }
    if !available.is_subset(&manifest_refs)
        || !missing.is_subset(&manifest_refs)
        || !indexed_available.is_subset(&manifest_refs)
        || !indexed_missing.is_subset(&manifest_refs)
        || !partial_missing.is_subset(&manifest_refs)
        || !partial_fetched.is_subset(&manifest_refs)
    {
        diagnostics.push("unknown-chunk-ref".to_string());
    }
    if !partial_missing.is_empty()
        && (!partial_missing.is_subset(&manifest_refs) || !partial_fetched.is_subset(&manifest_refs))
    {
        diagnostics.push("partial-fetch-mismatch".to_string());
    }
    if !partial_fetched.is_disjoint(&missing) {
        diagnostics.push("partial-fetch-repair-incomplete".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    ChunkAvailabilityDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn ref_set(refs: &[String]) -> OrderedSet<String> {
    refs.iter().cloned().collect()
}
