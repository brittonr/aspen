use super::*;

// r[verify molten.fabric_execution.environment]
// r[verify molten.fabric_execution.output]
#[test]
fn live_adapter_clears_environment_round_trips_input_and_publishes_bounded_output() {
    let script =
        "if [ \"${HOME+x}\" = x ]; then exit 9; fi; IFS= read -r value; printf '%s:%s' \"$FIXTURE\" \"$value\"";
    let (profile, request) = canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(script));
    let mut adapter = LiveExecutionAdapter::new(profile, MemoryPublisher::default()).expect("live adapter");
    let receipt = adapter
        .execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("bounded live execution");
    assert_eq!(receipt.process.lifecycle, ExecutionLifecycleState::Exited);
    assert_eq!(receipt.process.disposition, ExecutionObservedDisposition::ExitPolicyAccepted);
    assert_eq!(receipt.process.stdout.retained_bytes, EXPECTED_STDOUT);
    assert_eq!(adapter.publisher().published.len(), 2);
    assert_eq!(adapter.reconcile(HASH_B, GENERATION), ExecutionReconciliationStatus::Terminal {
        receipt_ref: receipt.receipt_ref,
    });
}

// r[verify molten.fabric_execution.output]
#[test]
fn live_adapter_bounds_output_and_preserves_publication_failure_receipt() {
    let mut bounded_request = request(script_arguments(&format!("printf '{FLOOD_OUTPUT}'")));
    bounded_request.limits.stdout_max_bytes = SMALL_STREAM_BYTES;
    let (profile, request) = canonicalize_request(ExecutionProfileKind::LiveBoundedProcess, bounded_request);
    let mut adapter = LiveExecutionAdapter::new(profile, MemoryPublisher {
        fail: true,
        published: Vec::new(),
    })
    .expect("live adapter");
    let failure = adapter
        .execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect_err("publication failure must be explicit");
    assert_eq!(failure.kind, ExecutionPortFailureKind::OutputPublication);
    let receipt = failure.receipt.expect("process receipt survives publication failure");
    assert!(receipt.process.stdout.truncated);
    assert_eq!(receipt.process.stdout.retained_byte_count, SMALL_STREAM_BYTES);
    assert_eq!(receipt.process.disposition, ExecutionObservedDisposition::OutputPolicyRejected);
}

// r[verify molten.fabric_execution.lifecycle]
// r[verify molten.fabric_execution.validation]
#[test]
fn live_adapter_preserves_rejected_exit_and_descendant_teardown() {
    let rejected_script = format!("exit {REJECTED_EXIT_CODE}");
    let (rejected_profile, rejected_request) =
        canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(&rejected_script));
    let mut rejected_adapter =
        LiveExecutionAdapter::new(rejected_profile, MemoryPublisher::default()).expect("rejected exit adapter");
    let rejected = rejected_adapter
        .execute(&rejected_request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("rejected exit is a bounded observation");
    assert_eq!(rejected.process.exit_code, Some(REJECTED_EXIT_CODE));
    assert_eq!(rejected.process.disposition, ExecutionObservedDisposition::ExitPolicyRejected);

    let descendant_script = format!("({NON_TERMINATING_SCRIPT}) & printf 'bounded'");
    let (teardown_profile, teardown_request) =
        canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(&descendant_script));
    let mut teardown_adapter =
        LiveExecutionAdapter::new(teardown_profile, MemoryPublisher::default()).expect("teardown adapter");
    let teardown = teardown_adapter
        .execute(&teardown_request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("process-group teardown closes descendant-held pipes");
    assert_eq!(teardown.process.stdout.retained_bytes, b"bounded");
    assert!(teardown.process.teardown_observed);
}

// r[verify molten.fabric_execution.lifecycle]
#[test]
fn live_adapter_reports_timeout_cancellation_and_definite_spawn_failure() {
    let mut bounded_timeout_request = request(script_arguments(NON_TERMINATING_SCRIPT));
    bounded_timeout_request.limits.timeout_ms = SHORT_TIMEOUT_MS;
    let (timeout_profile, timeout_request) =
        canonicalize_request(ExecutionProfileKind::LiveBoundedProcess, bounded_timeout_request);
    let mut timeout_adapter =
        LiveExecutionAdapter::new(timeout_profile, MemoryPublisher::default()).expect("timeout adapter");
    let timed_out = timeout_adapter
        .execute(&timeout_request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("timeout is an observation");
    assert_eq!(timed_out.process.lifecycle, ExecutionLifecycleState::TimedOut);

    let (cancel_profile, cancel_request) =
        canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(NON_TERMINATING_SCRIPT));
    let mut cancel_adapter =
        LiveExecutionAdapter::new(cancel_profile, MemoryPublisher::default()).expect("cancel adapter");
    let cancellation = AtomicBool::new(true);
    let cancelled = cancel_adapter
        .execute(&cancel_request, &resolved(Some(INPUT_BYTES.to_vec())), Some(&cancellation))
        .expect("cancellation is an observation");
    assert_eq!(cancelled.process.lifecycle, ExecutionLifecycleState::Cancelled);

    let (missing_profile, missing_request) =
        canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments("exit 0"));
    let mut missing_adapter =
        LiveExecutionAdapter::new(missing_profile, MemoryPublisher::default()).expect("missing adapter");
    let mut missing = resolved(Some(INPUT_BYTES.to_vec()));
    missing.executable_path = PathBuf::from("/definitely/not/an/executable");
    let failure = missing_adapter
        .execute(&missing_request, &missing, None)
        .expect_err("missing executable must fail before start");
    assert_eq!(failure.kind, ExecutionPortFailureKind::RejectedBeforeStart);
    assert_eq!(
        missing_adapter.reconcile(HASH_B, GENERATION),
        ExecutionReconciliationStatus::DefinitePreStartFailure
    );
}
