use super::*;

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn receipt(
    decision: crate::system_extension::CallbackExecutionDecision,
) -> crate::system_extension::NativeCallbackValue {
    let invocation = crate::system_extension::CallbackInvocation {
        callback: crate::system_extension::CallbackKind::Request,
        generation: crate::system_extension::Generation::new(1),
        incarnation: crate::system_extension::Incarnation::new(0),
        sequence: 2,
        event_ref: reference("counter-event"),
        payload_ref: Some(reference("counter-request")),
        logical_tick: 1,
        deadline_tick: 10,
    };
    let outcome = crate::system_extension::CallbackOutcome {
        output_refs: vec![],
        effects: vec![],
        state_ref: Some(reference("counter-state")),
        checkpoint_ref: None,
        health: crate::system_extension::HealthState::Healthy,
    };
    let receipt = crate::system_extension::callback_receipt(crate::system_extension::CallbackReceiptInput {
        manifest_ref: &reference("manifest"),
        extension_id: "counter",
        service_id: "counter-service",
        execution_profile: crate::system_extension::ExecutionProfile::NativeProcess,
        invocation: &invocation,
        decision,
        outcome: Some(&outcome),
        diagnostic: None,
    })
    .expect("original canonical callback receipt");
    crate::system_extension::NativeCallbackValue {
        value_ref: receipt.receipt_ref,
        bytes: crate::preserves_rail::canonical_bytes(&receipt.value).expect("canonical receipt bytes"),
    }
}

#[test]
fn original_callback_receipt_is_strictly_decoded_not_reconstructed() {
    let value = receipt(crate::system_extension::CallbackExecutionDecision::Succeeded);
    let observed = read_receipt(&value).expect("original successful request receipt");
    assert_eq!(observed.state_ref, reference("counter-state"));
    assert_eq!(observed.event_ref, reference("counter-event"));
    assert_eq!(observed.sequence, 2);
    assert!(read_receipt(&receipt(crate::system_extension::CallbackExecutionDecision::ExecutorFailed)).is_err());
    let mut tampered = value;
    tampered.bytes.push(0);
    assert!(read_receipt(&tampered).is_err());
}

fn proof_record() -> (Commit, crate::system_extension::NativeInstanceRecord, String) {
    let content_ref = reference("counter-request");
    let proof = Commit {
        instance_id: "counter-instance".to_string(),
        ingress_request_ref: reference("native-ingress"),
        content_ref: content_ref.clone(),
        callback_receipt_ref: receipt(crate::system_extension::CallbackExecutionDecision::Succeeded).value_ref,
        event_ref: reference("counter-event"),
        generation: 1,
        sequence: 2,
        state: crate::system_extension::NativeCallbackValue {
            value_ref: reference("counter-state"),
            bytes: b"counter-state".to_vec(),
        },
        journal_commit_ref: reference("journal-record"),
    };
    let record = instance_record(&proof);
    (proof, record, content_ref)
}

#[test]
fn semantic_commit_requires_exact_payload_and_all_original_terminal_operations() {
    let (proof, mut record, content_ref) = proof_record();
    assert_eq!(proof.content_ref(), content_ref);
    proof.validate_record(&record, &content_ref, 1).expect("exact durable semantic chain");
    assert!(proof.validate_record(&record, &reference("another-request"), 1).is_err());
    assert!(proof.validate_record(&record, &content_ref, 2).is_err());
    let saved_state_ref = record.state_ref.replace(reference("unrelated-state"));
    assert!(proof.validate_record(&record, &content_ref, 1).is_err());
    record.state_ref = saved_state_ref;
    record.completed_operations.pop();
    assert!(
        proof.validate_record(&record, &content_ref, 1).is_err(),
        "missing persisted original callback receipt"
    );
}

#[test]
fn unknown_or_unrelated_callback_cannot_complete_work() {
    let (proof, mut record, content_ref) = proof_record();
    record.completed_operations[1].state = crate::system_extension::NativeOperationState::Unknown;
    assert!(proof.validate_record(&record, &content_ref, 1).is_err());
    record.completed_operations[1].state = crate::system_extension::NativeOperationState::Terminal;
    record.completed_operations[1].parent_ref = reference("unrelated-event");
    assert!(proof.validate_record(&record, &content_ref, 1).is_err());
}

fn completed_operations(proof: &Commit) -> Vec<crate::system_extension::NativeOperationRecord> {
    let manifest_ref = reference("manifest");
    let callback_ref = crate::system_extension::native_identity_ref(&[
        "native-callback-operation-v2",
        &proof.instance_id,
        &manifest_ref,
        "request",
        "1",
        "2",
        &proof.event_ref,
    ]);
    let operation = |operation_ref: String, parent_ref: String, kind, terminal_ref: String| {
        crate::system_extension::NativeOperationRecord {
            schema: crate::system_extension::NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref,
            parent_ref,
            kind,
            generation: crate::system_extension::Generation::new(1),
            state: crate::system_extension::NativeOperationState::Terminal,
            terminal_ref: Some(terminal_ref),
            is_retry_permitted: false,
        }
    };
    vec![
        operation(
            proof.ingress_request_ref.clone(),
            proof.content_ref.clone(),
            crate::system_extension::NativeOperationKind::Ingress,
            proof.callback_receipt_ref.clone(),
        ),
        operation(
            callback_ref.clone(),
            proof.event_ref.clone(),
            crate::system_extension::NativeOperationKind::Callback,
            reference("execution-receipt"),
        ),
        operation(
            publication_ref(&callback_ref, "state", &proof.state.value_ref, 1),
            callback_ref,
            crate::system_extension::NativeOperationKind::ValuePublication,
            reference("state-publication"),
        ),
        operation(
            publication_ref(&proof.ingress_request_ref, "ingress-callback-receipt", &proof.callback_receipt_ref, 1),
            proof.ingress_request_ref.clone(),
            crate::system_extension::NativeOperationKind::ValuePublication,
            reference("receipt-publication"),
        ),
    ]
}

fn instance_record(proof: &Commit) -> crate::system_extension::NativeInstanceRecord {
    crate::system_extension::NativeInstanceRecord {
        schema: crate::system_extension::NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
        instance_id: proof.instance_id.clone(),
        extension_id: "counter".to_string(),
        service_id: "counter-service".to_string(),
        manifest_ref: reference("manifest"),
        executable_ref: reference("executable"),
        profile_ref: reference("profile"),
        state_schema_ref: reference("schema"),
        lifecycle: crate::system_extension::LifecycleState {
            generation: crate::system_extension::Generation::new(1),
            incarnation: crate::system_extension::Incarnation::new(0),
            phase: crate::system_extension::LifecyclePhase::Running,
            restart_attempts: 0,
            health: crate::system_extension::HealthState::Healthy,
            checkpoint_ref: None,
        },
        usage: crate::system_extension::ResourceUsage::default(),
        callback_sequence: 2,
        event_sequence: 2,
        state_ref: Some(proof.state.value_ref.clone()),
        checkpoint_ref: None,
        unresolved: vec![],
        completed_operations: completed_operations(proof),
        completed_operation_refs: vec![],
        evidence_refs: vec![],
        is_accepting_ingress: true,
    }
}
