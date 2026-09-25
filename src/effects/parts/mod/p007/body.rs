
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleRequest<'a> {
    pub kind: &'a str,
    pub operation: &'a str,
    pub run_ref: &'a str,
    pub session_ref: &'a str,
    pub actor_ref: Option<&'a str>,
    pub turn_ref: Option<&'a str>,
    pub policy_ref: &'a str,
    pub capability_context_ref: &'a str,
    pub context_ref: Option<&'a str>,
    pub resource_refs: &'a [String],
    pub logical_time: u64,
    pub remote_use: bool,
    pub revoked_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHandleValidation {
    pub handler_binding_ref: String,
    pub handle_ref: String,
    pub checks: Vec<String>,
}
