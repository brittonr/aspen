#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryDecision {
    Admit,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    StoreWrite,
    StoreDelete,
    TransportSend,
    ExecuteWorker,
    ClockRead,
    ReceiptWrite,
    RegistryRead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectPlan {
    pub decision: BoundaryDecision,
    pub effects: Vec<EffectKind>,
    pub diagnostics: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionInputs {
    pub has_authority: bool,
    pub evidence_fresh: bool,
    pub resource_allowed: bool,
    pub adapter_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreWriteInput {
    pub admission: AdmissionInputs,
    pub value_well_formed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionGcInput {
    pub admission: AdmissionInputs,
    pub remote_clearance_present: bool,
    pub index_complete: bool,
    pub plan_stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobExecutionInput {
    pub admission: AdmissionInputs,
    pub dag_acyclic: bool,
    pub manifest_present: bool,
    pub lease_fresh: bool,
    pub executor_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeEnqueueInput {
    pub admission: AdmissionInputs,
    pub duplicate_operation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarnessGateInput {
    pub suite_present: bool,
    pub report_well_formed: bool,
    pub schema_supported: bool,
    pub report_stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryDiscoveryInput {
    pub discovery_present: bool,
    pub authority_present: bool,
    pub provenance_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidencePolicyRuntimeInput {
    pub evidence_verified: bool,
    pub policy_admitted: bool,
    pub runtime_can_plan: bool,
    pub adapter_available: bool,
}

pub fn plan_adapter_effects(admission: AdmissionInputs, admitted_effects: &[EffectKind]) -> EffectPlan {
    let diagnostics = admission_diagnostics(admission);
    if !diagnostics.is_empty() {
        return deny(diagnostics);
    }
    admit(admitted_effects)
}

pub fn plan_store_write(input: StoreWriteInput) -> EffectPlan {
    if !input.value_well_formed {
        return deny(vec!["malformed store value"]);
    }
    plan_adapter_effects(input.admission, &[EffectKind::StoreWrite, EffectKind::ReceiptWrite])
}

pub fn plan_retention_gc(input: RetentionGcInput) -> EffectPlan {
    let mut diagnostics = admission_diagnostics(input.admission);
    if !input.remote_clearance_present {
        diagnostics.push("missing remote retention clearance");
    }
    if !input.index_complete {
        diagnostics.push("retention index incomplete");
    }
    if input.plan_stale {
        diagnostics.push("retention plan stale");
    }
    if !diagnostics.is_empty() {
        return deny(diagnostics);
    }
    admit(&[EffectKind::StoreDelete, EffectKind::ReceiptWrite])
}

pub fn plan_job_execution(input: JobExecutionInput) -> EffectPlan {
    let mut diagnostics = admission_diagnostics(input.admission);
    if !input.dag_acyclic {
        diagnostics.push("job dag contains cycle");
    }
    if !input.manifest_present {
        diagnostics.push("missing job manifest");
    }
    if !input.lease_fresh {
        diagnostics.push("stale lease token");
    }
    if !input.executor_supported {
        diagnostics.push("unsupported executor");
    }
    if !diagnostics.is_empty() {
        return deny(diagnostics);
    }
    admit(&[EffectKind::ExecuteWorker, EffectKind::ReceiptWrite])
}

pub fn plan_node_enqueue(input: NodeEnqueueInput) -> EffectPlan {
    let diagnostics = admission_diagnostics(input.admission);
    if !diagnostics.is_empty() {
        return deny(diagnostics);
    }
    if input.duplicate_operation {
        return EffectPlan {
            decision: BoundaryDecision::Admit,
            effects: vec![EffectKind::ReceiptWrite],
            diagnostics: vec!["duplicate operation replayed without enqueue mutation"],
        };
    }
    admit(&[EffectKind::StoreWrite, EffectKind::ReceiptWrite])
}

pub fn plan_harness_gate(input: HarnessGateInput) -> EffectPlan {
    let mut diagnostics = Vec::new();
    if !input.suite_present {
        diagnostics.push("missing harness suite");
    }
    if !input.report_well_formed {
        diagnostics.push("malformed harness report");
    }
    if !input.schema_supported {
        diagnostics.push("unsupported harness schema");
    }
    if input.report_stale {
        diagnostics.push("stale harness report");
    }
    if !diagnostics.is_empty() {
        return deny(diagnostics);
    }
    admit(&[EffectKind::ReceiptWrite])
}

pub fn plan_registry_discovery(input: RegistryDiscoveryInput) -> EffectPlan {
    if !input.discovery_present {
        return deny(vec!["missing registry discovery input"]);
    }
    if input.authority_present && input.provenance_present {
        return admit(&[EffectKind::RegistryRead, EffectKind::ReceiptWrite]);
    }
    EffectPlan {
        decision: BoundaryDecision::Deny,
        effects: vec![EffectKind::RegistryRead, EffectKind::ReceiptWrite],
        diagnostics: vec!["registry discovery is evidence-only and cannot grant trust alone"],
    }
}

pub fn plan_evidence_policy_runtime_flow(input: EvidencePolicyRuntimeInput) -> EffectPlan {
    let mut diagnostics = Vec::new();
    if !input.evidence_verified {
        diagnostics.push("evidence not verified");
    }
    if !input.policy_admitted {
        diagnostics.push("policy admission denied");
    }
    if !input.runtime_can_plan {
        diagnostics.push("runtime planning unavailable");
    }
    if !diagnostics.is_empty() {
        return deny(diagnostics);
    }
    let mut plan = admit(&[EffectKind::ReceiptWrite]);
    if !input.adapter_available {
        plan.diagnostics.push("adapter unavailable; receipt-only plan retained");
    }
    plan
}

fn admission_diagnostics(admission: AdmissionInputs) -> Vec<&'static str> {
    let mut diagnostics = Vec::new();
    if !admission.has_authority {
        diagnostics.push("missing authority");
    }
    if !admission.evidence_fresh {
        diagnostics.push("stale evidence");
    }
    if !admission.resource_allowed {
        diagnostics.push("resource denied");
    }
    if !admission.adapter_supported {
        diagnostics.push("unsupported adapter capability");
    }
    diagnostics
}

fn admit(effects: &[EffectKind]) -> EffectPlan {
    EffectPlan {
        decision: BoundaryDecision::Admit,
        effects: effects.to_vec(),
        diagnostics: Vec::new(),
    }
}

fn deny(diagnostics: Vec<&'static str>) -> EffectPlan {
    EffectPlan {
        decision: BoundaryDecision::Deny,
        effects: Vec::new(),
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admitted() -> AdmissionInputs {
        AdmissionInputs {
            has_authority: true,
            evidence_fresh: true,
            resource_allowed: true,
            adapter_supported: true,
        }
    }

    fn missing_authority() -> AdmissionInputs {
        AdmissionInputs {
            has_authority: false,
            evidence_fresh: true,
            resource_allowed: true,
            adapter_supported: true,
        }
    }

    #[test]
    fn adapter_plan_emits_effects_only_after_admission() {
        let plan = plan_adapter_effects(admitted(), &[EffectKind::TransportSend, EffectKind::ReceiptWrite]);

        assert_eq!(plan.decision, BoundaryDecision::Admit);
        assert_eq!(plan.effects, vec![EffectKind::TransportSend, EffectKind::ReceiptWrite]);
        assert!(plan.diagnostics.is_empty());
    }

    #[test]
    fn adapter_plan_denies_without_effects_when_authority_is_missing() {
        let plan = plan_adapter_effects(missing_authority(), &[EffectKind::TransportSend]);

        assert_eq!(plan.decision, BoundaryDecision::Deny);
        assert!(plan.effects.is_empty());
        assert_eq!(plan.diagnostics, vec!["missing authority"]);
    }

    #[test]
    fn store_write_denies_malformed_value_before_redb_write_plan() {
        let plan = plan_store_write(StoreWriteInput {
            admission: admitted(),
            value_well_formed: false,
        });

        assert_eq!(plan.decision, BoundaryDecision::Deny);
        assert!(plan.effects.is_empty());
        assert_eq!(plan.diagnostics, vec!["malformed store value"]);
    }

    #[test]
    fn retention_gc_requires_clearance_and_complete_index() {
        let positive = plan_retention_gc(RetentionGcInput {
            admission: admitted(),
            remote_clearance_present: true,
            index_complete: true,
            plan_stale: false,
        });
        assert_eq!(positive.effects, vec![EffectKind::StoreDelete, EffectKind::ReceiptWrite]);

        let negative = plan_retention_gc(RetentionGcInput {
            admission: admitted(),
            remote_clearance_present: false,
            index_complete: false,
            plan_stale: true,
        });
        assert_eq!(negative.decision, BoundaryDecision::Deny);
        assert!(negative.effects.is_empty());
        assert!(negative.diagnostics.contains(&"missing remote retention clearance"));
        assert!(negative.diagnostics.contains(&"retention index incomplete"));
        assert!(negative.diagnostics.contains(&"retention plan stale"));
    }

    #[test]
    fn job_execution_denies_cycles_stale_leases_missing_manifests_and_unsupported_executors() {
        let plan = plan_job_execution(JobExecutionInput {
            admission: admitted(),
            dag_acyclic: false,
            manifest_present: false,
            lease_fresh: false,
            executor_supported: false,
        });

        assert_eq!(plan.decision, BoundaryDecision::Deny);
        assert!(plan.effects.is_empty());
        assert!(plan.diagnostics.contains(&"job dag contains cycle"));
        assert!(plan.diagnostics.contains(&"missing job manifest"));
        assert!(plan.diagnostics.contains(&"stale lease token"));
        assert!(plan.diagnostics.contains(&"unsupported executor"));
    }

    #[test]
    fn node_duplicate_enqueue_records_receipt_without_queue_mutation() {
        let plan = plan_node_enqueue(NodeEnqueueInput {
            admission: admitted(),
            duplicate_operation: true,
        });

        assert_eq!(plan.decision, BoundaryDecision::Admit);
        assert_eq!(plan.effects, vec![EffectKind::ReceiptWrite]);
        assert_eq!(plan.diagnostics, vec!["duplicate operation replayed without enqueue mutation"]);
    }

    #[test]
    fn harness_gate_accepts_supported_report_and_rejects_stale_malformed_inputs() {
        let positive = plan_harness_gate(HarnessGateInput {
            suite_present: true,
            report_well_formed: true,
            schema_supported: true,
            report_stale: false,
        });
        assert_eq!(positive.decision, BoundaryDecision::Admit);

        let negative = plan_harness_gate(HarnessGateInput {
            suite_present: false,
            report_well_formed: false,
            schema_supported: false,
            report_stale: true,
        });
        assert_eq!(negative.decision, BoundaryDecision::Deny);
        assert!(negative.diagnostics.contains(&"missing harness suite"));
        assert!(negative.diagnostics.contains(&"malformed harness report"));
        assert!(negative.diagnostics.contains(&"unsupported harness schema"));
        assert!(negative.diagnostics.contains(&"stale harness report"));
    }

    #[test]
    fn registry_discovery_remains_non_authoritative_without_authority_and_provenance() {
        let plan = plan_registry_discovery(RegistryDiscoveryInput {
            discovery_present: true,
            authority_present: false,
            provenance_present: false,
        });

        assert_eq!(plan.decision, BoundaryDecision::Deny);
        assert_eq!(plan.effects, vec![EffectKind::RegistryRead, EffectKind::ReceiptWrite]);
        assert!(plan.diagnostics[0].contains("evidence-only"));
    }

    #[test]
    fn evidence_policy_runtime_flow_does_not_treat_adapter_availability_as_trust() {
        let plan = plan_evidence_policy_runtime_flow(EvidencePolicyRuntimeInput {
            evidence_verified: false,
            policy_admitted: false,
            runtime_can_plan: true,
            adapter_available: true,
        });

        assert_eq!(plan.decision, BoundaryDecision::Deny);
        assert!(plan.effects.is_empty());
        assert!(plan.diagnostics.contains(&"evidence not verified"));
        assert!(plan.diagnostics.contains(&"policy admission denied"));
    }
}
