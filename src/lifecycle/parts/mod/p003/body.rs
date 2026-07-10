// Lifecycle probe, restart, and backoff governance.
//
// Pure core DTOs and validation for lifecycle health checks, restart
// decisions with named backoff profiles, and cleanup gate coordination.
// Borrows the Kubernetes probe/lifecycle idea but stays in canonical
// Preserves/capability terms.
//
// Type aliases and common helpers are inherited from p000.

const MAX_BACKOFF_PROFILES: usize = 32;
const MAX_RESTART_ATTEMPTS: u64 = 1_000;
const _: () = assert!(MAX_BACKOFF_PROFILES > 0);
const _: () = assert!(MAX_RESTART_ATTEMPTS > 0);

// ---------------------------------------------------------------------------
// Lifecycle probe DTOs
// ---------------------------------------------------------------------------

/// Kind of lifecycle probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeKind {
    Startup,
    Readiness,
    Liveness,
    GracefulShutdown,
}

impl ProbeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProbeKind::Startup => "startup",
            ProbeKind::Readiness => "readiness",
            ProbeKind::Liveness => "liveness",
            ProbeKind::GracefulShutdown => "graceful-shutdown",
        }
    }
}

/// Lifecycle probe configuration and result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleProbe {
    pub kind: ProbeKind,
    pub success: bool,
    pub observed_generation: u64,
    pub probe_evidence_ref: String,
    pub status_condition_ref: Option<String>,
    pub policy_refs: Vec<String>,
}

/// Backoff profile for restart governance.
#[derive(Debug, Clone, PartialEq)]
pub struct BackoffProfile {
    pub name: String,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub max_attempts: u64,
}

// ---------------------------------------------------------------------------
// Restart decision DTOs
// ---------------------------------------------------------------------------

/// Input for a restart decision.
#[derive(Debug, Clone, PartialEq)]
pub struct RestartDecisionInput {
    pub entity_ref: String,
    pub entity_kind: String,
    pub current_generation: u64,
    pub probe_results: Vec<LifecycleProbe>,
    pub prior_restart_attempts: u64,
    pub backoff_profile: Option<BackoffProfile>,
    pub authority_refs: Vec<String>,
    pub resource_budget_refs: Vec<String>,
}

/// Restart decision — pass, deny, or defer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartDecision {
    pub decision: String,
    pub attempt_number: u64,
    pub diagnostics: Vec<String>,
    pub status_condition_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pure core: probe validation
// ---------------------------------------------------------------------------

/// Validate lifecycle probe results for a resource.
pub fn validate_probe_results(
    probes: &[LifecycleProbe],
    current_generation: u64,
) -> Result<Vec<String>> {
    let mut status_refs = Vec::new();

    for probe in probes {
        require_ref(&probe.probe_evidence_ref, "probe evidence ref")?;

        if probe.observed_generation > current_generation {
            return Err(MoltenError::invalid_harness(format!(
                "probe observed generation {} exceeds current {}",
                probe.observed_generation, current_generation,
            )));
        }

        if let Some(ref status_ref) = probe.status_condition_ref {
            require_ref(status_ref, "status condition ref")?;
            status_refs.push(status_ref.clone());
        }
    }

    Ok(status_refs)
}

/// Detect flapping probes — rapid alternation between success/failure.
pub fn detect_flapping_probes(probes: &[LifecycleProbe], threshold: usize) -> Vec<String> {
    if probes.len() < 2 || probes.len() <= threshold {
        return Vec::new();
    }

    let mut changes = 0usize;
    for window in probes.windows(2) {
        if window[0].success != window[1].success {
            changes += 1;
        }
    }

    if changes >= threshold {
        vec!["flapping probes detected: rapid success/failure alternation".to_string()]
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Pure core: restart decision evaluation
// ---------------------------------------------------------------------------

/// Evaluate a restart decision with bounded backoff.
///
/// Restart decisions require explicit policy, authority, and probe evidence.
/// Unbounded restart loops deny.
pub fn evaluate_restart_decision(input: &RestartDecisionInput) -> Result<RestartDecision> {
    require_ref(&input.entity_ref, "entity ref")?;
    validate_non_empty(&input.entity_kind, "entity kind")?;

    if input.current_generation == 0 {
        return Err(MoltenError::invalid_harness(
            "current generation must be at least 1",
        ));
    }

    // Must have authority
    if input.authority_refs.is_empty() {
        return Ok(RestartDecision {
            decision: "deny".to_string(),
            attempt_number: input.prior_restart_attempts,
            diagnostics: vec!["missing restart authority evidence".to_string()],
            status_condition_refs: Vec::new(),
        });
    }

    // Validate backoff profile
    let profile = match &input.backoff_profile {
        Some(profile) => {
            validate_non_empty(&profile.name, "backoff profile name")?;
            if profile.max_attempts == 0 {
                return Err(MoltenError::invalid_harness(
                    "backoff profile max_attempts must be at least 1",
                ));
            }
            if profile.max_attempts > MAX_RESTART_ATTEMPTS {
                return Err(MoltenError::invalid_harness(format!(
                    "backoff profile max_attempts {} exceeds maximum {MAX_RESTART_ATTEMPTS}",
                    profile.max_attempts,
                )));
            }
            profile.clone()
        }
        None => {
            return Ok(RestartDecision {
                decision: "deny".to_string(),
                attempt_number: input.prior_restart_attempts,
                diagnostics: vec![
                    "unnamed backoff profile — restart must use a named profile".to_string(),
                ],
                status_condition_refs: Vec::new(),
            });
        }
    };

    // Check attempt budget
    if input.prior_restart_attempts >= profile.max_attempts {
        return Ok(RestartDecision {
            decision: "deny".to_string(),
            attempt_number: input.prior_restart_attempts,
            diagnostics: vec![format!(
                "restart budget exhausted: {} of {} attempts",
                input.prior_restart_attempts, profile.max_attempts,
            )],
            status_condition_refs: Vec::new(),
        });
    }

    // Must have probe evidence for liveness failure
    let liveness_failures: Vec<_> = input
        .probe_results
        .iter()
        .filter(|p| p.kind == ProbeKind::Liveness && !p.success)
        .collect();

    if liveness_failures.is_empty() && input.prior_restart_attempts > 0 {
        return Ok(RestartDecision {
            decision: "deny".to_string(),
            attempt_number: input.prior_restart_attempts,
            diagnostics: vec![
                "restart requested without liveness probe evidence".to_string(),
            ],
            status_condition_refs: Vec::new(),
        });
    }

    let next_attempt = input.prior_restart_attempts + 1;

    Ok(RestartDecision {
        decision: "pass".to_string(),
        attempt_number: next_attempt,
        diagnostics: Vec::new(),
        status_condition_refs: vec!["restarting".to_string()],
    })
}

/// Evaluate a readiness transition from probe results.
pub fn evaluate_readiness(probe: &LifecycleProbe, current_generation: u64) -> Result<String> {
    if probe.kind != ProbeKind::Readiness {
        return Err(MoltenError::invalid_harness(format!(
            "expected readiness probe but got {:?}",
            probe.kind,
        )));
    }

    require_ref(&probe.probe_evidence_ref, "readiness probe evidence ref")?;

    if probe.observed_generation != current_generation {
        return Err(MoltenError::invalid_harness(format!(
            "readiness probe observed generation {} != current {}",
            probe.observed_generation, current_generation,
        )));
    }

    Ok(if probe.success {
        "ready".to_string()
    } else {
        "not-ready".to_string()
    })
}

// ---------------------------------------------------------------------------
// Preserves encoding helpers
// ---------------------------------------------------------------------------

pub fn probe_to_value(probe: &LifecycleProbe) -> IoValue {
    record("lifecycle-probe-v1", vec![
        string(probe.kind.as_str()),
        bool_value(probe.success),
        u64_value(probe.observed_generation),
        string(&probe.probe_evidence_ref),
        optional_ref_value(probe.status_condition_ref.as_deref()),
        refs_sequence(&probe.policy_refs),
    ])
}

pub fn backoff_profile_to_value(profile: &BackoffProfile) -> IoValue {
    record("backoff-profile-v1", vec![
        string(&profile.name),
        u64_value(profile.initial_delay_ms),
        u64_value(profile.max_delay_ms),
        string(format!("{:.1}", profile.multiplier)),
        u64_value(profile.max_attempts),
    ])
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lifecycle/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lifecycle/parts/mod/tests/m000/p001/body.rs"));
}