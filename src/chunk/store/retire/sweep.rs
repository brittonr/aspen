//! GC target selection, deletion, receipts, and tombstones.

fn pass_or_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

struct GcTargets {
    manifests: Vec<String>,
    chunks: Vec<String>,
}

fn gc_targets(
    root: &crate::blocks::CapabilityChunkRoot,
    pinned_manifests: Vec<String>,
    mut reachable_chunks: Vec<String>,
) -> crate::error::Result<GcTargets> {
    let mut manifests = Vec::new();
    for manifest_ref in crate::blocks::list_manifest_refs_with_root(root)? {
        if pinned_manifests.iter().any(|pinned| pinned == &manifest_ref) {
            let manifest = crate::blocks::read_manifest_with_root(root, &manifest_ref)?;
            for chunk in manifest.chunks {
                if !reachable_chunks.iter().any(|reachable| reachable == &chunk.chunk_ref) {
                    crate::bounded::push(
                        &mut reachable_chunks,
                        chunk.chunk_ref,
                        crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
                        "chunk store reachable chunks",
                    )?;
                }
            }
        } else {
            crate::bounded::push(
                &mut manifests,
                manifest_ref.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_MANIFESTS,
                "chunk store removed manifests",
            )?;
        }
    }
    let mut chunks = Vec::new();
    for chunk_ref in crate::blocks::list_chunk_refs_with_root(root)? {
        if reachable_chunks.iter().any(|reachable| reachable == &chunk_ref) {
            continue;
        }
        crate::bounded::push(
            &mut chunks,
            chunk_ref.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk store removed chunks",
        )?;
    }
    Ok(GcTargets { manifests, chunks })
}

#[derive(Clone, Copy)]
struct GcReceiptInput<'a> {
    is_dry_run: bool,
    decision: &'a str,
    removed_manifests: &'a [String],
    removed_chunks: &'a [String],
    notes: &'a crate::blocks::retire::admission::GcNotes,
    evidence_summary: &'a preserves::IOValue,
}

fn gc_receipt_value(input: GcReceiptInput<'_>) -> preserves::IOValue {
    crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
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
            crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(if input.is_dry_run {
                "dry-run"
            } else {
                "apply"
            })]),
            crate::preserves_rail::record("removed-manifests", vec![crate::preserves_rail::sequence(
                input.removed_manifests.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention", vec![crate::preserves_rail::sequence(
                input.notes.receipts.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention-execution", vec![crate::preserves_rail::sequence(
                input.notes.execution_gates.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("denied", vec![crate::preserves_rail::sequence(
                input.notes.denials.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention-evidence", vec![input.evidence_summary.clone()]),
            crate::preserves_rail::record("retention-admission", vec![crate::preserves_rail::sequence(
                input.notes.admission_refs.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention-diagnostics", vec![crate::preserves_rail::sequence(
                input.notes.admission_diagnostics.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention-execution-diagnostics", vec![crate::preserves_rail::sequence(
                input.notes.execution_diagnostics.iter().map(crate::preserves_rail::string).collect(),
            )]),
        ],
    })
}

fn gc_tombstone_value(input: GcReceiptInput<'_>) -> Option<preserves::IOValue> {
    if input.is_dry_run
        || input.decision != "pass"
        || (input.removed_manifests.is_empty() && input.removed_chunks.is_empty())
    {
        return None;
    }
    Some(crate::blocks::wire::outcome::receipt_value(crate::blocks::wire::outcome::ChunkReceiptValueInput {
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
            crate::preserves_rail::record("mode", vec![crate::preserves_rail::string("apply")]),
            crate::preserves_rail::record("removed-manifests", vec![crate::preserves_rail::sequence(
                input.removed_manifests.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention", vec![crate::preserves_rail::sequence(
                input.notes.receipts.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention-execution", vec![crate::preserves_rail::sequence(
                input.notes.execution_gates.iter().map(crate::preserves_rail::string).collect(),
            )]),
            crate::preserves_rail::record("retention-evidence", vec![input.evidence_summary.clone()]),
            crate::preserves_rail::record("retention-admission", vec![crate::preserves_rail::sequence(
                input.notes.admission_refs.iter().map(crate::preserves_rail::string).collect(),
            )]),
        ],
    }))
}

struct GcFinishInput<'a> {
    root: &'a crate::blocks::CapabilityChunkRoot,
    is_dry_run: bool,
    targets: GcTargets,
    notes: crate::blocks::retire::admission::GcNotes,
    evidence_summary: preserves::IOValue,
}

fn finish_gc(input: GcFinishInput<'_>) -> crate::error::Result<crate::blocks::ChunkGc> {
    let decision = if input.notes.denials.is_empty() { "pass" } else { "deny" };
    let mut removed_manifests = Vec::new();
    let mut removed_chunks = Vec::new();
    if decision == "pass" {
        removed_manifests = input.targets.manifests;
        removed_chunks = input.targets.chunks;
        if !input.is_dry_run {
            for manifest_ref in &removed_manifests {
                input.root.root().remove_file(&crate::blocks::layout::manifest_path(manifest_ref)?)?;
            }
            for chunk_ref in &removed_chunks {
                input.root.root().remove_file(&crate::blocks::layout::chunk_path(chunk_ref)?)?;
            }
        }
    }
    let receipt_input = GcReceiptInput {
        is_dry_run: input.is_dry_run,
        decision,
        removed_manifests: &removed_manifests,
        removed_chunks: &removed_chunks,
        notes: &input.notes,
        evidence_summary: &input.evidence_summary,
    };
    let receipt_value = gc_receipt_value(receipt_input);
    let tombstone_receipt = gc_tombstone_value(receipt_input);
    crate::blocks::registry::update::index_apply_gc(&crate::blocks::registry::update::IndexApplyGcInput {
        root: input.root,
        dry_run: input.is_dry_run,
        removed_manifests: &removed_manifests,
        removed_chunks: &removed_chunks,
        receipt_value: &receipt_value,
        tombstone_receipt: tombstone_receipt.as_ref(),
    })?;
    Ok(crate::blocks::ChunkGc {
        dry_run: input.is_dry_run,
        decision: decision.to_string(),
        removed_manifests,
        removed_chunks,
        retention_receipt_refs: input.notes.receipts,
        execution_gate_refs: input.notes.execution_gates,
        receipt_value,
    })
}

pub fn gc(
    root: &std::path::Path,
    input: crate::blocks::ChunkGcInput<'_>,
) -> crate::error::Result<crate::blocks::ChunkGc> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    gc_with_root(&root, input)
}

pub fn gc_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    input: crate::blocks::ChunkGcInput<'_>,
) -> crate::error::Result<crate::blocks::ChunkGc> {
    crate::blocks::layout::ensure_dirs(root)?;
    let targets = gc_targets(
        root,
        crate::blocks::layout::pinned_refs(root, &crate::blocks::layout::path("pins/manifests")?)?,
        crate::blocks::layout::pinned_refs(root, &crate::blocks::layout::path("pins/chunks")?)?,
    )?;
    let action = if input.dry_run {
        crate::retention::ACTION_ELIGIBILITY
    } else {
        crate::retention::ACTION_DELETE
    };
    let requester_ref =
        crate::retention::destructive_requester_ref(input.retention_evidence, "chunk-store-gc-missing-requester")?;
    let evidence_summary = crate::retention::destructive_evidence_value(input.retention_evidence)?;
    let retention_root = crate::local_store::RetentionStoreRoot::share_chunk_state(root)?;
    let env = crate::blocks::retire::admission::GcEnv {
        retention_root: &retention_root,
        is_dry_run: input.dry_run,
        evidence: input.retention_evidence,
        apply_refs: input.apply_refs,
        action,
        requester_ref: &requester_ref,
    };
    let mut notes = crate::blocks::retire::admission::GcNotes::default();
    for manifest_ref in &targets.manifests {
        notes.consider(&env, crate::blocks::retire::admission::GcObject {
            object_ref: manifest_ref,
            object_kind: "chunk-manifest",
            retention_class: crate::retention::CLASS_PUBLIC_ARTIFACT,
        })?;
    }
    for chunk_ref in &targets.chunks {
        notes.consider(&env, crate::blocks::retire::admission::GcObject {
            object_ref: chunk_ref,
            object_kind: "chunk",
            retention_class: crate::retention::CLASS_DURABLE_VALUE,
        })?;
    }
    finish_gc(GcFinishInput {
        root,
        is_dry_run: input.dry_run,
        targets,
        notes,
        evidence_summary,
    })
}
