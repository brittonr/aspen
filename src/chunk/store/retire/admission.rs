//! Per-target GC admission: retention evidence, evaluation, and (outside dry runs) the execution
//! gate.

struct ApplyRefMatchInput<'a> {
    root: &'a crate::local_store::RetentionStoreRoot,
    apply_refs: &'a [String],
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
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

pub(super) struct GcEnv<'a> {
    pub(super) retention_root: &'a crate::local_store::RetentionStoreRoot,
    pub(super) is_dry_run: bool,
    pub(super) evidence: &'a crate::retention::DestructiveEvidence,
    pub(super) apply_refs: &'a [String],
    pub(super) action: &'a str,
    pub(super) requester_ref: &'a str,
}

#[derive(Clone, Copy)]
pub(super) struct GcObject<'a> {
    pub(super) object_ref: &'a str,
    pub(super) object_kind: &'a str,
    pub(super) retention_class: &'a str,
}

#[derive(Default)]
pub(super) struct GcNotes {
    pub(super) admission_diagnostics: Vec<String>,
    pub(super) execution_diagnostics: Vec<String>,
    pub(super) admission_refs: Vec<String>,
    pub(super) receipts: Vec<String>,
    pub(super) execution_gates: Vec<String>,
    pub(super) denials: Vec<String>,
}

impl GcNotes {
    pub(super) fn consider(&mut self, env: &GcEnv<'_>, object: GcObject<'_>) -> crate::error::Result<()> {
        let admission =
            crate::retention::admit_destructive_evidence_with_root(crate::retention::DestructiveAdmissionInput {
                root: env.retention_root,
                evidence: env.evidence,
                object_ref: object.object_ref,
                object_kind: object.object_kind,
                retention_class: object.retention_class,
                action: env.action,
            })?;
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
        crate::bounded::push(
            &mut self.receipts,
            evaluation.receipt.receipt_ref.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk store retention receipt refs",
        )?;
        let is_execution_denied = if env.is_dry_run {
            false
        } else {
            self.note_execution(env, object)?
        };
        if admission.decision != "pass" || evaluation.receipt.decision != "pass" || is_execution_denied {
            crate::bounded::push(
                &mut self.denials,
                object.object_ref.to_string(),
                crate::blocks::limits::MAX_CHUNK_STORE_REFS,
                "chunk store retention denials",
            )?;
        }
        Ok(())
    }

    fn note_admission(&mut self, admission: &crate::retention::DestructiveAdmission) -> crate::error::Result<()> {
        for diagnostic in &admission.diagnostics {
            crate::bounded::push(
                &mut self.admission_diagnostics,
                diagnostic.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention admission diagnostics",
            )?;
        }
        for reference in &admission.admitted_refs {
            crate::bounded::push(
                &mut self.admission_refs,
                reference.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention admission refs",
            )?;
        }
        Ok(())
    }

    fn note_execution(&mut self, env: &GcEnv<'_>, object: GcObject<'_>) -> crate::error::Result<bool> {
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
        crate::bounded::push(
            &mut self.execution_gates,
            execution_gate.execution_ref.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk store retention execution gate refs",
        )?;
        if execution_gate.decision == "pass" {
            return Ok(false);
        }
        for diagnostic in &execution_gate.diagnostics {
            crate::bounded::push(
                &mut self.execution_diagnostics,
                diagnostic.clone(),
                crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
                "chunk store retention execution diagnostics",
            )?;
        }
        Ok(true)
    }
}
