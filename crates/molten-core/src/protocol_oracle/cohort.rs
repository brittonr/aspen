use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

/// One admitted projection with its canonical ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedProjection {
    pub projection: ProtocolProjection,
    pub projection_ref: String,
}

/// Why a cohort cannot support a passing result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletenessGap {
    /// A lossless participant skipped a source sequence.
    SequenceGap {
        participant: String,
        expected: u64,
        actual: u64,
    },
    /// A required participant produced no projection.
    MissingParticipant(String),
    /// A participant reported a partial observation.
    PartialObservation { participant: String, source_sequence: u64 },
}

/// Admitted projections, in run order, with their completeness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCohort {
    pub projections: Vec<AdmittedProjection>,
    pub gaps: Vec<CompletenessGap>,
    pub total_bytes: u64,
}

impl ProtocolCohort {
    pub fn is_complete(&self) -> bool {
        self.gaps.is_empty()
    }

    /// The projections of one participant in source order.
    pub fn participant(&self, participant: &str) -> impl Iterator<Item = &AdmittedProjection> {
        self.projections.iter().filter(move |admitted| admitted.projection.participant == participant)
    }
}

/// Why a cohort is malformed; a malformed cohort is never evaluated.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CohortIssue {
    Projection { run_sequence: u64, issue: ProjectionIssue },
    DuplicateSequence { participant: String, source_sequence: u64 },
    DuplicateRunSequence(u64),
    GenerationDrift { participant: String, source_sequence: u64 },
    TooManyRecords,
    TooManyBytes,
}

/// Assemble a cohort from the projections of one run.
///
/// Every projection is admitted under the profile. Duplicate sequences and a
/// participant whose generation moves backwards are malformed. A sequence gap
/// under a lossless profile, a required participant with no projection, or a
/// partial observation leaves the cohort incomplete.
// r[impl molten.fabric_simulation.protocol_projection]
pub fn assemble_cohort(
    profile: &ProtocolOracleProfile,
    projections: Vec<ProtocolProjection>,
) -> Result<ProtocolCohort, Vec<CohortIssue>> {
    let is_bounded = u32::try_from(projections.len()).is_ok_and(|count| count <= profile.bounds.max_records);
    if !is_bounded {
        return Err(vec![CohortIssue::TooManyRecords]);
    }
    let mut issues = BTreeSet::new();
    let mut gaps = BTreeSet::new();
    let mut admitted = Vec::with_capacity(projections.len());
    let mut total_bytes = 0_u64;
    let mut run_sequences = BTreeSet::new();
    let mut last_by_participant = BTreeMap::<String, (u64, u64)>::new();
    let mut ordered = projections;
    ordered.sort_by_key(|projection| projection.run_sequence);
    for projection in ordered {
        match admit_projection(profile, &projection) {
            Ok(bytes) => total_bytes = total_bytes.saturating_add(bytes),
            Err(issue) => {
                issues.insert(CohortIssue::Projection {
                    run_sequence: projection.run_sequence,
                    issue,
                });
                continue;
            }
        }
        if !run_sequences.insert(projection.run_sequence) {
            issues.insert(CohortIssue::DuplicateRunSequence(projection.run_sequence));
        }
        check_order(profile, &projection, &mut last_by_participant, &mut issues, &mut gaps);
        if projection.completeness == ObservationCompleteness::Partial {
            gaps.insert(CompletenessGap::PartialObservation {
                participant: projection.participant.clone(),
                source_sequence: projection.source_sequence,
            });
        }
        let projection_ref = projection_ref(&projection);
        admitted.push(AdmittedProjection {
            projection,
            projection_ref,
        });
    }
    if total_bytes > profile.bounds.max_total_bytes {
        issues.insert(CohortIssue::TooManyBytes);
    }
    for participant in profile.participants.iter().filter(|participant| participant.is_required) {
        if !last_by_participant.contains_key(&participant.participant) {
            gaps.insert(CompletenessGap::MissingParticipant(participant.participant.clone()));
        }
    }
    if !issues.is_empty() {
        return Err(issues.into_iter().collect());
    }
    Ok(ProtocolCohort {
        projections: admitted,
        gaps: gaps.into_iter().collect(),
        total_bytes,
    })
}

fn check_order(
    profile: &ProtocolOracleProfile,
    projection: &ProtocolProjection,
    last_by_participant: &mut BTreeMap<String, (u64, u64)>,
    issues: &mut BTreeSet<CohortIssue>,
    gaps: &mut BTreeSet<CompletenessGap>,
) {
    let expected = match last_by_participant.get(&projection.participant) {
        None => 0,
        Some((last_sequence, last_generation)) => {
            if projection.source_sequence <= *last_sequence {
                issues.insert(CohortIssue::DuplicateSequence {
                    participant: projection.participant.clone(),
                    source_sequence: projection.source_sequence,
                });
                return;
            }
            if projection.generation < *last_generation {
                issues.insert(CohortIssue::GenerationDrift {
                    participant: projection.participant.clone(),
                    source_sequence: projection.source_sequence,
                });
            }
            last_sequence.saturating_add(1)
        }
    };
    if profile.completeness == CompletenessMode::Lossless && projection.source_sequence != expected {
        gaps.insert(CompletenessGap::SequenceGap {
            participant: projection.participant.clone(),
            expected,
            actual: projection.source_sequence,
        });
    }
    last_by_participant.insert(projection.participant.clone(), (projection.source_sequence, projection.generation));
}
