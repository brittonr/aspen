
#[derive(Debug, Clone, Copy)]
pub struct OutputRequestValueInput<'a> {
    pub dag_ref: &'a str,
    pub roots: &'a [String],
    pub materialization: &'a str,
    pub policy_refs: &'a [String],
    pub handler_profile_ref: Option<&'a str>,
    pub seed_config_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct SyncRequestValueInput<'a> {
    pub job_ref: &'a str,
    pub stage_ids: &'a [String],
    pub target_peer: &'a str,
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct AdmissionRequestValueInput<'a> {
    pub job_ref: &'a str,
    pub sync_ref: &'a str,
    pub stage_ids: &'a [String],
    pub target_peer: &'a str,
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub resource_refs: &'a [String],
}
