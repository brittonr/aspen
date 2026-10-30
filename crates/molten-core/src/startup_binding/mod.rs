//! Exact-candidate admission. Authentication and byte measurement belong to the shell.

pub mod closure;

use std::string;
use std::vec;

pub const SCHEMA: &str = "molten-node-source-binding-v1";
pub const PRODUCER_PROFILE: &str = "molten-nix-source-binding-v1";
pub const REQUIRED_ROLES: [&str; 4] = [
    "primary-cohort",
    "library-cohort",
    "gate-receipt",
    "remediation-receipt",
];

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub role: string::String,
    pub content_ref: string::String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub schema: string::String,
    pub producer_profile: string::String,
    pub executable_ref: string::String,
    pub source_ref: string::String,
    pub dependencies_ref: string::String,
    pub toolchain_ref: string::String,
    pub policy_ref: string::String,
    pub profile_ref: string::String,
    pub artifacts: vec::Vec<Artifact>,
}

/// Selected independently of the untrusted evidence carrier.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub binding_ref: string::String,
    pub policy_ref: string::String,
    pub profile_ref: string::String,
}

/// Facts supplied only after the shell verifies signatures and measures bytes.
pub struct Observation<'a> {
    pub binding_ref: &'a str,
    pub executing_ref: &'a str,
    pub authenticated_output: bool,
    pub artifacts: &'a [Artifact],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Issue {
    UnsupportedSchema,
    UnsupportedProducer,
    InvalidIdentity,
    UnauthenticatedOutput,
    StaleSelection,
    WrongCandidate,
    PolicyMismatch,
    ProfileMismatch,
    IncompleteCohort,
    ArtifactMismatch,
}

/// This decision grants only this source-gate claim, never runtime authority.
// r[impl molten.audit_f02.candidate_binding]
pub fn admit(binding: &Binding, selected: &Selection, observed: &Observation<'_>) -> Result<(), Issue> {
    if binding.schema != SCHEMA {
        return Err(Issue::UnsupportedSchema);
    }
    if binding.producer_profile != PRODUCER_PROFILE {
        return Err(Issue::UnsupportedProducer);
    }
    validate_identities(binding, selected, observed)?;
    if !observed.authenticated_output {
        return Err(Issue::UnauthenticatedOutput);
    }
    if selected.binding_ref != observed.binding_ref {
        return Err(Issue::StaleSelection);
    }
    if binding.executable_ref != observed.executing_ref {
        return Err(Issue::WrongCandidate);
    }
    if binding.policy_ref != selected.policy_ref {
        return Err(Issue::PolicyMismatch);
    }
    if binding.profile_ref != selected.profile_ref {
        return Err(Issue::ProfileMismatch);
    }
    validate_cohort(&binding.artifacts, observed.artifacts)
}

fn validate_identities(binding: &Binding, selected: &Selection, observed: &Observation<'_>) -> Result<(), Issue> {
    let identities = [
        binding.executable_ref.as_str(),
        binding.source_ref.as_str(),
        binding.dependencies_ref.as_str(),
        binding.toolchain_ref.as_str(),
        binding.policy_ref.as_str(),
        binding.profile_ref.as_str(),
        selected.binding_ref.as_str(),
        selected.policy_ref.as_str(),
        selected.profile_ref.as_str(),
        observed.binding_ref,
        observed.executing_ref,
    ];
    if !identities.into_iter().all(is_content_ref) {
        return Err(Issue::InvalidIdentity);
    }
    Ok(())
}

fn validate_cohort(expected: &[Artifact], actual: &[Artifact]) -> Result<(), Issue> {
    if expected.len() != REQUIRED_ROLES.len() || actual.len() != REQUIRED_ROLES.len() {
        return Err(Issue::IncompleteCohort);
    }
    for role in REQUIRED_ROLES {
        let mut declared = expected.iter().filter(|artifact| artifact.role == role);
        let mut measured = actual.iter().filter(|artifact| artifact.role == role);
        let (Some(declared_artifact), Some(measured_artifact)) = (declared.next(), measured.next()) else {
            return Err(Issue::IncompleteCohort);
        };
        if declared.next().is_some() || measured.next().is_some() {
            return Err(Issue::IncompleteCohort);
        }
        if !is_content_ref(&declared_artifact.content_ref) || !is_content_ref(&measured_artifact.content_ref) {
            return Err(Issue::InvalidIdentity);
        }
        if declared_artifact.content_ref != measured_artifact.content_ref {
            return Err(Issue::ArtifactMismatch);
        }
    }
    Ok(())
}

fn is_content_ref(reference: &str) -> bool {
    let Some(digest) = reference.strip_prefix("blake3:") else {
        return false;
    };
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests;
