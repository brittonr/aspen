
pub fn receipts(registry_root: &Path, ledger_root: Option<&Path>, input: &GraphInput) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = build_query_value(&QueryValueInput {
        operation: "receipts",
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: false,
        include_dependents: false,
        filters: &[Filter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: "redacted-receipts",
        include_payload: true,
    })?;
    let mut items = Vec::new();
    append_registry_receipt_views(registry_root, &full_ref, &input.visibility, &mut items)?;
    if let Some(ledger_root) = ledger_root {
        append_ledger_receipt_views(ledger_root, &full_ref, &input.visibility, &mut items)?;
    }
    finish_query("receipts", query_value, items, Vec::new())
}

pub fn chunk_store(chunk_root: &Path, input: &ChunkStoreInput) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    let scan = StoreScan::new(chunk_root, hidden_set(&input.visibility))?.scan()?;
    let dedup_hits = scan.dedup_hits();
    let mut items = Vec::new();
    push_bounded(
        &mut items,
        chunk_store_status_value(&ChunkStoreStatusInput {
            manifest_refs: &scan.visible_manifest_refs,
            total_chunk_refs: scan.total_chunk_refs,
            unique_chunks: scan.visible_unique_chunks.len(),
            available_chunks: scan.visible_available_chunks.len(),
            missing_chunks: scan.visible_missing_chunks.len(),
            manifest_pins: scan.visible_manifest_pins,
            chunk_pins: scan.visible_chunk_pins.len(),
            dedup_hits,
        })?,
        MAX_CATALOG_ITEMS,
        "chunk catalog items",
    )?;
    let query_value = build_query_value(&QueryValueInput {
        operation: "chunk-store",
        root_refs: &scan.visible_manifest_refs,
        include_dependencies: false,
        include_dependents: false,
        filters: &[Filter::Text("chunk-store:".to_string())],
        visibility: &input.visibility,
        render_mode: "chunk-store-status",
        include_payload: false,
    })?;
    for item in scan.manifest_items {
        push_bounded(&mut items, item, MAX_CATALOG_ITEMS, "chunk catalog items")?;
    }
    finish_query("chunk-store", query_value, items, Vec::new())
}

struct StoreScan<'a> {
    chunk_root: &'a Path,
    hidden: Set<String>,
    available_chunk_refs: Set<String>,
    visible_manifest_refs: Vec<String>,
    manifest_items: Vec<IoValue>,
    total_chunk_refs: usize,
    visible_unique_chunks: Set<String>,
    visible_available_chunks: Set<String>,
    visible_missing_chunks: Set<String>,
    visible_manifest_pins: usize,
    visible_chunk_pins: Set<String>,
}

#[derive(Default)]
struct ManifestScan {
    visible_chunks: Vec<String>,
    hidden_chunk_count: usize,
    available: usize,
    missing: usize,
    chunk_pins: usize,
}

impl<'a> StoreScan<'a> {
    fn new(chunk_root: &'a Path, hidden: Set<String>) -> Result<Self> {
        Ok(Self {
            chunk_root,
            hidden,
            available_chunk_refs: crate::chunk_store::list_chunk_refs(chunk_root)?.into_iter().collect(),
            visible_manifest_refs: Vec::new(),
            manifest_items: Vec::new(),
            total_chunk_refs: 0,
            visible_unique_chunks: Set::new(),
            visible_available_chunks: Set::new(),
            visible_missing_chunks: Set::new(),
            visible_manifest_pins: 0,
            visible_chunk_pins: Set::new(),
        })
    }

    fn scan(mut self) -> Result<Self> {
        for manifest_ref in crate::chunk_store::list_manifest_refs(self.chunk_root)? {
            self.add_manifest(manifest_ref)?;
        }
        Ok(self)
    }

    fn add_manifest(&mut self, manifest_ref: String) -> Result<()> {
        if self.hidden.contains(&manifest_ref) {
            return Ok(());
        }
        let manifest = crate::chunk_store::read_manifest(self.chunk_root, &manifest_ref)?;
        if self.hidden.contains(&manifest.root_ref) {
            return Ok(());
        }
        push_bounded(
            &mut self.visible_manifest_refs,
            manifest_ref.clone(),
            MAX_CATALOG_REFS,
            "chunk catalog manifest refs",
        )?;
        let is_manifest_pinned = crate::chunk_store::manifest_is_pinned(self.chunk_root, &manifest_ref)?;
        if is_manifest_pinned {
            self.visible_manifest_pins =
                checked_count_sum(self.visible_manifest_pins, 1, MAX_CATALOG_REFS, "chunk catalog pins")?;
        }
        let stats = self.add_chunks(&manifest.chunks)?;
        push_bounded(
            &mut self.manifest_items,
            chunk_manifest_value(&ChunkManifestInput {
                manifest: &manifest,
                is_manifest_pinned,
                available_chunks: stats.available,
                missing_chunks: stats.missing,
                chunk_pins: stats.chunk_pins,
                hidden_chunk_count: stats.hidden_chunk_count,
                visible_chunks: &stats.visible_chunks,
            })?,
            MAX_CATALOG_ITEMS,
            "chunk catalog items",
        )
    }

    fn add_chunks(&mut self, chunks: &[crate::chunk_store::ChunkRef]) -> Result<ManifestScan> {
        let mut stats = ManifestScan::default();
        for chunk in chunks {
            self.total_chunk_refs =
                checked_count_sum(self.total_chunk_refs, 1, MAX_CATALOG_REFS, "chunk catalog chunk refs")?;
            if self.hidden.contains(&chunk.chunk_ref) {
                stats.hidden_chunk_count =
                    checked_count_sum(stats.hidden_chunk_count, 1, MAX_CATALOG_REFS, "chunk catalog hidden chunks")?;
                continue;
            }
            insert_bounded(
                &mut self.visible_unique_chunks,
                chunk.chunk_ref.clone(),
                MAX_CATALOG_REFS,
                "chunk catalog unique chunks",
            )?;
            push_bounded(
                &mut stats.visible_chunks,
                chunk.chunk_ref.clone(),
                MAX_CATALOG_REFS,
                "chunk catalog visible chunks",
            )?;
            self.add_available(chunk, &mut stats)?;
            self.add_pin(chunk, &mut stats)?;
        }
        Ok(stats)
    }

    fn add_available(&mut self, chunk: &crate::chunk_store::ChunkRef, stats: &mut ManifestScan) -> Result<()> {
        if self.available_chunk_refs.contains(&chunk.chunk_ref) {
            stats.available = checked_count_sum(stats.available, 1, MAX_CATALOG_REFS, "chunk catalog available")?;
            insert_bounded(
                &mut self.visible_available_chunks,
                chunk.chunk_ref.clone(),
                MAX_CATALOG_REFS,
                "chunk catalog available chunks",
            )?;
        } else {
            stats.missing = checked_count_sum(stats.missing, 1, MAX_CATALOG_REFS, "chunk catalog missing")?;
            insert_bounded(
                &mut self.visible_missing_chunks,
                chunk.chunk_ref.clone(),
                MAX_CATALOG_REFS,
                "chunk catalog missing chunks",
            )?;
        }
        Ok(())
    }

    fn add_pin(&mut self, chunk: &crate::chunk_store::ChunkRef, stats: &mut ManifestScan) -> Result<()> {
        if crate::chunk_store::chunk_is_pinned(self.chunk_root, &chunk.chunk_ref)? {
            stats.chunk_pins = checked_count_sum(stats.chunk_pins, 1, MAX_CATALOG_REFS, "chunk catalog chunk pins")?;
            insert_bounded(
                &mut self.visible_chunk_pins,
                chunk.chunk_ref.clone(),
                MAX_CATALOG_REFS,
                "chunk catalog pinned chunks",
            )?;
        }
        Ok(())
    }

    fn dedup_hits(&self) -> usize {
        self.total_chunk_refs.saturating_sub(self.visible_unique_chunks.len())
    }
}

pub fn resolve_short_id(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &ShortIdInput,
) -> Result<ShortIdResolution> {
    validate_visibility(&input.visibility)?;
    validate_non_empty(&input.prefix, "catalog short id prefix")?;
    let query_value = build_query_value(&QueryValueInput {
        operation: "short-id",
        root_refs: &[],
        include_dependencies: false,
        include_dependents: false,
        filters: &[Filter::Text(input.prefix.clone())],
        visibility: &input.visibility,
        render_mode: "resolution",
        include_payload: false,
    })?;
    let query_ref = canonical_hash(&query_value)?;
    let prefix = classify_short_id_prefix(&input.prefix);
    let visible_candidates = visible_candidate_refs(registry_root, ledger_root, &input.visibility)?;
    let candidates = short_id_candidates(&prefix, visible_candidates, &input.prefix, input.min_length);
    let outcome = short_id_outcome(&prefix, &candidates, input.min_length);
    let value = short_id_resolution_value(
        &input.prefix,
        outcome.full_ref.as_deref(),
        &candidates,
        &outcome.decision,
        &outcome.diagnostics,
    )?;
    let result_value = short_id_result_value(ShortIdResultInput {
        query_ref: &query_ref,
        prefix: &prefix,
        min_length: input.min_length,
        value: &value,
        outcome: &outcome,
        candidates: &candidates,
    })?;
    let result_ref = canonical_hash(&result_value)?;
    let refs = short_id_refs(&candidates, &query_ref, &result_ref)?;
    let receipt_value = short_id_receipt_value(&query_ref, &result_ref, &refs, &outcome)?;
    Ok(ShortIdResolution {
        prefix: input.prefix.clone(),
        full_ref: outcome.full_ref,
        candidates,
        decision: outcome.decision,
        value,
        receipt_value,
    })
}

struct ShortIdOutcome {
    decision: String,
    full_ref: Option<String>,
    diagnostics: Vec<String>,
}

fn short_id_candidates(
    prefix: &ShortIdPrefix<'_>,
    candidates: Vec<String>,
    input_prefix: &str,
    min_length: usize,
) -> Vec<String> {
    match prefix {
        ShortIdPrefix::FullRef => candidates.into_iter().filter(|candidate| candidate == input_prefix).collect(),
        ShortIdPrefix::HexPrefix(hex_prefix) if hex_prefix.len() >= min_length => candidates
            .into_iter()
            .filter(|candidate| canonical_ref_matches_prefix(candidate, hex_prefix))
            .collect(),
        ShortIdPrefix::HexPrefix(_) | ShortIdPrefix::Deny(_) => Vec::new(),
    }
}
