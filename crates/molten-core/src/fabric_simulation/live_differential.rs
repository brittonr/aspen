use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::fabric::FabricPortClass;
use crate::fabric_simulation::framing::FramedDigest;
use crate::fabric_simulation::valid_ref;

pub const MAX_LIVE_WORKLOAD_STEPS: usize = 64;
const MAX_ORDERING_KEY_BYTES: usize = 256;
const WORKLOAD_DOMAIN: &str = "molten.fabric-simulation.live-workload.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutcomeClass {
    Delivered,
    Rejected,
    Backpressured,
    Delayed,
    Unsupported,
}

impl OutcomeClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delivered => "delivered",
            Self::Rejected => "rejected",
            Self::Backpressured => "backpressured",
            Self::Delayed => "delayed",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveWorkloadStep {
    pub sequence: u64,
    pub class: FabricPortClass,
    pub request_ref: String,
    pub permitted: Vec<OutcomeClass>,
    pub ordering_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveWorkload {
    pub workload_ref: String,
    pub steps: Vec<LiveWorkloadStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiveDifferentialIssue {
    EmptyWorkload,
    WorkloadTooLarge { steps: usize, maximum: usize },
    DuplicateSequence(u64),
    MalformedRequestRef(usize),
    EmptyPermittedClasses(usize),
    DuplicatePermittedClass { step: usize, class: OutcomeClass },
    MalformedOrderingKey(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferentialSide {
    Simulated,
    Live,
}

impl DifferentialSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simulated => "simulated",
            Self::Live => "live",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncompleteReason {
    NotExecuted,
    Unsupported,
    AdapterUnavailable,
}

impl IncompleteReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotExecuted => "not-executed",
            Self::Unsupported => "unsupported",
            Self::AdapterUnavailable => "adapter-unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompleteStep {
    pub step: usize,
    pub side: DifferentialSide,
    pub reason: IncompleteReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermittedDivergence {
    pub step: usize,
    pub simulated: OutcomeClass,
    pub live: OutcomeClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderingDivergence {
    pub key: String,
    pub simulated: Vec<OutcomeClass>,
    pub live: Vec<OutcomeClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpermittedOutcome {
    pub step: usize,
    pub simulated: OutcomeClass,
    pub live: OutcomeClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveDifferentialDecision {
    Agreement {
        steps: usize,
    },
    PermittedDivergence {
        divergences: Vec<PermittedDivergence>,
        ordering: Vec<OrderingDivergence>,
    },
    Incomplete {
        incomplete: Vec<IncompleteStep>,
        divergences: Vec<PermittedDivergence>,
    },
    Failure {
        outcome: UnpermittedOutcome,
    },
}

impl LiveDifferentialDecision {
    pub const fn result_kind(&self) -> &'static str {
        match self {
            Self::Agreement { .. } => "agreement",
            Self::PermittedDivergence { .. } => "permitted-divergence",
            Self::Incomplete { .. } => "incomplete",
            Self::Failure { .. } => "failure",
        }
    }
}

/// A bounded executor over declared port-contract requests. One implementation is
/// selected per side; the pure differential never performs I/O itself.
pub trait WorkloadExecutor {
    fn execute_step(&mut self, step: &LiveWorkloadStep) -> Option<OutcomeClass>;
}

pub fn execute_workload<E: WorkloadExecutor>(executor: &mut E, workload: &LiveWorkload) -> Vec<Option<OutcomeClass>> {
    workload.steps.iter().map(|step| executor.execute_step(step)).collect()
}

impl LiveWorkload {
    // r[impl molten.fabric_simulation.live_differential]
    pub fn new(steps: Vec<LiveWorkloadStep>) -> Result<Self, Vec<LiveDifferentialIssue>> {
        let mut issues = Vec::with_capacity(4);
        if steps.is_empty() {
            issues.push(LiveDifferentialIssue::EmptyWorkload);
        }
        if steps.len() > MAX_LIVE_WORKLOAD_STEPS {
            issues.push(LiveDifferentialIssue::WorkloadTooLarge {
                steps: steps.len(),
                maximum: MAX_LIVE_WORKLOAD_STEPS,
            });
        }
        let mut sequences = BTreeSet::new();
        for (index, step) in steps.iter().enumerate() {
            if !sequences.insert(step.sequence) {
                issues.push(LiveDifferentialIssue::DuplicateSequence(step.sequence));
            }
            if !valid_ref(&step.request_ref) {
                issues.push(LiveDifferentialIssue::MalformedRequestRef(index));
            }
            if step.permitted.is_empty() {
                issues.push(LiveDifferentialIssue::EmptyPermittedClasses(index));
            }
            let mut permitted = BTreeSet::new();
            for class in &step.permitted {
                if !permitted.insert(*class) {
                    issues.push(LiveDifferentialIssue::DuplicatePermittedClass {
                        step: index,
                        class: *class,
                    });
                }
            }
            if step.ordering_key.as_deref().is_some_and(|key| !valid_ordering_key(key)) {
                issues.push(LiveDifferentialIssue::MalformedOrderingKey(index));
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        let workload_ref = workload_ref(&steps);
        Ok(Self { workload_ref, steps })
    }
}

// r[impl molten.fabric_simulation.collision_free_workload_and_complete_comparison]
fn valid_ordering_key(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ORDERING_KEY_BYTES && !value.chars().any(char::is_control)
}

// r[impl molten.fabric_simulation.collision_free_workload_and_complete_comparison]
pub(super) fn workload_ref(steps: &[LiveWorkloadStep]) -> String {
    let mut framed = FramedDigest::new(WORKLOAD_DOMAIN);
    framed.count(steps.len());
    for step in steps {
        framed.number(step.sequence);
        framed.text(step.class.as_str());
        framed.text(&step.request_ref);
        framed.count(step.permitted.len());
        for class in &step.permitted {
            framed.text(class.as_str());
        }
        framed.optional_text(step.ordering_key.as_deref());
    }
    framed.content_ref()
}

// r[impl molten.fabric_simulation.differential_results]
pub fn adapter_unavailable(workload: &LiveWorkload) -> LiveDifferentialDecision {
    LiveDifferentialDecision::Incomplete {
        incomplete: workload
            .steps
            .iter()
            .enumerate()
            .map(|(step, _)| IncompleteStep {
                step,
                side: DifferentialSide::Live,
                reason: IncompleteReason::AdapterUnavailable,
            })
            .collect(),
        divergences: Vec::new(),
    }
}

// r[impl molten.fabric_simulation.live_differential]
// r[impl molten.fabric_simulation.differential_results]
pub fn evaluate_live_differential(
    workload: &LiveWorkload,
    simulated: &[Option<OutcomeClass>],
    live: &[Option<OutcomeClass>],
) -> LiveDifferentialDecision {
    let mut incomplete = Vec::new();
    let mut divergences = Vec::new();
    for (step, declared) in workload.steps.iter().enumerate() {
        let simulated_class = simulated.get(step).copied().flatten();
        let live_class = live.get(step).copied().flatten();
        let live_unsupported = live_class == Some(OutcomeClass::Unsupported);
        let simulated_unsupported = simulated_class == Some(OutcomeClass::Unsupported);
        if live_unsupported {
            incomplete.push(IncompleteStep {
                step,
                side: DifferentialSide::Live,
                reason: IncompleteReason::Unsupported,
            });
        }
        if simulated_unsupported {
            incomplete.push(IncompleteStep {
                step,
                side: DifferentialSide::Simulated,
                reason: IncompleteReason::Unsupported,
            });
        }
        if live_unsupported || simulated_unsupported {
            continue;
        }
        let (Some(simulated_class), Some(live_class)) = (simulated_class, live_class) else {
            incomplete.push(IncompleteStep {
                step,
                side: if simulated_class.is_none() {
                    DifferentialSide::Simulated
                } else {
                    DifferentialSide::Live
                },
                reason: if simulated_class.is_none() && live_class.is_none() {
                    IncompleteReason::AdapterUnavailable
                } else {
                    IncompleteReason::NotExecuted
                },
            });
            continue;
        };
        // r[impl molten.fabric_simulation.collision_free_workload_and_complete_comparison]
        if !declared.permitted.contains(&live_class) || !declared.permitted.contains(&simulated_class) {
            return LiveDifferentialDecision::Failure {
                outcome: UnpermittedOutcome {
                    step,
                    simulated: simulated_class,
                    live: live_class,
                },
            };
        }
        if simulated_class != live_class {
            divergences.push(PermittedDivergence {
                step,
                simulated: simulated_class,
                live: live_class,
            });
        }
    }
    if !incomplete.is_empty() {
        return LiveDifferentialDecision::Incomplete {
            incomplete,
            divergences,
        };
    }
    let ordering = ordering_divergences(workload, simulated, live);
    if !divergences.is_empty() || !ordering.is_empty() {
        return LiveDifferentialDecision::PermittedDivergence { divergences, ordering };
    }
    LiveDifferentialDecision::Agreement {
        steps: workload.steps.len(),
    }
}

fn ordering_divergences(
    workload: &LiveWorkload,
    simulated: &[Option<OutcomeClass>],
    live: &[Option<OutcomeClass>],
) -> Vec<OrderingDivergence> {
    let mut keys: BTreeMap<&str, (Vec<OutcomeClass>, Vec<OutcomeClass>)> = BTreeMap::new();
    for (step, declared) in workload.steps.iter().enumerate() {
        let Some(key) = declared.ordering_key.as_deref() else {
            continue;
        };
        let Some(simulated_class) = simulated.get(step).copied().flatten() else {
            continue;
        };
        let Some(live_class) = live.get(step).copied().flatten() else {
            continue;
        };
        let entry = keys.entry(key).or_default();
        entry.0.push(simulated_class);
        entry.1.push(live_class);
    }
    keys.into_iter()
        .filter(|(_, (simulated, live))| simulated != live)
        .map(|(key, (simulated, live))| OrderingDivergence {
            key: key.to_string(),
            simulated,
            live,
        })
        .collect()
}
