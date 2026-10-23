use super::*;

/// The identities a protocol oracle run binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolRunIdentity {
    pub world_ref: String,
    pub runtime_ref: String,
    pub scheduler_ref: String,
    pub workload_ref: String,
    pub fault_ref: String,
    pub replay_ref: String,
}

/// The evidence of one protocol-aware run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolOracleEvidence {
    pub run: ProtocolRunIdentity,
    pub profile_id: String,
    pub projection_schema: String,
    pub oracle_ref: String,
    pub participants: Vec<String>,
    pub projection_refs: Vec<String>,
    pub gaps: Vec<CompletenessGap>,
    pub safety: Vec<SafetyResult>,
    pub liveness: Vec<LivenessResult>,
    pub novelty_refs: Vec<String>,
    pub counters: CounterTotals,
    pub non_claims: Vec<String>,
}

/// The overall reading of a run's evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceVerdict {
    Pass,
    Fail,
    /// Observations or preconditions are missing; never a pass.
    Incomplete,
}

/// The overall verdict: any failure fails; anything incomplete or not
/// evaluated keeps the run incomplete; only a complete cohort with every
/// result passing passes.
// r[impl molten.fabric_simulation.protocol_oracle_evidence]
pub fn evidence_verdict(evidence: &ProtocolOracleEvidence) -> EvidenceVerdict {
    let is_failed = evidence.safety.iter().any(|result| matches!(result.verdict, OracleVerdict::Fail(_)))
        || evidence.liveness.iter().any(|result| result.verdict == LivenessVerdict::Fail);
    if is_failed {
        return EvidenceVerdict::Fail;
    }
    let is_incomplete = !evidence.gaps.is_empty()
        || evidence.safety.iter().any(|result| result.verdict == OracleVerdict::Incomplete)
        || evidence.liveness.iter().any(|result| matches!(result.verdict, LivenessVerdict::Incomplete(_)));
    if is_incomplete {
        EvidenceVerdict::Incomplete
    } else {
        EvidenceVerdict::Pass
    }
}

fn verdict_value(verdict: &OracleVerdict) -> CanonicalValue {
    match verdict {
        OracleVerdict::Pass => CanonicalValue::symbol("pass"),
        OracleVerdict::Incomplete => CanonicalValue::symbol("incomplete"),
        OracleVerdict::NotEvaluated => CanonicalValue::symbol("not-evaluated"),
        OracleVerdict::Fail(witness) => CanonicalValue::record("fail", vec![
            CanonicalValue::unsigned(witness.run_sequence),
            CanonicalValue::string(&witness.participant),
            CanonicalValue::string(&witness.scheduler_choice_ref),
            CanonicalValue::string(&witness.projection_ref),
            CanonicalValue::optional(witness.other_projection_ref.as_deref().map(CanonicalValue::string)),
        ]),
    }
}

fn liveness_value(result: &LivenessResult) -> CanonicalValue {
    let verdict = match result.verdict {
        LivenessVerdict::Pass => CanonicalValue::symbol("pass"),
        LivenessVerdict::Fail => CanonicalValue::symbol("fail"),
        LivenessVerdict::NotEvaluated(condition) => {
            CanonicalValue::record("not-evaluated", vec![CanonicalValue::symbol(condition.as_str())])
        }
        LivenessVerdict::Incomplete(condition) => {
            CanonicalValue::record("incomplete", vec![CanonicalValue::symbol(condition.as_str())])
        }
    };
    CanonicalValue::record("liveness", vec![CanonicalValue::string(&result.participant), verdict])
}

fn gap_value(gap: &CompletenessGap) -> CanonicalValue {
    match gap {
        CompletenessGap::SequenceGap {
            participant,
            expected,
            actual,
        } => CanonicalValue::record("sequence-gap", vec![
            CanonicalValue::string(participant),
            CanonicalValue::unsigned(*expected),
            CanonicalValue::unsigned(*actual),
        ]),
        CompletenessGap::MissingParticipant(participant) => {
            CanonicalValue::record("missing-participant", vec![CanonicalValue::string(participant)])
        }
        CompletenessGap::PartialObservation {
            participant,
            source_sequence,
        } => CanonicalValue::record("partial-observation", vec![
            CanonicalValue::string(participant),
            CanonicalValue::unsigned(*source_sequence),
        ]),
    }
}

fn strings(values: &[String]) -> CanonicalValue {
    CanonicalValue::Sequence(values.iter().map(|value| CanonicalValue::string(value)).collect())
}

/// The canonical value of an evidence bundle.
pub fn evidence_value(evidence: &ProtocolOracleEvidence) -> CanonicalValue {
    let run = &evidence.run;
    CanonicalValue::record("protocol-oracle-evidence", vec![
        strings(&[
            run.world_ref.clone(),
            run.runtime_ref.clone(),
            run.scheduler_ref.clone(),
            run.workload_ref.clone(),
            run.fault_ref.clone(),
            run.replay_ref.clone(),
        ]),
        CanonicalValue::string(&evidence.profile_id),
        CanonicalValue::string(&evidence.projection_schema),
        CanonicalValue::string(&evidence.oracle_ref),
        strings(&evidence.participants),
        strings(&evidence.projection_refs),
        CanonicalValue::Sequence(evidence.gaps.iter().map(gap_value).collect()),
        CanonicalValue::Sequence(
            evidence
                .safety
                .iter()
                .map(|result| {
                    CanonicalValue::record("safety", vec![
                        CanonicalValue::symbol(result.level.as_str()),
                        CanonicalValue::symbol(&result.service),
                        CanonicalValue::string(&result.rule),
                        verdict_value(&result.verdict),
                    ])
                })
                .collect(),
        ),
        CanonicalValue::Sequence(evidence.liveness.iter().map(liveness_value).collect()),
        strings(&evidence.novelty_refs),
        CanonicalValue::Sequence(
            evidence
                .counters
                .iter()
                .map(|(name, value)| {
                    CanonicalValue::record("counter", vec![
                        CanonicalValue::string(name),
                        CanonicalValue::unsigned(*value),
                    ])
                })
                .collect(),
        ),
        strings(&evidence.non_claims),
    ])
}

/// The domain-separated canonical ref of an evidence bundle.
pub fn evidence_ref(evidence: &ProtocolOracleEvidence) -> String {
    domain_ref(EVIDENCE_DOMAIN, &encode_canonical(&evidence_value(evidence)))
}

/// A claim label someone wants to attach to protocol oracle evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolClaim {
    DeterministicSimulation,
    Kvm,
    Live,
    Production,
    Release,
    HardwarePerformance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolClaimIssue {
    /// Simulation evidence cannot satisfy a stronger claim on its own.
    RequiresProfileEvidence(ProtocolClaim),
    NotPassing(EvidenceVerdict),
    MalformedRef(&'static str),
    MissingNonClaim(&'static str),
}

/// Admit a claim over protocol oracle evidence.
///
/// Only a deterministic-simulation claim is admissible, and only when every
/// identity is canonical, the non-claims are present, and the verdict passes.
/// KVM, live, production, release, and hardware-performance claims need their
/// own profile evidence and are always refused here.
// r[impl molten.fabric_simulation.protocol_oracle_evidence]
pub fn admit_protocol_claim(evidence: &ProtocolOracleEvidence, claim: ProtocolClaim) -> Result<(), ProtocolClaimIssue> {
    if claim != ProtocolClaim::DeterministicSimulation {
        return Err(ProtocolClaimIssue::RequiresProfileEvidence(claim));
    }
    let run = &evidence.run;
    for (field, reference) in [
        ("world", &run.world_ref),
        ("runtime", &run.runtime_ref),
        ("scheduler", &run.scheduler_ref),
        ("workload", &run.workload_ref),
        ("faults", &run.fault_ref),
        ("replay", &run.replay_ref),
        ("oracle", &evidence.oracle_ref),
    ] {
        if !is_content_ref(reference) {
            return Err(ProtocolClaimIssue::MalformedRef(field));
        }
    }
    if let Some(missing) = PROTOCOL_ORACLE_NON_CLAIMS
        .iter()
        .find(|required| !evidence.non_claims.iter().any(|stated| stated == **required))
    {
        return Err(ProtocolClaimIssue::MissingNonClaim(missing));
    }
    match evidence_verdict(evidence) {
        EvidenceVerdict::Pass => Ok(()),
        other => Err(ProtocolClaimIssue::NotPassing(other)),
    }
}
