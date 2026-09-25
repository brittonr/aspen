
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGate {
    pub receipt_ref: String,
    pub decision: String,
    pub install_ref: String,
    pub protocol_ref: String,
    pub session_ids: Vec<String>,
    pub initial_state_count: usize,
    pub operation_count: usize,
    pub message_count: usize,
    pub final_state_count: usize,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub install_ref: String,
    pub protocol_ref: String,
    pub session_ids: Vec<String>,
    pub initial_state_refs: Vec<String>,
    pub operation_refs: Vec<String>,
    pub message_refs: Vec<String>,
    pub final_state_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}
