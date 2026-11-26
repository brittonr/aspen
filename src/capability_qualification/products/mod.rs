//! Native product producers. Each fixed scenario drives the actual runtime path
//! of one reviewed capability (Redb store, live bounded process, Iroh peer
//! process, world stores, live Raft replica processes) and reduces what it
//! observed to a deterministic canonical observation: invariant checks plus the
//! canonical runtime receipts the runtime itself emitted. Scratch paths, process
//! ids, ports, wall-clock time and elected leader identities never enter it, so
//! an independent re-execution by the admitted binary reproduces the same bytes.

mod control_plane;
mod durable;
mod effect;
pub(super) mod peer;
mod transport;
mod world;

pub use peer::Launcher;
pub(crate) use peer::run_peer;
use preserves::ValueImpl as _;

const MAX_CHECKS: usize = 64;
const MAX_RUNTIME_VALUES: usize = 64;
/// The shell production qualification runs its live process effect through.
const PRODUCTION_EFFECT_SHELL: &str = "/bin/sh";

/// The executables product producers start, supplied by their caller.
#[derive(Debug, Clone)]
pub struct Programs {
    /// How a producer starts distinct-process peers of itself.
    pub peer: Launcher,
    /// The shell the effect-uncertainty producer runs through the live bounded-process adapter. The
    /// adapter measures it and starts it through the measured handle path, which becomes its
    /// `argv[0]`, so it must not be a multicall binary that selects its behavior from that name.
    pub effect_shell: std::path::PathBuf,
}

impl Programs {
    /// Peers of the executing binary and the host `/bin/sh`, as production qualification runs them.
    pub fn current() -> crate::error::Result<Self> {
        Ok(Self {
            peer: Launcher::current()?,
            effect_shell: std::path::PathBuf::from(PRODUCTION_EFFECT_SHELL),
        })
    }

    /// Peers through the unit-test harness and a non-multicall shell resolved by test setup.
    #[cfg(test)]
    pub(in crate::capability_qualification) fn test_harness() -> Self {
        Self {
            peer: Launcher::test_harness(),
            effect_shell: crate::fabric_execution::tests::interpreter::non_multicall_shell(),
        }
    }
}

/// Invariant outcomes and canonical runtime receipts observed during one run.
#[derive(Debug, Default)]
pub(super) struct Evidence {
    checks: Vec<(&'static str, bool)>,
    runtime: Vec<preserves::IOValue>,
}

impl Evidence {
    pub(super) fn check(&mut self, name: &'static str, held: bool) {
        self.checks.push((name, held));
    }

    pub(super) fn runtime(&mut self, value: preserves::IOValue) {
        self.runtime.push(value);
    }
}

/// The fixed, subject-independent plan a product scenario executes.
pub(super) fn plan(scenario: super::Scenario) -> preserves::IOValue {
    let definition = scenario.definition();
    crate::preserves_rail::record("capability-product-plan-v1", vec![
        crate::preserves_rail::string(super::model::PRODUCT_PLAN_SCHEMA),
        crate::preserves_rail::string(definition.id),
        crate::preserves_rail::string(definition.capability.id),
        crate::preserves_rail::string(definition.family),
        crate::preserves_rail::string(definition.requirement),
        crate::preserves_rail::string(definition.coverage_kind),
        crate::preserves_rail::string(definition.capability.execution_profile),
        crate::preserves_rail::string(environment_id(definition.capability.environment)),
    ])
}

/// Execute the actual runtime path. An error is a canonical failure, never a pass.
pub(super) fn execute(scenario: super::Scenario, programs: &Programs) -> crate::error::Result<preserves::IOValue> {
    let capability = scenario.definition().capability;
    let evidence = if capability == super::model::DURABLE_STATE_RECOVERY {
        durable::run(scenario)?
    } else if capability == super::model::EFFECT_UNCERTAINTY {
        effect::run(scenario, &programs.effect_shell)?
    } else if capability == super::model::CROSS_PROCESS_TRANSPORT {
        transport::run(scenario, &programs.peer)?
    } else if capability == super::model::WORLD_REPLAY_PROMOTION_RESTORE {
        world::run(scenario)?
    } else if capability == super::model::CONTROL_PLANE_FAILOVER {
        control_plane::run(scenario, &programs.peer)?
    } else {
        return Err(super::invalid("scenario has no native product producer"));
    };
    observation(scenario, &evidence)
}

pub(super) fn observation(scenario: super::Scenario, evidence: &Evidence) -> crate::error::Result<preserves::IOValue> {
    if evidence.checks.is_empty() || evidence.runtime.is_empty() {
        return Err(super::invalid("product producer observed no invariant or runtime receipt"));
    }
    if evidence.checks.len() > MAX_CHECKS || evidence.runtime.len() > MAX_RUNTIME_VALUES {
        return Err(super::invalid("product observation exceeds its fixed bound"));
    }
    let mut names = std::collections::BTreeSet::new();
    if evidence.checks.iter().any(|(name, _)| !names.insert(*name)) {
        return Err(super::invalid("product observation repeats an invariant check"));
    }
    let definition = scenario.definition();
    Ok(crate::preserves_rail::record("capability-product-observation-v1", vec![
        crate::preserves_rail::string(super::model::PRODUCT_OBSERVATION_SCHEMA),
        crate::preserves_rail::string(definition.id),
        crate::preserves_rail::string(definition.requirement),
        crate::preserves_rail::string(definition.coverage_kind),
        crate::preserves_rail::sequence(
            evidence
                .checks
                .iter()
                .map(|(name, held)| {
                    crate::preserves_rail::record("check", vec![
                        crate::preserves_rail::string(name),
                        crate::preserves_rail::bool_value(*held),
                    ])
                })
                .collect(),
        ),
        crate::preserves_rail::sequence(evidence.runtime.clone()),
    ]))
}

/// Structural readback of a product observation: exact scenario, requirement and
/// polarity binding and every recorded invariant held. Identity with an actual
/// run is checked separately against the admitted binary's re-execution.
pub(super) fn verify_observation(scenario: super::Scenario, value: &preserves::IOValue) -> crate::error::Result<()> {
    let fields = value
        .collect_simple_record("capability-product-observation-v1", Some(6))
        .ok_or_else(|| super::invalid("expected native product observation"))?;
    let definition = scenario.definition();
    if super::text(&fields[0])? != super::model::PRODUCT_OBSERVATION_SCHEMA
        || super::text(&fields[1])? != definition.id
        || super::text(&fields[2])? != definition.requirement
        || super::text(&fields[3])? != definition.coverage_kind
    {
        return Err(super::invalid("product observation scenario/requirement/coverage mismatch"));
    }
    let checks = fields[4].collect_sequence().ok_or_else(|| super::invalid("product checks must be a sequence"))?;
    let runtime = fields[5].collect_sequence().ok_or_else(|| super::invalid("product runtime must be a sequence"))?;
    if checks.is_empty() || runtime.is_empty() || checks.len() > MAX_CHECKS || runtime.len() > MAX_RUNTIME_VALUES {
        return Err(super::invalid("product observation lacks bounded checks or runtime receipts"));
    }
    let mut names = std::collections::BTreeSet::new();
    for check in checks.iter() {
        let pair = check
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| super::invalid("malformed product invariant check"))?;
        let name = super::text(&pair[0])?;
        if !names.insert(name) {
            return Err(super::invalid("product observation repeats an invariant check"));
        }
        if pair[1].as_boolean() != Some(true) {
            return Err(super::invalid("product invariant did not hold"));
        }
    }
    Ok(())
}

pub(super) fn environment_id(environment: super::Environment) -> &'static str {
    match environment {
        super::Environment::Simulation => "simulation",
        super::Environment::SameProcess => "same-process",
        super::Environment::DistinctProcess => "distinct-process",
        super::Environment::NixosVm => "nixos-vm",
        super::Environment::CrossHost => "cross-host",
    }
}

/// Canonical class of a runtime error for deterministic observations. Messages
/// carry scratch paths or adapter text and are never part of an observation.
pub(super) fn error_class(error: &crate::error::Failure) -> preserves::IOValue {
    let class = match error {
        crate::error::Failure::Io(_) => "io",
        crate::error::Failure::NodeStateWriteOpenDenied { .. } => "node-state-write-open-denied",
        crate::error::Failure::NodeStatusHealthOpenDenied { .. } => "node-status-health-open-denied",
        crate::error::Failure::Preserves(_) => "preserves",
        crate::error::Failure::InvalidHarness(_) => "invalid-harness",
        crate::error::Failure::HarnessDivergence(_) => "harness-divergence",
        crate::error::Failure::Classified { class, .. } => class.code(),
    };
    crate::preserves_rail::record("runtime-error-class-v1", vec![crate::preserves_rail::string(class)])
}

pub(super) fn scratch(label: &str) -> crate::error::Result<tempfile::TempDir> {
    tempfile::Builder::new()
        .prefix(&format!("aspen-qualification-{label}-"))
        .tempdir()
        .map_err(crate::error::Failure::from)
}
