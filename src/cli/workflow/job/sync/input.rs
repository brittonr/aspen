use super::io;

pub(super) struct AdmissionInput<'a> {
    pub(super) target_registry: &'a std::path::Path,
    pub(super) job: &'a str,
    pub(super) sync_ref: Option<&'a str>,
    pub(super) stages: &'a [String],
    pub(super) target_peer: &'a str,
    pub(super) policy_refs: Vec<String>,
    pub(super) capability_refs: Vec<String>,
    pub(super) evidence_refs: Vec<String>,
    pub(super) resource_refs: Vec<String>,
}

pub(super) struct ExecutionInput<'a> {
    pub(super) target_registry: &'a std::path::Path,
    pub(super) job: &'a str,
    pub(super) admission_value: &'a preserves::IOValue,
    pub(super) target_peer: &'a str,
    pub(super) stages: &'a [String],
    pub(super) policy_refs: Vec<String>,
    pub(super) capability_refs: Vec<String>,
    pub(super) resource_refs: Vec<String>,
}

pub(super) struct ExecutionFromAdmissionInput<'a> {
    pub(super) target_registry: &'a std::path::Path,
    pub(super) job: &'a str,
    pub(super) admission_ref: Option<&'a str>,
    pub(super) target_peer: &'a str,
    pub(super) stages: &'a [String],
    pub(super) policy_refs: Vec<String>,
    pub(super) capability_refs: Vec<String>,
    pub(super) resource_refs: Vec<String>,
}

pub(super) fn request(
    source_registry: &std::path::Path,
    job: &str,
    stages: &[String],
    target_peer: &str,
    extra_evidence_refs: &[String],
) -> molten::error::Result<preserves::IOValue> {
    let dag = molten::job_dag::read_job_dag_file_or_registry(source_registry, job)?;
    let mut evidence_refs = vec![io::synthetic_ref("sync-evidence", &dag.job_ref)?];
    evidence_refs.extend(extra_evidence_refs.iter().cloned());
    molten::job_dag::job_sync_request_value(molten::job_dag::SyncRequestValueInput {
        job_ref: &dag.job_ref,
        stage_ids: stages,
        target_peer,
        policy_refs: &[io::synthetic_ref("sync-policy", &dag.job_ref)?],
        capability_refs: &[io::synthetic_ref("sync-capability", &dag.job_ref)?],
        evidence_refs: &evidence_refs,
    })
}

pub(super) fn admission(input: AdmissionInput<'_>) -> molten::error::Result<preserves::IOValue> {
    let mut policy_refs = input.policy_refs;
    let mut capability_refs = input.capability_refs;
    let mut evidence_refs = input.evidence_refs;
    let mut resource_refs = input.resource_refs;
    let dag = molten::job_dag::read_job_dag_file_or_registry(input.target_registry, input.job)?;
    let sync_ref = match input.sync_ref {
        Some(value) => value.to_string(),
        None => io::synthetic_ref("sync-evidence", &dag.job_ref)?,
    };
    if policy_refs.is_empty() {
        policy_refs.push(io::synthetic_ref("admission-policy", &dag.job_ref)?);
    }
    if capability_refs.is_empty() {
        capability_refs.push(io::synthetic_ref("admission-capability", &dag.job_ref)?);
    }
    if !evidence_refs.iter().any(|reference| reference == &sync_ref) {
        evidence_refs.push(sync_ref.clone());
    }
    if !evidence_refs.iter().any(|reference| reference != &sync_ref) {
        evidence_refs.push(io::synthetic_ref("strict-octet-gate", &dag.job_ref)?);
    }
    if resource_refs.is_empty() {
        let selected = if input.stages.is_empty() {
            dag.nodes.len()
        } else {
            input.stages.len()
        };
        for index in 0..selected.max(1) {
            resource_refs.push(io::synthetic_ref("admission-resource", &format!("{}:{index}", dag.job_ref))?);
        }
    }
    molten::job_dag::job_admission_request_value(molten::job_dag::AdmissionRequestValueInput {
        job_ref: &dag.job_ref,
        sync_ref: &sync_ref,
        stage_ids: input.stages,
        target_peer: input.target_peer,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        evidence_refs: &evidence_refs,
        resource_refs: &resource_refs,
    })
}

pub(super) fn execution(input: ExecutionInput<'_>) -> molten::error::Result<preserves::IOValue> {
    let admission = molten::job_dag::parse_job_admission_receipt_value(input.admission_value)?;
    let selected_stages = if input.stages.is_empty() {
        admission.stage_order.clone()
    } else {
        input.stages.to_vec()
    };
    let mut capability_refs = input.capability_refs;
    if capability_refs.is_empty() {
        capability_refs.extend(admission.authority_receipt_refs.iter().cloned());
    }
    from_admission_ref(ExecutionFromAdmissionInput {
        target_registry: input.target_registry,
        job: input.job,
        admission_ref: Some(&admission.receipt_ref),
        target_peer: input.target_peer,
        stages: &selected_stages,
        policy_refs: input.policy_refs,
        capability_refs,
        resource_refs: input.resource_refs,
    })
}

pub(super) fn from_admission_ref(input: ExecutionFromAdmissionInput<'_>) -> molten::error::Result<preserves::IOValue> {
    let dag = molten::job_dag::read_job_dag_file_or_registry(input.target_registry, input.job)?;
    let admission_ref = match input.admission_ref {
        Some(value) => value.to_string(),
        None => io::synthetic_ref("missing-admission-receipt", &dag.job_ref)?,
    };
    let storage_profile = io::synthetic_ref("target-storage-profile", &dag.job_ref)?;
    let cache_profile = io::synthetic_ref("target-cache-profile", &dag.job_ref)?;
    let chunk_profile = io::synthetic_ref("target-chunk-profile", &dag.job_ref)?;
    molten::job_dag::job_execution_request_value(molten::job_dag::ExecutionRequestValueInput {
        job_ref: &dag.job_ref,
        admission_ref: &admission_ref,
        stage_ids: input.stages,
        target_peer: input.target_peer,
        storage_profile_ref: &storage_profile,
        cache_profile_ref: &cache_profile,
        chunk_profile_ref: &chunk_profile,
        policy_refs: &input.policy_refs,
        capability_refs: &input.capability_refs,
        resource_refs: &input.resource_refs,
    })
}
