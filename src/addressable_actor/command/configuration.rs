//! Exact wire selections for the public actor command; references select, never authorize.

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeSelection {
    pub canonical_ref: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub kind: String,
    pub supported_domains: Vec<String>,
    pub max_duration_ticks: u64,
    pub max_uncertainty_ticks: u64,
    pub max_timers: u64,
    pub max_runnables: u64,
    pub max_entropy_request_bytes: u64,
    pub max_entropy_total_bytes: u64,
    pub max_scheduler_concurrency: u64,
    pub max_scheduler_queue_depth: u64,
    pub fairness_bound_turns: Option<u64>,
    pub ordering: String,
    pub replay: String,
    pub overload: String,
    pub evidence_mode: String,
    pub non_claims: Vec<String>,
}

impl TimeSelection {
    pub fn from_profile(profile: &crate::fabric_time::Profile) -> Self {
        let p = &profile.profile;
        Self {
            canonical_ref: profile.profile_ref.clone(),
            profile_id: p.profile_id.clone(),
            profile_ref: p.profile_ref.clone(),
            kind: p.kind.as_str().to_string(),
            supported_domains: p.supported_domains.iter().map(|v| v.as_str().to_string()).collect(),
            max_duration_ticks: p.max_duration_ticks,
            max_uncertainty_ticks: p.max_uncertainty_ticks,
            max_timers: p.max_timers,
            max_runnables: p.max_runnables,
            max_entropy_request_bytes: p.max_entropy_request_bytes,
            max_entropy_total_bytes: p.max_entropy_total_bytes,
            max_scheduler_concurrency: p.max_scheduler_concurrency,
            max_scheduler_queue_depth: p.max_scheduler_queue_depth,
            fairness_bound_turns: p.fairness_bound_turns,
            ordering: p.scheduler_policy.ordering.as_str().to_string(),
            replay: p.scheduler_policy.replay.as_str().to_string(),
            overload: p.scheduler_policy.overload.as_str().to_string(),
            evidence_mode: p.evidence_mode.as_str().to_string(),
            non_claims: p.non_claims.iter().map(|v| v.as_str().to_string()).collect(),
        }
    }

    pub fn admit(&self) -> crate::error::Result<crate::fabric_time::Profile> {
        let required: Vec<_> =
            crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.iter().map(|v| v.as_str().to_string()).collect();
        let mut actual = self.non_claims.clone();
        let mut expected = required;
        actual.sort();
        expected.sort();
        if actual != expected {
            return Err(invalid("time non-claims differ from admitted contract"));
        }
        let kind = match self.kind.as_str() {
            "live" => crate::fabric_time::TimeProfileKind::Live,
            "deterministic-simulation" => crate::fabric_time::TimeProfileKind::DeterministicSimulation,
            _ => return Err(invalid("unknown time profile kind")),
        };
        let supported_domains = self
            .supported_domains
            .iter()
            .map(|domain| match domain.as_str() {
                "wall-clock" => Ok(crate::fabric_time::TimeDomain::WallClock),
                "monotonic" => Ok(crate::fabric_time::TimeDomain::Monotonic),
                "logical" => Ok(crate::fabric_time::TimeDomain::Logical),
                "virtual" => Ok(crate::fabric_time::TimeDomain::Virtual),
                _ => Err(invalid("unknown time domain")),
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        let ordering = match self.ordering.as_str() {
            "fifo" => crate::fabric_time::SchedulerOrdering::Fifo,
            "priority-then-fifo" => crate::fabric_time::SchedulerOrdering::PriorityThenFifo,
            _ => return Err(invalid("unknown scheduler ordering")),
        };
        let replay = match self.replay.as_str() {
            "deterministic" => crate::fabric_time::SchedulerReplayPolicy::Deterministic,
            "recorded-choice-required" => crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
            _ => return Err(invalid("unknown scheduler replay policy")),
        };
        let overload = match self.overload.as_str() {
            "reject" => crate::fabric_time::SchedulerOverloadPolicy::Reject,
            _ => return Err(invalid("unsupported scheduler overload policy")),
        };
        let evidence_mode = match self.evidence_mode.as_str() {
            "aggregate" => crate::fabric_time::TimeEvidenceMode::Aggregate,
            "selected-semantic-boundaries" => crate::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
            _ => return Err(invalid("unknown time evidence mode")),
        };
        let admitted = crate::fabric_time::admit_profile(&crate::fabric_time::TimeProfileDescriptor {
            schema: crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
            profile_id: self.profile_id.clone(),
            profile_ref: self.profile_ref.clone(),
            kind,
            supported_domains,
            max_duration_ticks: self.max_duration_ticks,
            max_uncertainty_ticks: self.max_uncertainty_ticks,
            max_timers: self.max_timers,
            max_runnables: self.max_runnables,
            max_entropy_request_bytes: self.max_entropy_request_bytes,
            max_entropy_total_bytes: self.max_entropy_total_bytes,
            max_scheduler_concurrency: self.max_scheduler_concurrency,
            max_scheduler_queue_depth: self.max_scheduler_queue_depth,
            fairness_bound_turns: self.fairness_bound_turns,
            scheduler_policy: crate::fabric_time::SchedulerPolicy {
                ordering,
                replay,
                overload,
            },
            evidence_mode,
            non_claims: crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
        })?;
        if admitted.profile_ref != self.canonical_ref {
            return Err(invalid("time descriptor differs from exact canonical selection"));
        }
        Ok(admitted)
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipSelection {
    pub canonical_ref: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub provider_kind: String,
    pub authority_strength: String,
    pub authority_scope: String,
    pub max_view_age_ticks: u64,
    pub non_claims: Vec<String>,
}

impl MembershipSelection {
    pub fn from_profile(profile: &crate::fabric_membership::Profile) -> Self {
        let p = &profile.profile;
        Self {
            canonical_ref: profile.admission_ref.clone(),
            profile_id: p.profile_id.clone(),
            profile_ref: p.profile_ref.clone(),
            provider_kind: p.provider_kind.as_str().to_string(),
            authority_strength: p.authority_strength.as_str().to_string(),
            authority_scope: p.authority_scope.clone(),
            max_view_age_ticks: p.max_view_age_ticks,
            non_claims: p.non_claims.iter().map(|v| v.as_str().to_string()).collect(),
        }
    }
    pub fn admit(&self) -> crate::error::Result<crate::fabric_membership::Profile> {
        let claims: Vec<_> = crate::fabric_membership::REQUIRED_MEMBERSHIP_NON_CLAIMS
            .iter()
            .map(|v| v.as_str().to_string())
            .collect();
        if self.non_claims != claims {
            return Err(invalid("membership non-claims differ from admitted contract"));
        }
        let provider_kind = match self.provider_kind.as_str() {
            "static" => crate::fabric_membership::MembershipProviderKind::Static,
            "policy-managed" => crate::fabric_membership::MembershipProviderKind::PolicyManaged,
            "consistency-backed" => crate::fabric_membership::MembershipProviderKind::ConsistencyBacked,
            "deterministic-simulation" => crate::fabric_membership::MembershipProviderKind::DeterministicSimulation,
            _ => return Err(invalid("unknown membership provider")),
        };
        let authority_strength = match self.authority_strength.as_str() {
            "observation-only" => crate::fabric_membership::MembershipAuthorityStrength::ObservationOnly,
            "operator-declared" => crate::fabric_membership::MembershipAuthorityStrength::OperatorDeclared,
            "consistency-ordered" => crate::fabric_membership::MembershipAuthorityStrength::ConsistencyOrdered,
            "externally-enforced" => crate::fabric_membership::MembershipAuthorityStrength::ExternallyEnforced,
            _ => return Err(invalid("unknown membership authority strength")),
        };
        let profile = crate::fabric_membership::profile(&crate::fabric_membership::MembershipSourceProfile {
            schema: crate::fabric_membership::MEMBERSHIP_SOURCE_PROFILE_SCHEMA.to_string(),
            profile_id: self.profile_id.clone(),
            profile_ref: self.profile_ref.clone(),
            provider_kind,
            authority_strength,
            authority_scope: self.authority_scope.clone(),
            max_view_age_ticks: self.max_view_age_ticks,
            non_claims: crate::fabric_membership::REQUIRED_MEMBERSHIP_NON_CLAIMS.to_vec(),
        })?;
        if profile.admission_ref != self.canonical_ref {
            return Err(invalid("membership profile differs from exact canonical selection"));
        }
        Ok(profile)
    }
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
