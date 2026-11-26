//! World replay through the production replay service. A recorded runtime
//! world (source node) is captured commit by commit; replay restores its initial
//! commit into a destination node through the runtime-spine snapshot restore,
//! re-executes the same committed turns, captures each successor through the
//! world-commit capture shell, and lets `run_world_replay` compare and receipt.

mod admission;
mod closure;
mod history;
mod ports;

use crate::world_commit::WorldCommitPublicationPort;

const AUTHORITY_GENERATION: u64 = 1;
const SNAPSHOT_REVOCATION: &str = "aspen qualification replay snapshot revocation epoch 1";

/// Everything one replay run observed.
struct Run {
    request: molten_core::world_replay::WorldReplayPlanRequest,
    outcome: crate::world_replay::WorldReplayRunOutcome,
    materialized: usize,
    restores: usize,
    executed: Vec<u64>,
    captured: usize,
    receipt_kinds: Vec<&'static str>,
    authority_receipts: Vec<preserves::IOValue>,
}

// r[impl molten.world_replay.execution_boundary]
pub(super) fn admitted() -> crate::error::Result<super::super::Evidence> {
    let run = execute(false)?;
    let mut evidence = super::super::Evidence::default();
    let outcome = &run.outcome;
    let steps = run.request.trace.steps.iter().map(|step| step.position).collect::<Vec<_>>();
    evidence.check("capsule-members-materialized-and-verified", run.materialized == run.request.capsule.members.len());
    evidence.check("runtime-spine-snapshot-restored-once", run.restores == 1);
    evidence.check("current-admission-rerun-and-admitted", outcome.admission.admitted());
    evidence.check("every-transition-executed-in-order", run.executed == steps);
    evidence.check("every-successor-captured", run.captured == steps.len() && outcome.captures.len() == steps.len());
    evidence.check(
        "replay-receipt-replayed",
        outcome.receipt.decision == molten_core::world_replay::WorldReplayReceiptDecision::Replayed,
    );
    evidence.check(
        "successors-match-recorded-commits",
        outcome.receipt.horizon == steps.len() && outcome.divergence_record.is_none(),
    );
    evidence.check(
        "receipt-binds-current-admission",
        outcome.receipt.current_admission_ref.as_deref() == Some(outcome.admission.observation_ref.as_str()),
    );
    evidence.check(
        "replay-receipt-published-last",
        run.receipt_kinds.last() == Some(&crate::world_replay::WORLD_REPLAY_RECEIPT_RECORD),
    );
    evidence.runtime(outcome.receipt_record.value.clone());
    evidence.runtime(outcome.trace_record.value.clone());
    for receipt in run.authority_receipts {
        evidence.runtime(receipt);
    }
    Ok(evidence)
}

/// A complete, valid capsule replays nothing when current authority is revoked.
/// `revoke = false` is the control that must replay instead.
// r[impl molten.world_replay.execution_boundary]
pub(super) fn authority_denied(revoke: bool) -> crate::error::Result<super::super::Evidence> {
    let run = execute(revoke)?;
    let mut evidence = super::super::Evidence::default();
    let outcome = &run.outcome;
    let admission = &outcome.admission;
    evidence.check("capsule-complete-and-materialized", run.materialized == run.request.capsule.members.len());
    evidence.check("runtime-spine-snapshot-restored-once", run.restores == 1);
    evidence.check("current-authority-denied", !admission.authority_admitted);
    evidence.check(
        "other-current-admissions-held",
        admission.artifact_admitted
            && admission.schema_admitted
            && admission.resource_admitted
            && admission.runtime_admitted
            && admission.effect_admitted,
    );
    evidence.check(
        "replay-receipt-denied",
        outcome.receipt.decision == molten_core::world_replay::WorldReplayReceiptDecision::Denied,
    );
    evidence.check(
        "denied-before-any-transition",
        run.executed.is_empty() && outcome.executions.is_empty() && outcome.receipt.horizon == 0,
    );
    evidence.check("no-successor-captured", run.captured == 0 && outcome.captures.is_empty());
    evidence.check(
        "denial-receipt-published-last",
        run.receipt_kinds.last() == Some(&crate::world_replay::WORLD_REPLAY_RECEIPT_RECORD)
            && outcome.receipt.diagnostics == ["current replay admission denied"],
    );
    evidence.runtime(outcome.receipt_record.value.clone());
    for receipt in run.authority_receipts {
        evidence.runtime(receipt);
    }
    Ok(evidence)
}

fn dependency_refs() -> Vec<String> {
    [
        "molten runtime-spine committed turn",
        "molten world-commit capture shell",
        "molten world replay service",
    ]
    .map(super::runtime::reference)
    .to_vec()
}

/// The recorded inputs and the logical runtime profile, each keyed by the
/// identity of its canonical bytes.
fn closure_objects(
    recording: &history::Recording,
) -> crate::error::Result<std::collections::BTreeMap<String, Vec<u8>>> {
    let mut objects = recording
        .inputs
        .values()
        .map(|input| -> crate::error::Result<(String, Vec<u8>)> {
            Ok((input.step_ref()?, crate::preserves_rail::canonical_bytes(&input.to_value())?))
        })
        .collect::<crate::error::Result<std::collections::BTreeMap<_, _>>>()?;
    let profile_bytes =
        crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("logical-runtime-profile-v1", vec![
            crate::preserves_rail::string(crate::world_snapshot::LOCAL_LOGICAL_RUNTIME_PROFILE),
        ]))?;
    objects.insert(crate::preserves_rail::content_ref_from_bytes(&profile_bytes), profile_bytes);
    Ok(objects)
}

/// Every recorded commit, read back from the source store and re-identified.
fn commit_closures(
    recording: &history::Recording,
) -> crate::error::Result<Vec<molten_core::world_replay::WorldReplayCommitClosure>> {
    let mut closures = Vec::with_capacity(recording.commits.len());
    for commit in &recording.commits {
        let bytes = recording.store.read_commit(&commit.commit_ref).map_err(super::runtime::store_error)?;
        let is_verified = crate::world_commit::parse_canonical_world_commit_with_ref(
            &bytes,
            &commit.commit_ref,
            &super::runtime::commit_bounds(),
        )
        .is_ok_and(|parsed| parsed == *commit);
        closures.push(molten_core::world_replay::WorldReplayCommitClosure {
            commit_ref: commit.commit_ref.clone(),
            parents: commit.core.parents.clone(),
            roots: commit.core.roots.clone(),
            canonical_identity_verified: is_verified,
        });
    }
    Ok(closures)
}

/// The plan bounds the replay runs under: the molten-core defaults, spelled out.
fn plan_bounds() -> molten_core::world_replay::WorldReplayBounds {
    molten_core::world_replay::WorldReplayBounds {
        max_steps: molten_core::world_replay::MAX_WORLD_REPLAY_STEPS,
        max_members: molten_core::world_replay::MAX_WORLD_REPLAY_MEMBERS,
        max_member_bytes: molten_core::world_replay::MAX_WORLD_REPLAY_MEMBER_BYTES,
        max_total_bytes: molten_core::world_replay::MAX_WORLD_REPLAY_TOTAL_BYTES,
        max_field_path_segments: molten_core::world_replay::MAX_WORLD_REPLAY_FIELD_PATH_SEGMENTS,
        max_field_segment_bytes: molten_core::world_replay::MAX_WORLD_REPLAY_FIELD_SEGMENT_BYTES,
        max_diagnostics: molten_core::world_replay::MAX_WORLD_REPLAY_DIAGNOSTICS,
    }
}

/// The capsule over the recorded closure and the plan request that carries it.
fn sealed_plan<'a>(
    recording: &'a history::Recording,
    boundary: &'a molten_core::world_replay::WorldReplayObservationBoundary,
) -> crate::error::Result<(closure::CapsuleStore<'a>, molten_core::world_replay::WorldReplayPlanRequest)> {
    let objects = closure_objects(recording)?;
    let closures = commit_closures(recording)?;
    let mut requirements = molten_core::world_replay::required_world_replay_closure(&recording.trace, &closures, &[]);
    requirements.extend(molten_core::world_replay::required_world_replay_boundary_closure(boundary));
    requirements.sort();
    requirements.dedup();
    let capsule_store = closure::CapsuleStore {
        trace: &recording.trace,
        boundary,
        source: &recording.store,
        objects,
        materialized: 0,
    };
    let capsule = capsule_store.capsule(requirements)?;
    let request = molten_core::world_replay::WorldReplayPlanRequest {
        supported_profile_refs: vec![recording.trace.profile.profile_ref.as_str().to_string()],
        trace: recording.trace.clone(),
        capsule,
        commits: closures,
        additional_requirements: Vec::new(),
        bounds: plan_bounds(),
        boundary: boundary.clone(),
    };
    Ok((capsule_store, request))
}

/// Open the destination node and publish the current snapshot admission of the
/// recorded initial world.
fn admitted_destination(
    scratch: &std::path::Path,
    recording: &history::Recording,
) -> crate::error::Result<(super::fixture::NodeRoot, molten_core::world_snapshot::SnapshotDescriptor)> {
    let node = super::fixture::NodeRoot::open(&scratch.join("destination"), "replay-destination")?;
    let descriptor =
        node.adapter()?.prepare_bundle(&recording.initial.image(), recording.trace.initial_commit.clone())?;
    let snapshot_ref = super::fixture::descriptor_ref(&descriptor)?;
    let current = super::fixture::admission_document(AUTHORITY_GENERATION, SNAPSHOT_REVOCATION, true);
    node.admit(super::fixture::SnapshotGrant {
        signer: &node.signer,
        descriptor: &descriptor,
        descriptor_ref: &snapshot_ref,
        granted: &current,
        current: &current,
    })?;
    Ok((node, descriptor))
}

/// Sign the execution token and publish the current authority document; with
/// `revoke` the document revokes the token issuer.
fn published_authority(
    root: &crate::node_state::DirectoryRoot,
    request: &molten_core::world_replay::WorldReplayPlanRequest,
    revoke: bool,
) -> crate::error::Result<(
    ed25519_dalek::VerifyingKey,
    crate::capability_tokens::CapabilityToken,
    ed25519_dalek::Signature,
)> {
    let issuer = super::runtime::fixture_key("world replay execution authority");
    let (token, signature) = admission::execution_token(&issuer, request)?;
    let revoked = if revoke {
        vec![token.issuer_ref.clone()]
    } else {
        Vec::new()
    };
    admission::CurrentAuthority::publish(root, AUTHORITY_GENERATION, &revoked)?;
    Ok((issuer.verifying_key(), token, signature))
}

fn execute(revoke: bool) -> crate::error::Result<Run> {
    let scratch = super::super::scratch("world-replay")?;
    let source = super::fixture::NodeRoot::open(&scratch.path().join("source"), "replay-source")?;
    let recording = history::record(&source.root)?;
    let boundary = history::boundary(&recording.trace)?;
    let (mut capsule_store, request) = sealed_plan(&recording, &boundary)?;

    let (node, descriptor) = admitted_destination(scratch.path(), &recording)?;
    let (issuer, token, signature) = published_authority(&node.root, &request, revoke)?;

    let receipt_root = node.root.clone();
    let store = super::runtime::commit_store(&node.root)?;
    let shared = ports::Destination::shared(node, descriptor, store);
    let mut restore = ports::SnapshotRestore(std::rc::Rc::clone(&shared));
    let mut authority = admission::CurrentAuthority {
        destination: std::rc::Rc::clone(&shared),
        issuer,
        token,
        signature,
        boundary: boundary.clone(),
        receipts: Vec::new(),
    };
    let mut transitions = ports::RuntimeTransitions {
        destination: std::rc::Rc::clone(&shared),
        inputs: recording.inputs.clone(),
    };
    let mut capture_port = ports::RuntimeCapture(std::rc::Rc::clone(&shared));
    let mut receipts = ports::Receipts {
        root: receipt_root,
        kinds: Vec::new(),
    };
    let outcome = crate::world_replay::run_world_replay(
        &request,
        &recording.commits[0],
        &dependency_refs(),
        crate::world_replay::WorldReplayPorts {
            materialization: &mut capsule_store,
            restore: &mut restore,
            admission: &mut authority,
            transitions: &mut transitions,
            capture: &mut capture_port,
            receipts: &mut receipts,
        },
    )?;
    let materialized = capsule_store.materialized;
    let destination = shared.borrow();
    Ok(Run {
        request,
        outcome,
        materialized,
        restores: destination.restores,
        executed: destination.executed.clone(),
        captured: destination.captured,
        receipt_kinds: receipts.kinds,
        authority_receipts: authority.receipts,
    })
}
