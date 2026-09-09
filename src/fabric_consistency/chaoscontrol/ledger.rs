use std::collections::BTreeMap;

use super::ChaosControlChainObservation;
use super::ChaosControlObservationMode;
use crate::error::MoltenError;
use crate::error::Result;

// Observer-side accounting over a possibly lossy observation stream. Chain
// and state conflicts at one index are recorded as safety violations and
// remain failed regardless of later observations; missing observations are
// accounted as dropped events, which block accepted conformance as an
// observer failure without relabeling the gap as a consensus-safety failure.
// Continuous cross-replica safety evaluation itself lands with the later
// fault-campaign phase; this ledger only accounts one bounded stream.

pub const MAX_LEDGER_REPLICAS: usize = 64;
pub const MAX_LEDGER_DROPPED_EVENTS: u64 = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosControlSafetyViolationClass {
    ReplicaHistoryConflict,
    ChainLinkConflict,
}

impl ChaosControlSafetyViolationClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReplicaHistoryConflict => "replica-history-conflict",
            Self::ChainLinkConflict => "chain-link-conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosControlSafetyViolation {
    pub class: ChaosControlSafetyViolationClass,
    pub replica_ref: String,
    pub command_index: u64,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaosControlConformanceVerdict {
    Ready,
    BlockedByObserverGap {
        dropped_events: u64,
    },
    RejectedSafety {
        violations: Vec<ChaosControlSafetyViolation>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosControlIngestStatus {
    Appended,
    DuplicateSuppressed,
    ViolationRecorded,
}

#[derive(Debug, Default)]
struct ReplicaHistory {
    digests: BTreeMap<u64, String>,
    state_refs: BTreeMap<u64, String>,
    highest_index: Option<u64>,
}

impl ReplicaHistory {
    fn missing_count(&self) -> u64 {
        let Some(highest) = self.highest_index else {
            return 0;
        };
        highest.saturating_sub(self.digests.len() as u64)
    }
}

#[derive(Debug)]
pub struct ChaosControlObservationLedger {
    observation_mode: ChaosControlObservationMode,
    max_replicas: usize,
    max_dropped_events: u64,
    replicas: BTreeMap<String, ReplicaHistory>,
    violations: Vec<ChaosControlSafetyViolation>,
    duplicates_suppressed: u64,
}

impl ChaosControlObservationLedger {
    pub fn new(observation_mode: ChaosControlObservationMode) -> Self {
        Self {
            observation_mode,
            max_replicas: MAX_LEDGER_REPLICAS,
            max_dropped_events: MAX_LEDGER_DROPPED_EVENTS,
            replicas: BTreeMap::new(),
            violations: Vec::new(),
            duplicates_suppressed: 0,
        }
    }

    pub const fn duplicates_suppressed(&self) -> u64 {
        self.duplicates_suppressed
    }

    pub fn violations(&self) -> &[ChaosControlSafetyViolation] {
        &self.violations
    }

    // Ingests one projected observation. A re-report of an already observed
    // index is suppressed only when the digest and state ref match; any other
    // change is a recorded safety violation that later observations cannot
    // clear. Linkage is checked against whatever neighbors are already
    // observed, so out-of-order arrival is still link-checked once both sides
    // are present.
    pub fn ingest(&mut self, observation: &ChaosControlChainObservation) -> Result<ChaosControlIngestStatus> {
        crate::preserves_rail::validate_content_ref(&observation.prior_digest)?;
        crate::preserves_rail::validate_content_ref(&observation.next_digest)?;
        if !self.replicas.contains_key(&observation.replica_ref) && self.replicas.len() >= self.max_replicas {
            return Err(MoltenError::invalid_harness("ChaosControl observation ledger replica bound exceeded"));
        }
        let current_dropped = self.dropped_total();
        let history = self.replicas.entry(observation.replica_ref.clone()).or_default();
        if let Some(observed_digest) = history.digests.get(&observation.command_index) {
            if observed_digest == &observation.next_digest
                && history.state_refs.get(&observation.command_index) == Some(&observation.application_state_ref)
            {
                self.duplicates_suppressed += 1;
                return Ok(ChaosControlIngestStatus::DuplicateSuppressed);
            }
            self.record_violation(
                ChaosControlSafetyViolationClass::ReplicaHistoryConflict,
                &observation.replica_ref,
                observation.command_index,
                "replica changed an observed chain digest or state ref",
            );
            return Ok(ChaosControlIngestStatus::ViolationRecorded);
        }
        let predecessor_conflict = history
            .digests
            .get(&observation.command_index.saturating_sub(1))
            .filter(|observed| observation.command_index > 1 && *observed != &observation.prior_digest)
            .is_some();
        let successor_conflict = history
            .digests
            .get(&(observation.command_index + 1))
            .filter(|successor_prior| **successor_prior != observation.next_digest)
            .is_some();
        if predecessor_conflict || successor_conflict {
            history.digests.insert(observation.command_index, observation.next_digest.clone());
            history.state_refs.insert(observation.command_index, observation.application_state_ref.clone());
            history.highest_index = history.highest_index.max(Some(observation.command_index));
            self.record_violation(
                ChaosControlSafetyViolationClass::ChainLinkConflict,
                &observation.replica_ref,
                observation.command_index,
                "observation does not link to its observed neighbor digests",
            );
            return Ok(ChaosControlIngestStatus::ViolationRecorded);
        }
        if let Some(highest) = history.highest_index {
            if observation.command_index > highest + 1 {
                let gap = observation.command_index - (highest + 1);
                let next_dropped = current_dropped.saturating_add(gap);
                if next_dropped > self.max_dropped_events {
                    return Err(MoltenError::invalid_harness("ChaosControl dropped-event accounting bound exceeded"));
                }
            }
        }
        history.digests.insert(observation.command_index, observation.next_digest.clone());
        history.state_refs.insert(observation.command_index, observation.application_state_ref.clone());
        history.highest_index = history.highest_index.max(Some(observation.command_index));
        Ok(ChaosControlIngestStatus::Appended)
    }

    // Accepted conformance is ready only when no safety violation was
    // recorded and, under lossless mode, every observed index is present. A
    // gap blocks conformance as an observer failure; it never becomes a
    // safety rejection.
    pub fn conformance_verdict(&self) -> ChaosControlConformanceVerdict {
        if !self.violations.is_empty() {
            return ChaosControlConformanceVerdict::RejectedSafety {
                violations: self.violations.clone(),
            };
        }
        if self.observation_mode == ChaosControlObservationMode::Lossless {
            let missing = self.replicas.values().map(ReplicaHistory::missing_count).sum();
            if missing > 0 {
                return ChaosControlConformanceVerdict::BlockedByObserverGap {
                    dropped_events: missing,
                };
            }
        }
        ChaosControlConformanceVerdict::Ready
    }

    fn dropped_total(&self) -> u64 {
        self.replicas.values().map(ReplicaHistory::missing_count).sum()
    }

    fn record_violation(
        &mut self,
        class: ChaosControlSafetyViolationClass,
        replica_ref: &str,
        command_index: u64,
        detail: &str,
    ) {
        self.violations.push(ChaosControlSafetyViolation {
            class,
            replica_ref: replica_ref.to_string(),
            command_index,
            detail: detail.to_string(),
        });
    }
}
