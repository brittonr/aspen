use super::*;

mod cohort;
mod evidence;
mod projection;
mod reference;
mod safety;

const SERVICE: &str = "replicated-log";

fn content(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn identity() -> OracleIdentity {
    OracleIdentity {
        oracle_id: "molten.reference-protocol-oracle".to_owned(),
        version: 1,
    }
}

/// A profile with two replicas of one service and every safety level.
fn profile() -> ProtocolOracleProfile {
    ProtocolOracleProfile {
        profile_id: "test-protocol-oracle".to_owned(),
        oracle: identity(),
        oracle_ref: oracle_ref_of(&identity()),
        services: vec![ServiceProfile {
            service: SERVICE.to_owned(),
            logical_position: LogicalPositionKind::NextOffset,
            safety: vec![
                SafetyLevel::Local,
                SafetyLevel::Pairwise,
                SafetyLevel::Cohort,
                SafetyLevel::Durability,
            ],
            physical_layout: PhysicalLayout::SemanticOnly,
        }],
        participants: ["node-a", "node-b"]
            .into_iter()
            .map(|participant| ParticipantProfile {
                participant: participant.to_owned(),
                service: SERVICE.to_owned(),
                is_required: true,
            })
            .collect(),
        completeness: CompletenessMode::Lossless,
        liveness: LivenessPreconditions {
            requires_readiness: true,
            requires_membership: true,
            disruptive_faults_block: true,
            requires_durability: true,
            virtual_progress_horizon: 8,
        },
        novelty_fields: vec![
            NoveltyField::Service,
            NoveltyField::LogicalPosition,
            NoveltyField::State,
        ],
        counters: vec![
            CounterSchema {
                name: "transitions".to_owned(),
                resets_on_generation: false,
            },
            CounterSchema {
                name: "repairs".to_owned(),
                resets_on_generation: true,
            },
        ],
        bounds: ProjectionBounds {
            max_records: 64,
            max_state_bytes: 4_096,
            max_total_bytes: 65_536,
            max_participants: 8,
            max_counters: 8,
        },
        non_claims: PROTOCOL_ORACLE_NON_CLAIMS.iter().map(|claim| (*claim).to_owned()).collect(),
    }
}

/// A projection of a log state `<entries next_offset>` for one participant.
fn projection(participant: &str, source_sequence: u64, run_sequence: u64, entries: &[&str]) -> ProtocolProjection {
    let next_offset = u64::try_from(entries.len()).expect("entry count");
    let state = CanonicalValue::record("replicated-log", vec![
        CanonicalValue::unsigned(next_offset),
        CanonicalValue::unsigned(0),
        CanonicalValue::optional(None),
        CanonicalValue::Sequence(
            entries
                .iter()
                .zip(0_u64..)
                .map(|(payload, offset)| {
                    CanonicalValue::record("entry", vec![
                        CanonicalValue::unsigned(offset),
                        CanonicalValue::string(&content(payload)),
                    ])
                })
                .collect(),
        ),
    ]);
    ProtocolProjection {
        protocol: "molten.reference-services".to_owned(),
        service: SERVICE.to_owned(),
        participant: participant.to_owned(),
        generation: 1,
        source_sequence,
        run_sequence,
        scheduler_choice_ref: content(&format!("choice-{run_sequence}")),
        transition_class: TransitionClass::Applied,
        logical_position: next_offset,
        state_ref: state_ref_of(&state),
        state,
        physical_ref: None,
        history_ref: content(&format!("history-{participant}-{source_sequence}")),
        durability: DurabilityFact::Volatile,
        progress: source_sequence,
        completeness: ObservationCompleteness::Complete,
        reported_invariants: vec!["log-offsets-contiguous".to_owned()],
    }
}

/// Semantics with no local rules, for exercising the cross-participant levels.
struct NoLocalRules;

impl ProtocolSemantics for NoLocalRules {
    fn local_rules(&self, _service: &str) -> &'static [&'static str] {
        &[]
    }

    fn check_local(&self, _previous: Option<&ProtocolProjection>, _next: &ProtocolProjection) -> Vec<&'static str> {
        Vec::new()
    }
}

fn verdict(results: &[SafetyResult], level: SafetyLevel) -> OracleVerdict {
    results
        .iter()
        .find(|result| result.level == level)
        .map(|result| result.verdict.clone())
        .expect("level result")
}

// r[verify molten.fabric_simulation.protocol_projection]
#[test]
fn the_reviewed_profile_validates_and_each_broken_field_is_refused() {
    assert!(validate_oracle_profile(&profile()).is_empty());
    let mut broken = profile();
    broken.oracle_ref = content("other-oracle");
    broken.participants.push(broken.participants[0].clone());
    broken.participants[1].service = "unknown-service".to_owned();
    broken.bounds.max_state_bytes = 70_000;
    broken.non_claims.pop();
    let issues = validate_oracle_profile(&broken);
    for expected in [
        ProfileIssue::OracleRefMismatch,
        ProfileIssue::DuplicateParticipant("node-a".to_owned()),
        ProfileIssue::UndeclaredService("unknown-service".to_owned()),
        ProfileIssue::BoundOutOfRange("state-within-total"),
    ] {
        assert!(issues.contains(&expected), "{expected:?} in {issues:?}");
    }
    assert!(issues.iter().any(|issue| matches!(issue, ProfileIssue::MissingNonClaim(_))));
}
