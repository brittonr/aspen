const TRUST: &str = "coordination-operator.json";
const POLICY: &str = "coordination-policy.json";
const GRANT: &str = "coordination-grant.json";
const PROBE: &str = "coordination-probe.json";
const LIMIT: u64 = 1_048_576;
const NANOS_PER_MILLI: u64 = 1_000_000;
const CLOCK_PROFILE_ID: &str = "molten.live-reliability.authority-clock";
/// One hour of nanosecond ticks; the authority clock arms no deadlines of its own.
const CLOCK_MAX_TICKS: u64 = 3_600_000_000_000;
/// The authority clock admits no timers, runnables, entropy draws or scheduler work of its own.
const CLOCK_UNIT_BOUND: u64 = 1;

pub struct ProvisionInput {
    pub operator_secret: [u8; 32],
    pub policy: super::Policy,
    pub grant: super::Grant,
    pub probe_secret: [u8; 32],
}

/// Explicit node-owner provisioning, never invoked by runtime admission or source-gate loading.
pub fn provision(root: &crate::node_state::DirectoryRoot, input: ProvisionInput) -> crate::error::Result<()> {
    let operator = iroh::SecretKey::from_bytes(&input.operator_secret);
    let probe = iroh::SecretKey::from_bytes(&input.probe_secret);
    if input.policy.operator_public_key != operator.public().to_string()
        || input.grant.probe_public_key != probe.public().to_string()
    {
        return Err(super::denied("explicit issuer or dedicated probe key does not match provisioning input"));
    }
    let policy = super::SignedPolicy {
        signature: super::canonical::sign("operator-policy", &input.policy, &operator)?,
        policy: input.policy,
    };
    let grant = super::SignedGrant {
        signature: super::canonical::sign("operator-grant", &input.grant, &operator)?,
        grant: input.grant,
    };
    super::issuance::verify(&policy, &grant, &operator.public().to_string(), now()?)?;
    let secrets = root.secrets()?;
    for name in [TRUST, POLICY, GRANT, PROBE] {
        if secrets.try_exists(&path(name)?)? {
            return Err(super::denied("operator provisioning cannot replace existing authority"));
        }
    }
    persist(&secrets, PROBE, &input.probe_secret)?;
    persist(&secrets, POLICY, &policy)?;
    persist(&secrets, GRANT, &grant)?;
    // Trust is installed last: partial provisioning cannot admit effects.
    persist(&secrets, TRUST, &operator.public().to_string())?;
    load(root)?;
    Ok(())
}

pub fn load(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<super::VerifiedGrant> {
    load_at(root, now()?)
}

pub fn load_at(root: &crate::node_state::DirectoryRoot, now_ms: u64) -> crate::error::Result<super::VerifiedGrant> {
    let secrets = root.secrets()?;
    let issuer: String = protected(&secrets, TRUST)?;
    let policy = protected(&secrets, POLICY)?;
    let grant = protected(&secrets, GRANT)?;
    super::issuance::verify(&policy, &grant, &issuer, now_ms)
}

pub fn load_probe_key(
    root: &crate::node_state::DirectoryRoot,
    grant: &super::VerifiedGrant,
) -> crate::error::Result<iroh::SecretKey> {
    grant.verify_current(root)?;
    let bytes: [u8; 32] = protected(&root.secrets()?, PROBE)?;
    let key = iroh::SecretKey::from_bytes(&bytes);
    if key.public().to_string() != grant.probe_public_key() {
        return Err(super::denied("dedicated probe key differs from current grant"));
    }
    Ok(key)
}

fn path(name: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse(name)
}

fn protected<T: serde::de::DeserializeOwned>(scope: &crate::node_state::Scope, name: &str) -> crate::error::Result<T> {
    let locator = path(name)?;
    if scope.unix_mode(&locator)?.is_none_or(|mode| mode & 0o777 != 0o600) {
        return Err(super::denied("coordination authority must be owner-only mode 0600"));
    }
    serde_json::from_slice(&scope.read(&locator, LIMIT)?).map_err(|e| super::denied(e.to_string()))
}

fn persist<T: serde::Serialize>(scope: &crate::node_state::Scope, name: &str, value: &T) -> crate::error::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|e| super::denied(e.to_string()))?;
    scope.write_restricted(&path(name)?, &bytes, 0o600)
}

/// Wall time for grant validity windows, read only through the admitted fabric-time live clock.
fn now() -> crate::error::Result<u64> {
    let observation = wall_clock()?.observe_wall()?;
    Ok(observation.unix_nanos / NANOS_PER_MILLI)
}

fn wall_clock() -> crate::error::Result<crate::fabric_time::LiveClockAdapter> {
    let profile = crate::fabric_time::admit_profile(&crate::fabric_time::TimeProfileDescriptor {
        schema: crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: CLOCK_PROFILE_ID.to_string(),
        profile_ref: crate::preserves_rail::content_ref_from_bytes(CLOCK_PROFILE_ID.as_bytes()),
        kind: crate::fabric_time::TimeProfileKind::Live,
        supported_domains: crate::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: CLOCK_MAX_TICKS,
        max_uncertainty_ticks: CLOCK_UNIT_BOUND,
        max_timers: CLOCK_UNIT_BOUND,
        max_runnables: CLOCK_UNIT_BOUND,
        max_entropy_request_bytes: CLOCK_UNIT_BOUND,
        max_entropy_total_bytes: CLOCK_UNIT_BOUND,
        max_scheduler_concurrency: CLOCK_UNIT_BOUND,
        max_scheduler_queue_depth: CLOCK_UNIT_BOUND,
        fairness_bound_turns: None,
        scheduler_policy: crate::fabric_time::SchedulerPolicy {
            ordering: crate::fabric_time::SchedulerOrdering::PriorityThenFifo,
            replay: crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: crate::fabric_time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: crate::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    })?;
    crate::fabric_time::LiveClockAdapter::new(&profile.profile, 0)
}
