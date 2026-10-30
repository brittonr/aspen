#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulerPolicyDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub ordering: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub replay: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub overload: String,
}

impl SchedulerPolicyDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_time::SchedulerPolicy> {
        Ok(crate::fabric_time::SchedulerPolicy {
            ordering: super::choice(
                &self.ordering,
                &[
                    crate::fabric_time::SchedulerOrdering::Fifo,
                    crate::fabric_time::SchedulerOrdering::PriorityThenFifo,
                ],
                crate::fabric_time::SchedulerOrdering::as_str,
            )?,
            replay: super::choice(
                &self.replay,
                &[
                    crate::fabric_time::SchedulerReplayPolicy::Deterministic,
                    crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
                ],
                crate::fabric_time::SchedulerReplayPolicy::as_str,
            )?,
            overload: super::choice(
                &self.overload,
                &[
                    crate::fabric_time::SchedulerOverloadPolicy::Reject,
                    crate::fabric_time::SchedulerOverloadPolicy::Backpressure,
                ],
                crate::fabric_time::SchedulerOverloadPolicy::as_str,
            )?,
        })
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeProfileDescriptorDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub schema: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_ref: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub kind: String,
    #[serde(deserialize_with = "super::bounded::texts")]
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
    pub scheduler_policy: SchedulerPolicyDto,
    #[serde(deserialize_with = "super::bounded::text")]
    pub evidence_mode: String,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub non_claims: Vec<String>,
}

impl TimeProfileDescriptorDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_time::TimeProfileDescriptor> {
        Ok(crate::fabric_time::TimeProfileDescriptor {
            schema: self.schema,
            profile_id: self.profile_id,
            profile_ref: self.profile_ref,
            kind: super::choice(
                &self.kind,
                &[
                    crate::fabric_time::TimeProfileKind::Live,
                    crate::fabric_time::TimeProfileKind::DeterministicSimulation,
                ],
                crate::fabric_time::TimeProfileKind::as_str,
            )?,
            supported_domains: super::choices(
                self.supported_domains,
                &[
                    crate::fabric_time::TimeDomain::WallClock,
                    crate::fabric_time::TimeDomain::Monotonic,
                    crate::fabric_time::TimeDomain::Logical,
                    crate::fabric_time::TimeDomain::Virtual,
                ],
                crate::fabric_time::TimeDomain::as_str,
            )?,
            max_duration_ticks: self.max_duration_ticks,
            max_uncertainty_ticks: self.max_uncertainty_ticks,
            max_timers: self.max_timers,
            max_runnables: self.max_runnables,
            max_entropy_request_bytes: self.max_entropy_request_bytes,
            max_entropy_total_bytes: self.max_entropy_total_bytes,
            max_scheduler_concurrency: self.max_scheduler_concurrency,
            max_scheduler_queue_depth: self.max_scheduler_queue_depth,
            fairness_bound_turns: self.fairness_bound_turns,
            scheduler_policy: self.scheduler_policy.into_raw()?,
            evidence_mode: super::choice(
                &self.evidence_mode,
                &[
                    crate::fabric_time::TimeEvidenceMode::Aggregate,
                    crate::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
                ],
                crate::fabric_time::TimeEvidenceMode::as_str,
            )?,
            non_claims: super::choices(
                self.non_claims,
                &crate::fabric_time::REQUIRED_TIME_NON_CLAIMS,
                crate::fabric_time::TimeNonClaim::as_str,
            )?,
        })
    }
    pub fn into_descriptor(self) -> crate::error::Result<crate::fabric_time::Profile> {
        crate::fabric_time::admit_profile(&self.into_raw()?)
    }
}
