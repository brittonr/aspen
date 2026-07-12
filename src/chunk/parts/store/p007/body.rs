
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

fn matching_apply_ref<'a>(input: ApplyRefMatchInput<'a>) -> Option<&'a str> {
    let mut fallback_ref = None;
    for apply_ref in input.apply_refs {
        let Ok(apply) = crate::retention::read_gc_apply_with_root(input.root, apply_ref) else {
            if fallback_ref.is_none() {
                fallback_ref = Some(apply_ref.as_str());
            }
            continue;
        };
        if apply.decision == "pass"
            && apply.subsystem == input.subsystem
            && apply.action == input.action
            && apply.object_ref == input.object_ref
            && apply.object_kind == input.object_kind
            && apply.retention_class == input.retention_class
        {
            return Some(apply_ref.as_str());
        }
        if fallback_ref.is_none() {
            fallback_ref = Some(apply_ref.as_str());
        }
    }
    fallback_ref
}

struct GcTargets {
    manifests: Vec<String>,
    chunks: Vec<String>,
}

fn gc_targets(
    root: &CapabilityChunkRoot,
    pinned_manifests: Vec<String>,
    mut reachable_chunks: Vec<String>,
) -> Result<GcTargets> {
    let mut manifests = Vec::new();
    for manifest_ref in list_manifest_refs_with_root(root)? {
        if pinned_manifests.iter().any(|pinned| pinned == &manifest_ref) {
            let manifest = read_manifest_with_root(root, &manifest_ref)?;
            for chunk in manifest.chunks {
                if !reachable_chunks.iter().any(|reachable| reachable == &chunk.chunk_ref) {
                    push_bounded(
                        &mut reachable_chunks,
                        chunk.chunk_ref,
                        MAX_CHUNK_STORE_CHUNKS,
                        "chunk store reachable chunks",
                    )?;
                }
            }
        } else {
            push_bounded(
                &mut manifests,
                manifest_ref.clone(),
                MAX_CHUNK_STORE_MANIFESTS,
                "chunk store removed manifests",
            )?;
        }
    }
    let mut chunks = Vec::new();
    for chunk_ref in list_chunk_refs_with_root(root)? {
        if reachable_chunks.iter().any(|reachable| reachable == &chunk_ref) {
            continue;
        }
        push_bounded(&mut chunks, chunk_ref.clone(), MAX_CHUNK_STORE_CHUNKS, "chunk store removed chunks")?;
    }
    Ok(GcTargets { manifests, chunks })
}

struct GcEnv<'a> {
    retention_root: &'a crate::local_store::RetentionStoreRoot,
    is_dry_run: bool,
    evidence: &'a crate::retention::DestructiveEvidence,
    apply_refs: &'a [String],
    action: &'a str,
    requester_ref: &'a str,
}

#[derive(Clone, Copy)]
struct GcObject<'a> {
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

#[derive(Default)]
struct GcNotes {
    admission_diagnostics: Vec<String>,
    execution_diagnostics: Vec<String>,
    admission_refs: Vec<String>,
    receipts: Vec<String>,
    execution_gates: Vec<String>,
    denials: Vec<String>,
}

impl GcNotes {
    fn consider(&mut self, env: &GcEnv<'_>, object: GcObject<'_>) -> Result<()> {
        let admission = crate::retention::admit_destructive_evidence_with_root(
            crate::retention::DestructiveAdmissionInput {
                root: env.retention_root,
                evidence: env.evidence,
                object_ref: object.object_ref,
                object_kind: object.object_kind,
                retention_class: object.retention_class,
                action: env.action,
            },
        )?;
        self.note_admission(&admission)?;
        let evaluation = crate::retention::evaluate_with_root(crate::retention::EvaluationInput {
            root: env.retention_root,
            object_ref: object.object_ref,
            object_kind: object.object_kind,
            retention_class: object.retention_class,
            action: env.action,
            requester_ref: env.requester_ref,
            is_reference_index_complete: env.evidence.is_reference_index_complete,
            retained_refs: &env.evidence.retained_refs,
            remote_refs: &env.evidence.remote_refs,
            policy_refs: &env.evidence.policy_refs,
            evidence_refs: &env.evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })?;
        push_bounded(
            &mut self.receipts,
            evaluation.receipt.receipt_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk store retention receipt refs",
        )?;
        let is_execution_denied = if env.is_dry_run {
            false
        } else {
            self.note_execution(env, object)?
        };
        if admission.decision != "pass" || evaluation.receipt.decision != "pass" || is_execution_denied {
            push_bounded(
                &mut self.denials,
                object.object_ref.to_string(),
                MAX_CHUNK_STORE_REFS,
                "chunk store retention denials",
            )?;
        }
        Ok(())
    }

    fn note_admission(&mut self, admission: &crate::retention::DestructiveAdmission) -> Result<()> {
        for diagnostic in &admission.diagnostics {
            push_bounded(
                &mut self.admission_diagnostics,
                diagnostic.clone(),
                MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention admission diagnostics",
            )?;
        }
        for reference in &admission.admitted_refs {
            push_bounded(
                &mut self.admission_refs,
                reference.clone(),
                MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention admission refs",
            )?;
        }
        Ok(())
    }

    fn note_execution(&mut self, env: &GcEnv<'_>, object: GcObject<'_>) -> Result<bool> {
        let apply_ref = matching_apply_ref(ApplyRefMatchInput {
            root: env.retention_root,
            apply_refs: env.apply_refs,
            subsystem: "chunk-gc",
            action: env.action,
            object_ref: object.object_ref,
            object_kind: object.object_kind,
            retention_class: object.retention_class,
        });
        let execution_gate =
            crate::retention::store_gc_execution_gate_with_root(crate::retention::GcExecutionGateInput {
            root: env.retention_root,
            subsystem: "chunk-gc",
            action: env.action,
            object_ref: object.object_ref,
            object_kind: object.object_kind,
            retention_class: object.retention_class,
            apply_ref,
        })?;
        push_bounded(
            &mut self.execution_gates,
            execution_gate.execution_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk store retention execution gate refs",
        )?;
        if execution_gate.decision == "pass" {
            return Ok(false);
        }
        for diagnostic in &execution_gate.diagnostics {
            push_bounded(
                &mut self.execution_diagnostics,
                diagnostic.clone(),
                MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention execution diagnostics",
            )?;
        }
        Ok(true)
    }
}

#[derive(Clone, Copy)]
struct GcReceiptInput<'a> {
    is_dry_run: bool,
    decision: &'a str,
    removed_manifests: &'a [String],
    removed_chunks: &'a [String],
    notes: &'a GcNotes,
    evidence_summary: &'a IoValue,
}

fn gc_receipt_value(input: GcReceiptInput<'_>) -> IoValue {
    receipt_value(ChunkStoreReceiptValueInput {
        operation: "gc",
        decision: input.decision,
        manifest_ref: None,
        chunk_refs: input.removed_chunks,
        checks: vec![
            ("pin-reachability", "pass"),
            ("deny-incomplete-reachability-proof", "pass"),
            ("chunk-tombstone-eligibility", if input.decision == "pass" { "pass" } else { "fail" }),
            ("retention-receipt-bound", "pass"),
            (
                "retention-execution-gate",
                pass_or_fail(input.is_dry_run || input.notes.execution_diagnostics.is_empty()),
            ),
            ("retention-authority-evidence", pass_or_fail(input.notes.admission_diagnostics.is_empty())),
            ("redb-index-update", if input.decision == "pass" { "pass" } else { "fail" }),
        ],
        details: vec![
            record("mode", vec![string(if input.is_dry_run { "dry-run" } else { "apply" })]),
            record("removed-manifests", vec![sequence(input.removed_manifests.iter().map(string).collect())]),
            record("retention", vec![sequence(input.notes.receipts.iter().map(string).collect())]),
            record("retention-execution", vec![sequence(input.notes.execution_gates.iter().map(string).collect())]),
            record("denied", vec![sequence(input.notes.denials.iter().map(string).collect())]),
            record("retention-evidence", vec![input.evidence_summary.clone()]),
            record("retention-admission", vec![sequence(input.notes.admission_refs.iter().map(string).collect())]),
            record("retention-diagnostics", vec![sequence(
                input.notes.admission_diagnostics.iter().map(string).collect(),
            )]),
            record("retention-execution-diagnostics", vec![sequence(
                input.notes.execution_diagnostics.iter().map(string).collect(),
            )]),
        ],
    })
}

fn gc_tombstone_value(input: GcReceiptInput<'_>) -> Option<IoValue> {
    if input.is_dry_run
        || input.decision != "pass"
        || (input.removed_manifests.is_empty() && input.removed_chunks.is_empty())
    {
        return None;
    }
    Some(self::receipt_value(ChunkStoreReceiptValueInput {
        operation: "tombstone",
        decision: "pass",
        manifest_ref: None,
        chunk_refs: input.removed_chunks,
        checks: vec![
            ("pin-reachability", "pass"),
            ("tombstone-eligibility", "pass"),
            ("gc-mode-binding", "pass"),
            ("retention-receipt-bound", "pass"),
            ("retention-execution-gate", "pass"),
            ("retention-authority-evidence", "pass"),
        ],
        details: vec![
            record("mode", vec![string("apply")]),
            record("removed-manifests", vec![sequence(input.removed_manifests.iter().map(string).collect())]),
            record("retention", vec![sequence(input.notes.receipts.iter().map(string).collect())]),
            record("retention-execution", vec![sequence(input.notes.execution_gates.iter().map(string).collect())]),
            record("retention-evidence", vec![input.evidence_summary.clone()]),
            record("retention-admission", vec![sequence(input.notes.admission_refs.iter().map(string).collect())]),
        ],
    }))
}

struct GcFinishInput<'a> {
    root: &'a CapabilityChunkRoot,
    is_dry_run: bool,
    targets: GcTargets,
    notes: GcNotes,
    evidence_summary: IoValue,
}
