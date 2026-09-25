
struct ChangeInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: &'a CoordinationRequest,
    snapshot: &'a CoordinationStateSnapshot,
}

struct PartsInput<'a> {
    transition: &'a PrimitiveTransitionResult,
    request: &'a CoordinationRequest,
    manifest: &'a CoordinationServiceManifest,
    engine_manifest: &'a crate::raft_control_plane::RaftGroupManifest,
    engine_epoch: u64,
    before_snapshot: &'a CoordinationStateSnapshot,
    snapshot: &'a CoordinationStateSnapshot,
    proposal_ref: &'a str,
}

struct PassReceiptInput<'a> {
    request: &'a CoordinationRequest,
    proposal_ref: &'a str,
    token_ref: Option<&'a str>,
    before_state_ref: &'a str,
    state_ref: &'a str,
    assertion_refs: &'a [String],
    output_refs: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct SuccessParts {
    token: Option<FencingToken>,
    receipt: CoordinationReceipt,
    assertion: CoordinationStatusAssertion,
}

struct ValuesInput<'a> {
    proposal: &'a Proposal,
    request: &'a CoordinationRequest,
    receipt: &'a CoordinationReceipt,
    token: Option<&'a FencingToken>,
    snapshot: &'a CoordinationStateSnapshot,
    assertion: &'a CoordinationStatusAssertion,
}

struct SuccessInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: CoordinationRequest,
    before_snapshot: CoordinationStateSnapshot,
    transition: PrimitiveTransitionResult,
    snapshot: CoordinationStateSnapshot,
    proposal: Proposal,
}
