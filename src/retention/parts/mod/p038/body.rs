
pub fn explain_candidate(input: CandidateExplainInput<'_>) -> Result<CandidateExplain> {
    let root = open_capability_retention_root(input.root)?;
    explain_candidate_with_root(CandidateExplainInput {
        root: &root,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
    })
}

pub fn explain_candidate_with_root(
    input: CandidateExplainInput<'_, CapabilityRetentionRoot>,
) -> Result<CandidateExplain> {
    validate_candidate_explain_input(&input)?;
    let filter = CandidateFilter {
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        subsystem: input.subsystem,
    };
    let refs = MatchRefs::collect(input.root, &filter)?;
    let diagnostics = candidate_explain_diagnostics(&refs.value_input(&input, &[]))?;
    let value = candidate_explain_value(&refs.value_input(&input, &diagnostics))?;
    parse_candidate_explain(&value)
}

impl MatchRefs {
    fn collect(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Self> {
        let pin_refs = pins_for(root, filter)?;
        let admission_refs = admissions_for(root, filter)?;
        let remote_clearance_refs = clearances_for(root, filter)?;
        let remote_clearance_import_refs = imports_for(root, &remote_clearance_refs)?;
        let gc_plan_refs = plans_for(root, filter)?;
        let gc_apply_refs = applies_for(root, filter)?;
        let gc_execution_refs = executions_for(root, filter)?;
        let gc_audit_refs = audits_for(root, filter)?;
        let retention_receipt_refs = receipts_for(root, filter)?;
        let tombstone_refs = tombstones_for(root, filter)?;
        Ok(Self {
            pin_refs,
            admission_refs,
            remote_clearance_refs,
            remote_clearance_import_refs,
            gc_plan_refs,
            gc_apply_refs,
            gc_execution_refs,
            gc_audit_refs,
            retention_receipt_refs,
            tombstone_refs,
        })
    }

    fn value_input<'a, Root: ?Sized>(
        &'a self,
        input: &CandidateExplainInput<'a, Root>,
        diagnostics: &'a [String],
    ) -> CandidateExplainValueInput<'a> {
        CandidateExplainValueInput {
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            subsystem: input.subsystem,
            pin_refs: &self.pin_refs,
            admission_refs: &self.admission_refs,
            remote_clearance_refs: &self.remote_clearance_refs,
            remote_clearance_import_refs: &self.remote_clearance_import_refs,
            gc_plan_refs: &self.gc_plan_refs,
            gc_apply_refs: &self.gc_apply_refs,
            gc_execution_refs: &self.gc_execution_refs,
            gc_audit_refs: &self.gc_audit_refs,
            retention_receipt_refs: &self.retention_receipt_refs,
            tombstone_refs: &self.tombstone_refs,
            diagnostics,
        }
    }
}

fn pins_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(root, RefListing { directory: PIN_DIR, label: "retention candidate pins" }, parse_pin, |pin| filter.matches_object(&pin.object_ref, &pin.object_kind, &pin.retention_class), |pin| pin.pin_ref.clone())
}

fn admissions_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(root, RefListing { directory: ADMISSION_DIR, label: "retention candidate admissions" }, parse_evidence_admission, |admission| {
            filter.matches_retention(
                &admission.object_ref,
                &admission.object_kind,
                &admission.retention_class,
                &admission.action,
            )
        }, |admission| admission.admission_ref.clone())
}

fn clearances_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(root, RefListing { directory: REMOTE_CLEARANCE_DIR, label: "retention candidate remote clearances" }, parse_remote_gc_clearance, |clearance| {
            filter.matches_retention(
                &clearance.object_ref,
                &clearance.object_kind,
                &clearance.retention_class,
                &clearance.action,
            )
        }, |clearance| clearance.clearance_ref.clone())
}

fn imports_for(root: &CapabilityRetentionRoot, remote_clearance_refs: &[String]) -> Result<Vec<String>> {
    collect_matching_refs(root, RefListing { directory: REMOTE_CLEARANCE_IMPORT_DIR, label: "retention candidate remote clearance imports" }, parse_remote_gc_clearance_import, |import| import.clearance_ref.as_ref().is_some_and(|reference| remote_clearance_refs.contains(reference)), |import| import.import_ref.clone())
}

fn plans_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(root, RefListing { directory: GC_PLAN_DIR, label: "retention candidate GC plans" }, parse_gc_plan, |plan| {
            filter.matches_gc(&plan.subsystem, &plan.object_ref, &plan.object_kind, &plan.retention_class, &plan.action)
        }, |plan| plan.plan_ref.clone())
}

fn applies_for(root: &CapabilityRetentionRoot, filter: &CandidateFilter<'_>) -> Result<Vec<String>> {
    collect_matching_refs(root, RefListing { directory: GC_APPLY_DIR, label: "retention candidate GC applies" }, parse_gc_apply, |apply| {
            filter.matches_gc(
                &apply.subsystem,
                &apply.object_ref,
                &apply.object_kind,
                &apply.retention_class,
                &apply.action,
            )
        }, |apply| apply.apply_ref.clone())
}
