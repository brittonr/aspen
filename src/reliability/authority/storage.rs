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

/// Mode and bytes come from one no-follow opened handle, so a leaf swapped between the mode check
/// and the read can never be admitted.
// r[impl molten.runtime_spine.admission_and_capability_hardening]
fn protected<T: serde::de::DeserializeOwned>(scope: &crate::node_state::Scope, name: &str) -> crate::error::Result<T> {
    let crate::node_state::FileObservation::Regular(file) = scope.observe_file(&path(name)?)? else {
        return Err(super::denied("coordination authority must be owner-only mode 0600"));
    };
    if file.unix_mode().is_none_or(|mode| mode & 0o777 != 0o600) {
        return Err(super::denied("coordination authority must be owner-only mode 0600"));
    }
    #[cfg(test)]
    protected_seam::after_mode_check();
    serde_json::from_slice(&file.read_bounded(LIMIT)?).map_err(|e| super::denied(e.to_string()))
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

/// Test-only hook between `protected`'s mode check and its read, so a regression can swap the leaf
/// at exactly that boundary.
#[cfg(test)]
mod protected_seam {
    type Hook = Box<dyn FnMut()>;

    thread_local! {
        static AFTER_MODE_CHECK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
    }

    pub(super) fn set_after_mode_check(hook: impl FnMut() + 'static) {
        AFTER_MODE_CHECK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }

    pub(super) fn after_mode_check() {
        let hook = AFTER_MODE_CHECK.with(|slot| slot.borrow_mut().take());
        if let Some(mut hook) = hook {
            hook();
        }
    }
}

#[cfg(all(test, unix))]
mod protected_tests {
    const LEAF: &str = "coordination-protected-swap.json";

    // r[verify molten.runtime_spine.admission_and_capability_hardening]
    #[test]
    fn reads_mode_and_bytes_from_one_handle() {
        use std::os::unix::fs::PermissionsExt;

        let workspace = crate::test_support::process_workspace("reliability-protected-swap").expect("workspace");
        let root = AsRef::<std::path::Path>::as_ref(&workspace).to_path_buf();
        let scope = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Secrets, &root).expect("scope");
        super::persist(&scope, LEAF, &"owner").expect("owner-only leaf");
        assert_eq!(super::protected::<String>(&scope, LEAF).expect("positive control"), "owner");

        // After the 0600 check passes, a loose intruder file replaces the leaf before the read.
        let (leaf, loose) = (root.join(LEAF), root.join("loose.copy"));
        std::fs::write(&loose, serde_json::to_vec("intruder").expect("json")).expect("loose copy");
        std::fs::set_permissions(&loose, std::fs::Permissions::from_mode(0o644)).expect("loose mode");
        let (swap_leaf, swap_loose) = (leaf.clone(), loose.clone());
        super::protected_seam::set_after_mode_check(move || {
            std::fs::rename(&swap_loose, &swap_leaf).expect("swap in loose leaf");
        });
        assert_eq!(
            super::protected::<String>(&scope, LEAF).expect("bytes from the checked handle"),
            "owner",
            "bytes from a non-0600 leaf were admitted"
        );
        // The swapped-in loose leaf is denied outright.
        assert!(super::protected::<String>(&scope, LEAF).is_err());
    }
}
