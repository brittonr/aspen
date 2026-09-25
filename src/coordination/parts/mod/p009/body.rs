
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarrierState {
    pub participants: OrderedSet<String>,
    pub required: u64,
    pub is_released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub endpoint_ref: String,
    pub evidence_ref: String,
}

// r[impl molten.coordination_state_machine_proof.primitive_transition_cores]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveTransitionResult {
    pub kind: String,
    pub decision: String,
    pub before_state: CoordinationState,
    pub after_state: CoordinationState,
    pub token: Option<FencingToken>,
    pub status_fact: IoValue,
    pub output_facts: Vec<IoValue>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(&'static str, &'static str)>,
    pub shell_intents: Vec<String>,
}

impl PrimitiveTransitionResult {
    fn state_for_receipt(&self) -> &CoordinationState {
        if self.decision == "pass" {
            &self.after_state
        } else {
            &self.before_state
        }
    }
}

#[derive(Debug, Clone)]
struct PreparedMutation {
    state: CoordinationState,
    token: Option<FencingToken>,
    status_fact: IoValue,
    checks: Vec<(&'static str, &'static str)>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptTransitionInput<'a> {
    pub kind: &'a str,
    pub before_state_ref: &'a str,
    pub after_state_ref: Option<&'a str>,
    pub preserved_state_ref: Option<&'a str>,
    pub output_refs: &'a [String],
    pub control_plane_intent_ref: Option<&'a str>,
    pub prior_receipt_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptValueInput<'a> {
    pub decision: &'a str,
    pub service: &'a str,
    pub operation: &'a str,
    pub read_consistency_mode: &'a str,
    pub request_ref: &'a str,
    pub raft_receipt_ref: Option<&'a str>,
    pub token_ref: Option<&'a str>,
    pub state_ref: &'a str,
    pub transition: ReceiptTransitionInput<'a>,
    pub dataspace_assertion_refs: &'a [String],
    pub diagnostics: &'a [String],
    pub checks: &'a [(&'a str, &'a str)],
}
