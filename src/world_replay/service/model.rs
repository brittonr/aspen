use molten_core::world_replay::*;

use super::super::*;

pub struct WorldReplayPorts<'a> {
    pub materialization: &'a mut dyn WorldReplayMaterializationPort,
    pub restore: &'a mut dyn WorldReplayRestorePort,
    pub admission: &'a mut dyn WorldReplayAdmissionPort,
    pub transitions: &'a mut dyn WorldReplayTransitionPort,
    pub capture: &'a mut dyn WorldReplayCapturePort,
    pub receipts: &'a mut dyn WorldReplayReceiptPort,
}

#[derive(Debug, Clone)]
pub struct WorldReplayRunOutcome {
    pub plan: WorldReplayPlan,
    pub trace_record: CanonicalWorldReplayRecord,
    pub capsule_record: CanonicalWorldReplayRecord,
    pub plan_record: CanonicalWorldReplayRecord,
    pub restore: WorldReplayRestoreObservation,
    pub admission: WorldReplayAdmissionObservation,
    pub executions: Vec<WorldReplayExecutionObservation>,
    pub captures: Vec<WorldReplayCaptureObservation>,
    pub divergence_record: Option<CanonicalWorldReplayRecord>,
    pub receipt: WorldReplayReceipt,
    pub receipt_record: CanonicalWorldReplayRecord,
}

pub struct WorldReplayImportPorts<'a> {
    pub validation: &'a mut dyn WorldReplayImportValidationPort,
    pub publication: &'a mut dyn WorldReplayImportPublicationPort,
    pub receipts: &'a mut dyn WorldReplayReceiptPort,
}

#[derive(Debug, Clone)]
pub struct WorldReplayImportOutcome {
    pub verifications: Vec<WorldReplayImportVerification>,
    pub staged_refs: Vec<String>,
    pub receipt: WorldReplayImportReceipt,
    pub receipt_record: CanonicalWorldReplayRecord,
}

#[derive(Debug, Clone)]
pub struct WorldReplayExportOutcome {
    pub capsule_record: CanonicalWorldReplayRecord,
    pub observations: Vec<WorldReplayExchangeObservation>,
}
