//! Actual signed replacement ownership, with no uncertain-work retry or scope reactivation.

use preserves::ValueImpl;

pub(super) fn observe(
    cohort: &crate::producer::Cohort,
    observed: super::Observed,
) -> crate::producer::Result<preserves::IOValue> {
    let fixture = cohort.nodes.get(observed.actor).ok_or("replacement actor missing")?;
    let index = u64::try_from(observed.actor).map_err(|_| "replacement actor exceeds bound")?;
    let tick = observed.next_tick;
    let reopen_tick = tick.checked_add(3).ok_or("replacement tick overflow")?;
    let before_native = super::admission::readback::native(fixture)?;
    let before = inventory(cohort)?;
    let issuer = crate::authority::Issuer::new(crate::driver::operations::seed(index)?)?;
    let install = crate::driver::setup::authorize_replacement(&cohort.signed_main, fixture, &issuer, index, tick)?;
    let mut node = crate::driver::start(&cohort.signed_main, fixture, index, issuer)?;
    let serving = startup(&mut node, fixture, tick, &observed.work_ref)?;
    let admission = install_admission(fixture, &node.issuer, index, tick, &install)?;
    let output = crate::driver::process::reopened_status(&node, fixture, reopen_tick)?;
    let reopened = super::admission::readback::receipt(&output)?;
    super::admission::readback::require_unknown(&reopened, &observed.work_ref)?;
    let after_native = super::admission::readback::native(fixture)?;
    predecessor_preserved(&before_native, &after_native)?;
    let after = inventory(cohort)?;
    if before != after {
        return Err(
            "replacement changed Scope state, replay reservations or protected Raft proposal/commit inventory".into()
        );
    }
    Ok(molten::preserves_rail::record("collaboration-native-process-replacement-observation-v1", vec![
        observed.value,
        molten::preserves_rail::u64_value(index),
        molten::preserves_rail::string(&observed.work_ref),
        molten::system_extension::canonical_instance_record(&before_native)?.value,
        before,
        serving,
        admission,
        reopened,
        molten::system_extension::canonical_instance_record(&after_native)?.value,
        after,
        molten::preserves_rail::sequence(
            [
                "No Unknown delivery, callback, proposal, reactivation or reconciliation was retried.",
                "Replacement install admission is not scope mutation authority or effect completion.",
                "Replacement install proof binds the exact credential request digest and invocation nonce; the persisted replay value is basalt's invocation digest, not the producer request reference.",
                "Single-voter replacement does not prove restored quorum, availability or future recovery.",
                "Election hard-state records are excluded; all append/truncate/commit records are compared.",
                "Replacement activation may re-observe its own instance; the predecessor effect, consumed completions and generation/incarnation fence stay identical.",
            ]
            .into_iter()
            .map(molten::preserves_rail::string)
            .collect(),
        ),
    ]))
}

/// A replacement may activate and re-observe its own instance, but it must retain the exact
/// predecessor effect — either unresolved Unknown or produced but unconsumed — and consume no
/// completion for it, while staying in the same generation/incarnation fence.
fn predecessor_preserved(
    before: &molten::system_extension::NativeInstanceRecord,
    after: &molten::system_extension::NativeInstanceRecord,
) -> crate::producer::Result<()> {
    let predecessors: Vec<_> = before
        .unresolved
        .iter()
        .chain(before.completed_operations.iter())
        .filter(|operation| operation.kind == molten::system_extension::NativeOperationKind::Effect)
        .collect();
    let [predecessor] = predecessors.as_slice() else {
        return Err("replacement requires exactly one predecessor effect".into());
    };
    let is_retained = after.unresolved.iter().chain(after.completed_operations.iter()).any(|operation| {
        operation.operation_ref == predecessor.operation_ref
            && operation.kind == predecessor.kind
            && operation.state == predecessor.state
            && operation.terminal_ref == predecessor.terminal_ref
    });
    if !is_retained
        || after.completed_operation_refs.contains(&predecessor.operation_ref)
        || before.unresolved != after.unresolved
        || before.completed_operation_refs != after.completed_operation_refs
        || before.lifecycle.generation != after.lifecycle.generation
        || before.lifecycle.incarnation != after.lifecycle.incarnation
        || after.completed_operations.iter().any(|operation| {
            operation.kind == molten::system_extension::NativeOperationKind::Effect
                && operation.parent_ref == predecessor.parent_ref
                && operation.operation_ref != predecessor.operation_ref
        })
    {
        return Err("replacement did not preserve the exact predecessor effect fence".into());
    }
    Ok(())
}

fn startup(
    node: &mut crate::driver::Node,
    fixture: &crate::producer::Node,
    tick: u64,
    work_ref: &str,
) -> crate::producer::Result<preserves::IOValue> {
    let preview_tick = tick.checked_add(1).ok_or("replacement tick overflow")?;
    let status_tick = tick.checked_add(2).ok_or("replacement tick overflow")?;
    Ok(match crate::driver::await_ready(node) {
        Ok(output) => {
            let ready = molten::preserves_rail::parse_text(crate::driver::flow::value(&output, crate::driver::READY)?)?;
            let fields = molten::preserves_rail::simple_record_fields(&ready, crate::driver::READY, 4)?;
            if crate::driver::operations::wire::text(&fields[0])? != fixture.bundle_ref
                || crate::driver::operations::wire::number(&fields[2])? != tick
            {
                return Err("replacement readiness did not identify its fresh startup".into());
            }
            let preview = fenced_preview(node, fixture, preview_tick)?;
            let status = super::admission::readback::live_status(node, fixture, status_tick)?;
            super::admission::readback::require_unknown(&status, work_ref)?;
            crate::driver::finish(node)?;
            molten::preserves_rail::record("collaboration-native-replacement-ready-v1", vec![ready, preview, status])
        }
        Err(error) => {
            // A readiness timeout, panic, signal or still-owned child is not a predecessor denial.
            let exit = node
                .child
                .try_wait()?
                .ok_or_else(|| format!("replacement did not exit before readiness: {error}"))?;
            let control = node.control.as_mut().ok_or("replacement control missing")?;
            control.close();
            control.join();
            if exit.code() != Some(1) {
                return Err(format!("replacement startup did not exit with ordinary CLI error: {exit}").into());
            }
            let output = std::fs::read_to_string(&node.log)?;
            if output.lines().any(|line| line.trim_start().starts_with("<node-service-host-ready-v1")) {
                return Err("failed replacement published readiness".into());
            }
            molten::preserves_rail::record("collaboration-native-replacement-exited-v1", vec![
                molten::preserves_rail::u64_value(tick),
                molten::preserves_rail::u64_value(1),
            ])
        }
    })
}

/// Preview only: no observation, signing request, holder grant or effect may escape the fence.
fn fenced_preview(
    node: &crate::driver::Node,
    fixture: &crate::producer::Node,
    tick: u64,
) -> crate::producer::Result<preserves::IOValue> {
    let read = crate::producer::reference(&format!("replacement-scope-read-{}-{tick}", node.index));
    let operation = crate::driver::operations::wire::operation("scope-observe", vec![
        molten::preserves_rail::string(crate::producer::SERVICE),
        molten::preserves_rail::string(&read),
    ]);
    fixture.root.write(
        &molten::node_state::RelativePath::parse("operation.preserves")?,
        &molten::preserves_rail::canonical_bytes(&operation)?,
    )?;
    let (success, output) = crate::driver::process::invoke(
        &crate::driver::process::host_of(node),
        "scope-observe",
        tick,
        Some("--preview"),
    )?;
    if success
        || output.contains("<node-service-host-operation-request-v1")
        || output.contains("<node-service-host-scope-observed-v1")
        || output.contains("<collaboration-scope-observation-v1")
    {
        return Err("replacement unresolved predecessor exposed a scope observation or signing request".into());
    }
    // Retained control only discloses its closed public error vocabulary, never raw process logs.
    Ok(molten::preserves_rail::record("collaboration-native-replacement-scope-preview-denied-v1", vec![
        molten::preserves_rail::u64_value(tick),
        molten::preserves_rail::string(&read),
        molten::preserves_rail::string(output),
    ]))
}

/// Read the actual verifier's no-replace admission, addressed by this exact holder invocation.
fn install_admission(
    fixture: &crate::producer::Node,
    issuer: &crate::authority::Issuer,
    index: u64,
    tick: u64,
    install: &molten::node_service_hosting::OperationRequest,
) -> crate::producer::Result<preserves::IOValue> {
    let configuration = molten::node_service_host::effects::config::reference(&fixture.effects)?;
    let reference = molten::node_service_host::authority::request_ref(&fixture.bundle_ref, install)?;
    if install.operation != "install-effects"
        || install.logical_tick != tick
        || install.subject != configuration
        || install.state_ref != configuration
    {
        return Err("replacement install request is not bound to the exact current effects configuration".into());
    }
    let key = molten::preserves_rail::canonical_hash(&molten::preserves_rail::record(
        "node-service-host-replay-key-v1",
        vec![
            molten::preserves_rail::string(issuer.holder_did()),
            preserves::IOValue::bytes(crate::driver::process::nonce(index, tick, 3)?),
        ],
    ))?;
    // The credential path is the exact producer request digest, so its file binds this request.
    let credential = molten::node_service_host::authority::wire::decode_credential(
        &fixture.root.read(&molten::node_service_host::authority::credential_path(&reference)?, 1_048_576)?,
    )?;
    if credential.nonce != crate::driver::process::nonce(index, tick, 3)? || credential.token.is_empty() {
        return Err("actual replacement install credential does not bind the exact invocation nonce".into());
    }
    let bytes = fixture.root.read(
        &molten::node_state::RelativePath::parse(&format!("service-host-replay-{}.preserves", &key[7..]))?,
        65_536,
    )?;
    let receipt = molten::preserves_rail::parse_canonical_bytes(&bytes)?;
    let fields = molten::preserves_rail::simple_record_fields(&receipt, "node-service-host-replay-admission-v1", 2)?;
    let digest: [u8; 32] = fields[1]
        .as_bytestring()
        .ok_or("install admission digest missing")?
        .as_ref()
        .try_into()
        .map_err(|_| "install admission digest width")?;
    if crate::driver::operations::wire::text(&fields[0])? != key {
        return Err("actual replacement install replay admission key mismatched".into());
    }
    // The stored value is basalt's canonical holder-invocation digest, not the producer request
    // reference; the exact request binding is proven by the credential path and nonce above.
    if digest == [0; 32] {
        return Err("actual replacement install replay admission digest is empty".into());
    }
    Ok(receipt)
}

/// Read every application's state and retain all durable protected Raft mutations, in order.
fn inventory(cohort: &crate::producer::Cohort) -> crate::producer::Result<preserves::IOValue> {
    let mut nodes = Vec::with_capacity(cohort.nodes.len());
    for fixture in &cohort.nodes {
        let retained = super::uncommitted::inventory(fixture)?;
        let records = molten::preserves_rail::required_sequence_field(&retained, "actual replacement Raft records")?;
        let proposals = records
            .as_slice()
            .iter()
            .filter(|value| {
                value.collect_simple_record("raft-log-mutation-v1", Some(2)).is_some()
                    || value.collect_simple_record("raft-commit-boundary-v1", Some(1)).is_some()
            })
            .map(molten::preserves_rail::value_to_iovalue)
            .collect();
        let state = crate::driver::scope::persisted(fixture)?;
        nodes.push(molten::preserves_rail::record("collaboration-native-replacement-durable-inventory-v1", vec![
            molten::preserves_rail::string(&fixture.runtime.node_id),
            molten::preserves_rail::sequence(proposals),
            molten::collaboration_scope::state_value(&state),
            replay(fixture)?,
        ]));
    }
    Ok(molten::preserves_rail::sequence(nodes))
}

/// Compare semantic replay keys and exact admitted digests, not incidental journal layout.
fn replay(fixture: &crate::producer::Node) -> crate::producer::Result<preserves::IOValue> {
    let input = &fixture.runtime.scope_store;
    let profile = input.profile.clone().into_profile()?;
    let namespace = input.namespace.clone().into_descriptor(&profile)?;
    let root = molten::local_store::DurableStoreRoot::from_ledger(
        &fixture.root.ledger_store()?,
        &molten::local_store::Locator::parse(&input.storage_namespace)?,
    )?;
    let adapter = molten::fabric_durability::RedbDurableStateAdapter::open_root(root, profile, namespace)?;
    let values: crate::producer::Result<Vec<_>> = adapter
        .state()
        .ordered
        .iter()
        .filter(|(key, _)| key.starts_with(b"collaboration/replay/"))
        .map(|(key, stored)| {
            let value = molten::preserves_rail::parse_canonical_bytes(&stored.value)?;
            let fields = molten::preserves_rail::simple_record_fields(&value, "collaboration-replay-admission-v1", 2)?;
            let reference = crate::driver::operations::wire::text(&fields[0])?;
            if key.as_slice() != format!("collaboration/replay/{reference}").as_bytes()
                || molten::preserves_rail::canonical_hash(&value)? != stored.value_ref
            {
                return Err("actual Scope replay admission key or value identity mismatched".into());
            }
            Ok(value)
        })
        .collect();
    Ok(molten::preserves_rail::sequence(values?))
}
