//! Dev-only trace replay against the real fast-path pure functions.
use quint_connect::Driver;
use quint_connect::State;
use serde::de::DeserializeOwned;

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
struct Projection {
    phase: String,
    acceleration_view: u64,
    base_view: u64,
    last_normal_view: u64,
    accepted_commands: std::collections::BTreeSet<String>,
    recovery_set: std::collections::BTreeSet<String>,
    has_marker: bool,
    applied_commands: std::collections::BTreeSet<String>,
    replied_commands: std::collections::BTreeSet<String>,
    commit_order: Vec<String>,
    apply_order: Vec<String>,
    last_decision: String,
    last_issue: String,
    last_conflict_free: bool,
    application_events: u64,
    reply_events: u64,
    duplicate_suppressed: bool,
}

struct FastPathDriver {
    recovery: molten::fabric_consistency::fastpath::RecoveryState,
    ledger: molten::fabric_consistency::fastpath::ApplicationLedger,
    committed: Vec<String>,
    applied: Vec<String>,
    replied: std::collections::BTreeSet<String>,
    decision: String,
    issue: String,
    conflict_free: bool,
    application_events: u64,
    reply_events: u64,
    duplicate_suppressed: bool,
}

impl FastPathDriver {
    fn empty() -> Self {
        Self {
            recovery: molten::fabric_consistency::fastpath::RecoveryState {
                acceleration_view: 1,
                base_view: 1,
                last_normal_view: 1,
                phase: molten::fabric_consistency::fastpath::RecoveryPhase::Normal,
                accepted_commands: std::collections::BTreeSet::new(),
                recovery_set: std::collections::BTreeSet::new(),
                marker_ref: None,
            },
            ledger: molten::fabric_consistency::fastpath::ApplicationLedger::new(),
            committed: Vec::new(),
            applied: Vec::new(),
            replied: std::collections::BTreeSet::new(),
            decision: "None".to_owned(),
            issue: String::new(),
            conflict_free: true,
            application_events: 0,
            reply_events: 0,
            duplicate_suppressed: false,
        }
    }

    fn require_quint() -> quint_connect::Result {
        let output = std::process::Command::new("quint").arg("--version").output()?;
        if !output.status.success() {
            return Err(std::io::Error::other("required pinned Quint version command failed").into());
        }
        let version = String::from_utf8(output.stdout)?;
        if version.split_whitespace().last() != Some("0.33.1") {
            return Err(std::io::Error::other("Quint cohort drift: required version 0.33.1").into());
        }
        Ok(())
    }

    fn recover(&mut self, action: molten::fabric_consistency::fastpath::RecoveryAction) {
        match molten::fabric_consistency::fastpath::transition_recovery(&self.recovery, &action) {
            Ok(next) => {
                self.recovery = next;
                self.issue.clear();
            }
            Err(issue) => self.issue = format!("{issue:?}"),
        }
    }

    fn fast_commit(&mut self, command: String, ack_mode: String, promise_mode: String, conflict_free: bool) {
        let attempt = attempt(&command, &ack_mode, &promise_mode, conflict_free, &self.recovery);
        // The derived five-replica quorums: superquorum 4, so a superquorum can omit the proposer.
        let quorums = molten::fabric_consistency::fastpath::derive_quorums(5).expect("five-replica quorums");
        let decision = molten::fabric_consistency::fastpath::evaluate_stable_view(&attempt, quorums);
        self.conflict_free = conflict_free;
        self.decision = match decision {
            molten::fabric_consistency::fastpath::StableViewDecision::FastCommitted => {
                self.recovery.accepted_commands.insert(command.clone());
                if !self.committed.contains(&command) {
                    self.committed.push(command);
                }
                "FastCommitted".to_owned()
            }
            molten::fabric_consistency::fastpath::StableViewDecision::OriginalOnly => "OriginalOnly".to_owned(),
            molten::fabric_consistency::fastpath::StableViewDecision::Fallback(reason) => format!("{reason:?}"),
        };
    }

    fn converge(&mut self, command: String) -> quint_connect::Result {
        // Each model command is one session operation, so the ledger never sees a conflicting retry.
        let outcome = self
            .ledger
            .converge(&operation(&command))
            .map_err(|issue| std::io::Error::other(molten::fabric_consistency::fastpath::failure(issue)))?;
        if outcome.is_applied {
            self.applied.push(command.clone());
            self.application_events += 1;
        }
        if outcome.is_replied {
            self.replied.insert(command);
            self.reply_events += 1;
        }
        self.duplicate_suppressed = outcome.is_duplicate_suppressed;
        Ok(())
    }
}

fn pick<T: DeserializeOwned>(step: &quint_connect::Step, name: &str) -> quint_connect::Result<T> {
    let value = step
        .nondet_picks
        .get(name)
        .ok_or_else(|| std::io::Error::other(format!("missing Quint input {name}")))?;
    Ok(T::deserialize(value.clone())?)
}

impl Driver for FastPathDriver {
    type State = Projection;

    fn config() -> quint_connect::Config {
        quint_connect::Config {
            state: &["state"],
            nondet: &["action_taken"],
        }
    }

    fn step(&mut self, step: &quint_connect::Step) -> quint_connect::Result {
        match step.action_taken.as_str() {
            "Init" => {
                Self::require_quint()?;
                *self = Self::empty();
            }
            "Accept" => {
                self.recovery.accepted_commands.insert(pick(step, "command")?);
            }
            "BeginViewChange" => self.recover(molten::fabric_consistency::fastpath::RecoveryAction::BeginViewChange {
                new_base_view: pick(step, "new_view")?,
            }),
            "AgreeRecoverySet" => {
                self.recover(molten::fabric_consistency::fastpath::RecoveryAction::AgreeRecoverySet {
                    commands: pick(step, "commands")?,
                })
            }
            "CommitRecoveryMarker" => {
                let is_nonempty: bool = pick(step, "nonempty")?;
                self.recover(molten::fabric_consistency::fastpath::RecoveryAction::CommitRecoveryMarker {
                    marker_ref: if is_nonempty {
                        "model-marker".to_owned()
                    } else {
                        String::new()
                    },
                });
            }
            "ResumeNormalView" => {
                self.recover(molten::fabric_consistency::fastpath::RecoveryAction::ResumeNormalView {
                    new_acceleration_view: pick(step, "new_view")?,
                })
            }
            "FastCommit" => self.fast_commit(
                pick(step, "command")?,
                pick(step, "ack_mode")?,
                pick(step, "promise_mode")?,
                pick(step, "conflict_free")?,
            ),
            "Converge" => self.converge(pick(step, "command")?)?,
            other => return Err(std::io::Error::other(format!("unknown Quint action {other}")).into()),
        }
        Ok(())
    }
}

// r[verify molten.consensus.fast_path_model.validation]
impl State<FastPathDriver> for Projection {
    fn from_driver(driver: &FastPathDriver) -> quint_connect::Result<Self> {
        // Identities/order below come only from real admission/convergence outcomes.
        // The model supplies the caller's ordered-convergence precondition; the
        // ledger itself does not implement a consensus ordering engine.
        assert_eq!(driver.ledger.applied_operations().count(), driver.applied.len());
        Ok(Self {
            phase: format!("{:?}", driver.recovery.phase),
            acceleration_view: driver.recovery.acceleration_view,
            base_view: driver.recovery.base_view,
            last_normal_view: driver.recovery.last_normal_view,
            accepted_commands: driver.recovery.accepted_commands.clone(),
            recovery_set: driver.recovery.recovery_set.clone(),
            has_marker: driver.recovery.marker_ref.is_some(),
            applied_commands: driver.applied.iter().cloned().collect(),
            replied_commands: driver.replied.clone(),
            commit_order: driver.committed.clone(),
            apply_order: driver.applied.clone(),
            last_decision: driver.decision.clone(),
            last_issue: driver.issue.clone(),
            last_conflict_free: driver.conflict_free,
            application_events: driver.application_events,
            reply_events: driver.reply_events,
            duplicate_suppressed: driver.duplicate_suppressed,
        })
    }
}

include!("parts/fastpathquint/p000/body.rs");

// r[verify molten.consensus.fast_path_model.stable_view]
#[quint_connect::quint_test(
    spec = "verification/tla/ConsensusFastPath.qnt",
    test = "mixedViewGuardTest",
    max_samples = 1,
    seed = "5"
)]
fn mixed_view_guard_replay() -> impl Driver {
    FastPathDriver::empty()
}

// r[verify molten.consensus.fast_path_model.stable_view]
#[quint_connect::quint_test(
    spec = "verification/tla/ConsensusFastPath.qnt",
    test = "proposerAcknowledgementTest",
    max_samples = 1,
    seed = "5"
)]
fn proposer_acknowledgement_replay() -> impl Driver {
    FastPathDriver::empty()
}

// r[verify molten.consensus.fast_path_model.view_change_recovery]
// r[verify molten.consensus.fast_path_model.fallback_identity]
#[quint_connect::quint_test(
    spec = "verification/tla/ConsensusFastPath.qnt",
    test = "recoveryAndDuplicateTest",
    max_samples = 1,
    seed = "5"
)]
fn recovery_and_duplicate_replay() -> impl Driver {
    FastPathDriver::empty()
}

// r[verify molten.consensus.fast_path_model.view_change_recovery]
#[quint_connect::quint_test(
    spec = "verification/tla/ConsensusFastPath.qnt",
    test = "emptyRecoveryTest",
    max_samples = 1,
    seed = "5"
)]
fn empty_recovery_replay() -> impl Driver {
    FastPathDriver::empty()
}

// r[verify molten.consensus.fast_path_model.validation]
#[quint_connect::quint_run(
    spec = "verification/tla/ConsensusFastPath.qnt",
    max_samples = 64,
    max_steps = 48,
    seed = "5"
)]
fn bounded_transition_replay() -> impl Driver {
    FastPathDriver::empty()
}
