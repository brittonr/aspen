//! Post-effect uncertainty over a live bounded child process. The effect runs as
//! a distinct OS process (`/bin/sh`) through the production live execution
//! adapter; the native work boundary is the production capability-rooted
//! service host, whose durable work record must stay Unknown and must never be
//! requeued once that process effect may have happened.

mod bounded;
mod host;
#[cfg(test)]
mod tests;

/// The admitted effect executable. A real distinct OS process, never a shell
/// builtin inside this process.
const EFFECT_PROGRAM: &str = "/bin/sh";
const MISSING_PROGRAM: &str = "/nonexistent/aspen-qualification-effect";
/// Created by the child in its workspace before it exits without reading its
/// input: proof that a distinct process started and its effect may have happened.
const CHILD_STARTED_MARKER: &str = "effect-started";
const EFFECT_SCRIPT: &str = ": > effect-started; exit 0";
const GENERATION: u64 = 1;
const TIMEOUT_MS: u64 = 5_000;
const POLL_INTERVAL_MS: u64 = 5;
const TEARDOWN_TIMEOUT_MS: u64 = 1_000;
const STREAM_BYTES: u64 = 4_096;
/// Exceeds the Linux pipe capacity, so a child that exits without reading
/// leaves delivery of its input, and therefore its effect, unknown.
const UNCONSUMED_INPUT_BYTES: u64 = 262_144;
const UNITS: u64 = 1;
const DEADLINE_TICKS: u64 = 10_000;
const SERVICE_ID: &str = "qualification-effect-service";
const EXTENSION_ID: &str = "qualification-effect-extension";
const GROUP_ID: &str = "qualification-effect-group";
const WORK_PAYLOAD: &[u8] = b"qualification durable effect work";
const INSTALL_TICK: u64 = 1;
const SUBMIT_TICK: u64 = 2;
const CLAIM_TICK: u64 = 3;
const DELIVER_TICK: u64 = 4;
const RETRY_TICK: u64 = 5;
const REOPEN_TICK: u64 = 6;

pub(super) fn run(scenario: super::super::Scenario) -> crate::error::Result<super::Evidence> {
    match scenario {
        super::super::Scenario::ExecutionUnknownAfterStart => execution_unknown_after_start(),
        super::super::Scenario::ExecutionPreStartDefinite => execution_prestart_definite(),
        super::super::Scenario::NativeWorkUnknownAfterEffect => native_work_unknown_after_start(),
        super::super::Scenario::NativeUncertainWorkNotReadmitted => native_uncertain_work_not_readmitted(),
        super::super::Scenario::GrantedSend
        | super::super::Scenario::DeniedSend
        | super::super::Scenario::DurableRestartRecovery
        | super::super::Scenario::DurableCorruptSnapshotDenied
        | super::super::Scenario::TransportEndpointImport
        | super::super::Scenario::TransportEndpointTamperDenied
        | super::super::Scenario::TransportSessionAcknowledged
        | super::super::Scenario::TransportSessionLostAcknowledgement
        | super::super::Scenario::WorldReplayAdmitted
        | super::super::Scenario::WorldReplayAuthorityDenied
        | super::super::Scenario::WorldRestoreCurrentAdmission
        | super::super::Scenario::WorldRestoreStaleAdmissionDenied
        | super::super::Scenario::WorldActivationCurrentFacts
        | super::super::Scenario::WorldActivationRevokedDenied
        | super::super::Scenario::RaftFailoverCommitSurvives
        | super::super::Scenario::RaftStaleLeaderFenced
        | super::super::Scenario::RaftFinalQuorumEvidence
        | super::super::Scenario::RaftFalseQuorumDenied => {
            Err(super::super::invalid("scenario is not an effect-uncertainty scenario"))
        }
    }
}

// r[impl molten.fabric_execution.uncertainty]
fn execution_unknown_after_start() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut effect = bounded::LiveEffect::new()?;
    evidence.runtime(effect.profile_value());
    let failure = effect.execute(EFFECT_PROGRAM, &effect.resolved(EFFECT_PROGRAM))?;
    evidence.check("effect-child-process-started", effect.child_started());
    evidence.check(
        "post-start-failure-classified-unknown",
        failure.kind == crate::fabric_execution::PortFailureKind::UnknownAfterStart,
    );
    evidence.check("no-terminal-receipt-claimed", failure.receipt.is_none());
    evidence.check("no-process-completion-claimed", failure.process_observation.is_none());
    evidence.check("no-output-published", effect.published() == 0);
    evidence.check(
        "reconciliation-required",
        effect.reconcile() == crate::fabric_execution::ReconciliationStatus::UnknownRequiresReconciliation,
    );
    evidence.runtime(bounded::failure_value(&failure));
    Ok(evidence)
}

// r[impl molten.fabric_execution.uncertainty]
fn execution_prestart_definite() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let mut effect = bounded::LiveEffect::new()?;

    let mut crossed = effect.resolved(EFFECT_PROGRAM);
    crossed.executable_identity_ref = reference("qualification effect substituted executable measurement");
    let denied = effect.execute(EFFECT_PROGRAM, &crossed)?;
    evidence.check(
        "crossed-resolution-denied-before-start",
        denied.kind == crate::fabric_execution::PortFailureKind::ResolutionDenied,
    );
    evidence.check(
        "crossed-resolution-not-recorded",
        effect.reconcile() == crate::fabric_execution::ReconciliationStatus::NotFound,
    );
    evidence.check("crossed-resolution-started-no-child", !effect.child_started());
    evidence.runtime(bounded::failure_value(&denied));

    let missing = effect.execute(MISSING_PROGRAM, &effect.resolved(MISSING_PROGRAM))?;
    evidence.check(
        "spawn-failure-rejected-before-start",
        missing.kind == crate::fabric_execution::PortFailureKind::RejectedBeforeStart,
    );
    evidence.check(
        "spawn-failure-definite-not-unknown",
        effect.reconcile() == crate::fabric_execution::ReconciliationStatus::DefinitePreStartFailure,
    );
    evidence.check("definite-failure-has-no-receipt", missing.receipt.is_none());
    evidence.check("no-output-published", effect.published() == 0);
    evidence.check("spawn-failure-started-no-child", !effect.child_started());
    evidence.runtime(bounded::failure_value(&missing));
    Ok(evidence)
}

// r[impl molten.system_extension.native_effect_uncertainty]
fn native_work_unknown_after_start() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let scratch = super::scratch("native-effect")?;
    let root = crate::node_state::DirectoryRoot::open_existing(scratch.path())?;
    root.create_layout()?;
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0_u64));
    let mut host = host::open(&root, &attempts)?;
    let delivered = host::deliver_work(&mut host)?;
    evidence.check("effect-delivery-reported-uncertain", host::is_uncertain(&delivered));
    if let Err(error) = &delivered {
        evidence.runtime(super::error_class(error));
    }
    evidence.check("process-effect-child-started-once", attempts.get() == 1);
    let status = host.status(DELIVER_TICK)?;
    evidence.check("durable-work-unknown", host::work_state(&status)? == "unknown");
    evidence.runtime(status);
    drop(host);

    let mut reopened = host::open(&root, &attempts)?;
    let status = reopened.status(REOPEN_TICK)?;
    evidence.check("reopened-work-still-unknown", host::work_state(&status)? == "unknown");
    evidence.check("reopen-does-not-requeue-effect", attempts.get() == 1);
    evidence.runtime(status);
    Ok(evidence)
}

// r[impl molten.system_extension.native_effect_uncertainty]
fn native_uncertain_work_not_readmitted() -> crate::error::Result<super::Evidence> {
    let mut evidence = super::Evidence::default();
    let scratch = super::scratch("native-effect-readmit")?;
    let root = crate::node_state::DirectoryRoot::open_existing(scratch.path())?;
    root.create_layout()?;
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0_u64));
    let mut host = host::open(&root, &attempts)?;
    let delivered = host::deliver_work(&mut host)?;
    evidence.check("effect-delivery-reported-uncertain", host::is_uncertain(&delivered));
    evidence.check("process-effect-child-started-once", attempts.get() == 1);

    let readmit = host.readmit(SERVICE_ID, &host::work_ref(), RETRY_TICK);
    evidence.check("uncertain-work-readmission-denied", readmit.is_err());
    let retry = host.deliver(&host::delivery_input(Some(reference("qualification replayed delivery claim"))));
    evidence.check("uncertain-work-redelivery-denied", retry.is_err());
    evidence.check("uncertain-effect-not-repeated", attempts.get() == 1);
    drop(host);

    let mut reopened = host::open(&root, &attempts)?;
    evidence.check(
        "reopened-uncertain-work-readmission-denied",
        reopened.readmit(SERVICE_ID, &host::work_ref(), REOPEN_TICK).is_err(),
    );
    evidence.check("reopen-does-not-requeue-effect", attempts.get() == 1);
    let status = reopened.status(REOPEN_TICK)?;
    evidence.check("durable-work-unknown", host::work_state(&status)? == "unknown");
    evidence.runtime(status);
    Ok(evidence)
}

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}
