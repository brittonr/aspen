pub(super) struct QueryCliInput {
    pub(super) pattern_kind: String,
    pub(super) pattern: String,
    pub(super) artifact_kinds: Vec<String>,
    pub(super) root_refs: Vec<String>,
    pub(super) dependency_inclusion_enabled: bool,
    pub(super) hidden_refs: Vec<String>,
}

pub(super) struct PlanCliInput {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) artifact_kinds: Vec<String>,
    pub(super) root_refs: Vec<String>,
    pub(super) dependency_inclusion_enabled: bool,
    pub(super) hidden_refs: Vec<String>,
}
