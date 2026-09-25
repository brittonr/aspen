// Root-only retention-admitted cache invalidation.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InvalidateInput {
    pub key_ref: Option<String>,
    pub dependency_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub capability_ref: Option<String>,
    pub revocation_ref: Option<String>,
    pub operation: Option<String>,
    pub reason: String,
    pub retention_evidence: crate::retention::DestructiveEvidence,
    pub apply_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalidation {
    pub decision: String,
    pub invalidated_key_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub execution_gate_refs: Vec<String>,
    pub receipt_value: IoValue,
}

pub fn invalidate(root: &Path, input: &InvalidateInput) -> Result<Invalidation> {
    ensure_dirs(root)?;
    validate_invalidate_input(input)?;
    let selected_key_refs = selected_keys(root, input)?;
    let reason = invalidation_reason(input);
    let requester_ref = crate::retention::destructive_requester_ref(
        &input.retention_evidence,
        "eval-cache-invalidate-missing-requester",
    )?;
    let run = run_retention(root, input, &requester_ref, &selected_key_refs)?;
    let decision = run.decision();
    let invalidated_key_refs = if decision == "pass" {
        selected_key_refs
    } else {
        Vec::new()
    };
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    if decision == "pass" {
        let mut tombstones = write_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
        for key_ref in &invalidated_key_refs {
            tombstones.insert(key_ref.as_str(), reason.as_str()).map_err(index_error)?;
        }
    }
    let refs = invalidate_refs(input, &invalidated_key_refs, &run)?;
    let diagnostics = invalidate_diagnostics(input, decision, &invalidated_key_refs, &run);
    let receipt = invalidate_receipt(decision, &refs, &diagnostics, &run)?;
    store_receipt_in_tx(&write_txn, &receipt)?;
    write_txn.commit().map_err(index_error)?;
    Ok(Invalidation {
        decision: decision.to_string(),
        invalidated_key_refs,
        retention_receipt_refs: run.receipts,
        execution_gate_refs: run.gates,
        receipt_value: receipt,
    })
}

fn selected_keys(root: &Path, input: &InvalidateInput) -> Result<Vec<String>> {
    let mut keys = BtreeSet::new();
    if let Some(key_ref) = input.key_ref.as_ref() {
        keys.insert(key_ref.clone());
    }
    for summary in list(root, &ListFilter::default())? {
        if input.operation.as_ref().is_some_and(|operation| operation == &summary.operation) {
            keys.insert(summary.key_ref.clone());
        }
        if has_ref_filter(input) && summary_refs_match(root, input, &summary.key_ref)? {
            keys.insert(summary.key_ref.clone());
        }
    }
    Ok(keys.into_iter().collect())
}

fn has_ref_filter(input: &InvalidateInput) -> bool {
    input.dependency_ref.is_some()
        || input.policy_ref.is_some()
        || input.capability_ref.is_some()
        || input.revocation_ref.is_some()
}

fn summary_refs_match(root: &Path, input: &InvalidateInput, key_ref: &str) -> Result<bool> {
    let Some((key, value)) = read_key_value_pair(root, key_ref)? else {
        return Ok(false);
    };
    Ok(input
        .dependency_ref
        .as_ref()
        .is_some_and(|reference| key.dependency_refs.contains(reference) || value.dependency_refs.contains(reference))
        || input
            .policy_ref
            .as_ref()
            .is_some_and(|reference| key.policy_refs.contains(reference) || value.policy_refs.contains(reference))
        || input.capability_ref.as_ref().is_some_and(|reference| key.capability_refs.contains(reference))
        || input.revocation_ref.as_ref().is_some_and(|reference| key.revocation_refs.contains(reference)))
}

fn invalidation_reason(input: &InvalidateInput) -> String {
    if input.reason.is_empty() {
        "manual-invalidate".to_string()
    } else {
        input.reason.clone()
    }
}

#[derive(Default)]
struct InvalRun {
    admission_diagnostics: Vec<String>,
    execution_diagnostics: Vec<String>,
    admission_refs: Vec<String>,
    receipts: Vec<String>,
    gates: Vec<String>,
    denials: Vec<String>,
}

struct InvalStep {
    key_ref: String,
    admission_diagnostics: Vec<String>,
    execution_diagnostics: Vec<String>,
    admission_refs: Vec<String>,
    receipt_ref: String,
    gate_ref: String,
    denied: bool,
}

impl InvalRun {
    fn add(&mut self, step: InvalStep) -> Result<()> {
        for diagnostic in step.admission_diagnostics {
            push_bounded(
                &mut self.admission_diagnostics,
                diagnostic,
                MAX_EVAL_CACHE_SCAN_ENTRIES,
                "eval cache retention admission diagnostics",
            )?;
        }
        for diagnostic in step.execution_diagnostics {
            push_bounded(
                &mut self.execution_diagnostics,
                diagnostic,
                MAX_EVAL_CACHE_SCAN_ENTRIES,
                "eval cache retention execution diagnostics",
            )?;
        }
        for reference in step.admission_refs {
            push_bounded(
                &mut self.admission_refs,
                reference,
                MAX_EVAL_CACHE_SCAN_ENTRIES,
                "eval cache retention admission refs",
            )?;
        }
        push_bounded(
            &mut self.receipts,
            step.receipt_ref,
            MAX_EVAL_CACHE_SCAN_ENTRIES,
            "eval cache retention receipt refs",
        )?;
        push_bounded(
            &mut self.gates,
            step.gate_ref,
            MAX_EVAL_CACHE_SCAN_ENTRIES,
            "eval cache retention execution gate refs",
        )?;
        if step.denied {
            push_bounded(&mut self.denials, step.key_ref, MAX_EVAL_CACHE_SCAN_ENTRIES, "eval cache retention denials")?;
        }
        Ok(())
    }

    fn decision(&self) -> &'static str {
        if self.denials.is_empty() { "pass" } else { "deny" }
    }

    fn has_admission_denial(&self) -> bool {
        !self.admission_diagnostics.is_empty()
    }

    fn has_execution_denial(&self) -> bool {
        !self.execution_diagnostics.is_empty()
    }
}

fn run_retention(
    root: &Path,
    input: &InvalidateInput,
    requester_ref: &str,
    selected_key_refs: &[String],
) -> Result<InvalRun> {
    let mut run = InvalRun::default();
    for key_ref in selected_key_refs {
        run.add(evaluate_invalidate_key(root, input, requester_ref, key_ref)?)?;
    }
    Ok(run)
}

fn evaluate_invalidate_key(
    root: &Path,
    input: &InvalidateInput,
    requester_ref: &str,
    key_ref: &str,
) -> Result<InvalStep> {
    let admission = crate::retention::admit_destructive_evidence(crate::retention::DestructiveAdmissionInput {
        root,
        evidence: &input.retention_evidence,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: crate::retention::CLASS_EPHEMERAL_CACHE,
        action: crate::retention::ACTION_TOMBSTONE,
    })?;
    let evaluation = crate::retention::evaluate(crate::retention::EvaluationInput {
        root,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: crate::retention::CLASS_EPHEMERAL_CACHE,
        action: crate::retention::ACTION_TOMBSTONE,
        requester_ref,
        is_reference_index_complete: input.retention_evidence.is_reference_index_complete,
        retained_refs: &input.retention_evidence.retained_refs,
        remote_refs: &input.retention_evidence.remote_refs,
        policy_refs: &input.retention_evidence.policy_refs,
        evidence_refs: &input.retention_evidence.evidence_refs,
        has_delete_authority: admission.has_delete_authority,
        has_remote_gc_clearance: admission.has_remote_gc_clearance,
    })?;
    let apply_ref = matching_apply_ref(ApplyRefMatchInput {
        root,
        apply_refs: &input.apply_refs,
        subsystem: "eval-cache-invalidate",
        action: crate::retention::ACTION_TOMBSTONE,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: crate::retention::CLASS_EPHEMERAL_CACHE,
    });
    let gate = crate::retention::store_gc_execution_gate(crate::retention::GcExecutionGateInput {
        root,
        subsystem: "eval-cache-invalidate",
        action: crate::retention::ACTION_TOMBSTONE,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: crate::retention::CLASS_EPHEMERAL_CACHE,
        apply_ref,
    })?;
    let is_gate_denied = gate.decision != "pass";
    let is_denied = admission.decision != "pass" || evaluation.receipt.decision != "pass" || is_gate_denied;
    Ok(InvalStep {
        key_ref: key_ref.to_string(),
        admission_diagnostics: admission.diagnostics,
        execution_diagnostics: if is_gate_denied { gate.diagnostics } else { Vec::new() },
        admission_refs: admission.admitted_refs,
        receipt_ref: evaluation.receipt.receipt_ref,
        gate_ref: gate.execution_ref,
        denied: is_denied,
    })
}

struct RefSink {
    refs: Vec<String>,
}

impl RefSink {
    fn new(seed_refs: &[String]) -> Self {
        Self {
            refs: seed_refs.to_vec(),
        }
    }

    fn push(&mut self, reference: &str) -> Result<()> {
        push_bounded(&mut self.refs, reference.to_string(), MAX_EVAL_CACHE_SCAN_ENTRIES, "eval cache receipt refs")
    }

    fn push_all(&mut self, references: &[String]) -> Result<()> {
        for reference in references {
            self.push(reference)?;
        }
        Ok(())
    }

    fn finish(self) -> Vec<String> {
        self.refs
    }
}

fn invalidate_refs(input: &InvalidateInput, invalidated_key_refs: &[String], run: &InvalRun) -> Result<Vec<String>> {
    let mut sink = RefSink::new(invalidated_key_refs);
    if let Some(requester_ref) = input.retention_evidence.requester_ref.as_ref() {
        sink.push(requester_ref)?;
    }
    sink.push_all(&input.retention_evidence.policy_refs)?;
    sink.push_all(&input.retention_evidence.authority_refs)?;
    sink.push_all(&input.retention_evidence.evidence_refs)?;
    sink.push_all(&input.retention_evidence.retained_refs)?;
    sink.push_all(&input.retention_evidence.remote_peer_refs)?;
    sink.push_all(&input.retention_evidence.remote_refs)?;
    sink.push_all(&input.retention_evidence.reference_index_refs)?;
    sink.push_all(&input.retention_evidence.remote_gc_refs)?;
    sink.push_all(&input.retention_evidence.remote_clearance_refs)?;
    sink.push_all(&run.admission_refs)?;
    sink.push_all(&run.receipts)?;
    sink.push_all(&run.gates)?;
    Ok(sink.finish())
}

fn invalidate_diagnostics(
    input: &InvalidateInput,
    decision: &str,
    invalidated_key_refs: &[String],
    run: &InvalRun,
) -> Vec<String> {
    let mut diagnostics = if decision == "pass" {
        vec![format!("invalidated {} keys", invalidated_key_refs.len())]
    } else {
        vec![format!("retention denied {} keys", run.denials.len())]
    };
    diagnostics.push(format!(
        "retention evidence requester={} policy={} authority={} evidence={} retained={} remote_peers={} remote={} reference_index={} remote_gc={} remote_clearance={} index_complete={}",
        input.retention_evidence.requester_ref.is_some(),
        input.retention_evidence.policy_refs.len(),
        input.retention_evidence.authority_refs.len(),
        input.retention_evidence.evidence_refs.len(),
        input.retention_evidence.retained_refs.len(),
        input.retention_evidence.remote_peer_refs.len(),
        input.retention_evidence.remote_refs.len(),
        input.retention_evidence.reference_index_refs.len(),
        input.retention_evidence.remote_gc_refs.len(),
        input.retention_evidence.remote_clearance_refs.len(),
        input.retention_evidence.is_reference_index_complete
    ));
    diagnostics.extend(run.admission_diagnostics.iter().cloned());
    diagnostics.extend(run.execution_diagnostics.iter().cloned());
    diagnostics
}

fn invalidate_receipt(decision: &str, refs: &[String], diagnostics: &[String], run: &InvalRun) -> Result<IoValue> {
    receipt_value(&ReceiptValueInput {
        operation: "invalidate",
        decision,
        key_ref: None,
        value_ref: None,
        refs,
        diagnostics,
        checks: &[
            ("cache-invalidation", if decision == "pass" { "pass" } else { "fail" }),
            ("tombstone", if decision == "pass" { "pass" } else { "fail" }),
            ("retention-receipt-bound", "pass"),
            ("retention-execution-gate", if run.has_execution_denial() { "fail" } else { "pass" }),
            ("retention-authority-evidence", if run.has_admission_denial() { "fail" } else { "pass" }),
            ("deny-before-tombstone", if decision == "pass" { "pass" } else { "fail" }),
        ],
    })
}

fn validate_invalidate_input(input: &InvalidateInput) -> Result<()> {
    if let Some(key_ref) = input.key_ref.as_ref() {
        validate_ref(key_ref, "invalidate key ref")?;
    }
    if let Some(dependency_ref) = input.dependency_ref.as_ref() {
        validate_ref(dependency_ref, "invalidate dependency ref")?;
    }
    if let Some(policy_ref) = input.policy_ref.as_ref() {
        validate_ref(policy_ref, "invalidate policy ref")?;
    }
    if let Some(capability_ref) = input.capability_ref.as_ref() {
        validate_ref(capability_ref, "invalidate capability ref")?;
    }
    if let Some(revocation_ref) = input.revocation_ref.as_ref() {
        validate_ref(revocation_ref, "invalidate revocation ref")?;
    }
    if let Some(operation) = input.operation.as_ref() {
        validate_operation(operation)?;
    }
    validate_refs(&input.apply_refs, "invalidate apply ref")?;
    crate::retention::validate_destructive_evidence(&input.retention_evidence)?;
    Ok(())
}

struct ApplyRefMatchInput<'a> {
    root: &'a Path,
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
        let Ok(apply) = crate::retention::read_gc_apply(input.root, apply_ref) else {
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
