use super::*;

/// Exceeds the default Linux pipe capacity (64 KiB), so a writer to a child that
/// exits without reading must observe a closed pipe instead of buffering.
const PIPE_OVERFLOW_INPUT_BYTES: u64 = 262_144;

/// Admits a request whose child never reads stdin, so no input is supplied.
pub(super) fn canonical_request_without_input(
    kind: ExecutionProfileKind,
    arguments: Vec<String>,
) -> (CanonicalExecutionProfile, CanonicalExecutionRequest) {
    canonicalize_request(kind, request_without_input(arguments))
}

pub(super) fn request_without_input(arguments: Vec<String>) -> ExecutionRequest {
    ExecutionRequest {
        stdin_ref: None,
        ..request(arguments)
    }
}

// r[verify molten.fabric_execution.uncertainty]
// Pins today's conservative mapping: the pinned bounded-exec revision reports a
// normally exited child that closed stdin before consuming its input as a
// `WriteStdin` error, and the adapter keeps that post-start failure unknown.
// The follow-up that adopts bounded-exec's input-delivery observation revises
// this test to expect the normal publication path instead.
#[test]
fn live_adapter_keeps_unconsumed_oversized_input_unknown_without_completion_claim() {
    let mut oversized_request = request(script_arguments("exit 0"));
    oversized_request.limits.stdin_max_bytes = PIPE_OVERFLOW_INPUT_BYTES;
    let mut oversized_descriptor = descriptor(ExecutionProfileKind::LiveBoundedProcess);
    oversized_descriptor.max_stdin_bytes = PIPE_OVERFLOW_INPUT_BYTES;
    let (profile, request) = canonicalize_request_with_descriptor(&oversized_descriptor, oversized_request);
    let mut adapter = LiveExecutionAdapter::new(profile, MemoryPublisher::default()).expect("live adapter");
    let input = vec![0_u8; usize::try_from(PIPE_OVERFLOW_INPUT_BYTES).expect("fixture input fits the host")];
    let failure = adapter
        .execute(&request, &resolved(Some(input)), None)
        .expect_err("unconsumed oversized input is not a completion");
    assert_eq!(failure.kind, ExecutionPortFailureKind::UnknownAfterStart);
    assert!(failure.process_observation.is_none());
    assert!(failure.receipt.is_none());
    assert!(adapter.publisher().published.is_empty());
    assert_eq!(adapter.reconcile(HASH_B, GENERATION), ExecutionReconciliationStatus::UnknownRequiresReconciliation);
}
