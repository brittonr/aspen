
pub const FABRIC_SIMULATION_PROTOCOL_STATUS_SCHEMA: &str = "molten.fabric-simulation.protocol-status.v1";
const PROTOCOL_STATUS_LABEL: &str = "fabric-simulation-protocol-status-v1";
const PROTOCOL_STATUS_FIELD_COUNT: usize = 14;

/// The bounded operator status of a run's protocol oracle evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolStatus {
    pub evidence_ref: String,
    pub verdict: String,
    pub projections: u64,
    pub gaps: u64,
    pub safety_pass: u64,
    pub safety_fail: u64,
    pub safety_incomplete: u64,
    pub safety_not_evaluated: u64,
    pub liveness_pass: u64,
    pub liveness_fail: u64,
    /// Liveness results that are not evaluated or incomplete.
    pub liveness_open: u64,
    pub novelty: u64,
    /// `<level>/<service>/<rule>@<run-sequence>` of the earliest safety failure.
    pub first_failure: Option<String>,
}

fn count(items: impl Iterator<Item = bool>) -> crate::error::Result<u64> {
    u64::try_from(items.filter(|is_counted| *is_counted).count())
        .map_err(|_| crate::error::Failure::invalid_harness("protocol status count overflow"))
}

/// Summarize protocol oracle evidence for operators.
// r[impl molten.fabric_simulation.protocol_oracle_evidence]
pub fn protocol_status(
    evidence: &molten_core::protocol_oracle::ProtocolOracleEvidence,
    evidence_ref: &str,
) -> crate::error::Result<ProtocolStatus> {
    crate::preserves_rail::validate_content_ref(evidence_ref)?;
    let verdict = match molten_core::protocol_oracle::evidence_verdict(evidence) {
        molten_core::protocol_oracle::EvidenceVerdict::Pass => "pass",
        molten_core::protocol_oracle::EvidenceVerdict::Fail => "fail",
        molten_core::protocol_oracle::EvidenceVerdict::Incomplete => "incomplete",
    };
    let first_failure = evidence
        .safety
        .iter()
        .filter_map(|result| match &result.verdict {
            molten_core::protocol_oracle::OracleVerdict::Fail(witness) => Some((witness.run_sequence, result)),
            _ => None,
        })
        .min_by_key(|(run_sequence, _)| *run_sequence)
        .map(|(run_sequence, result)| {
            format!("{}/{}/{}@{run_sequence}", result.level.as_str(), result.service, result.rule)
        });
    let safety = |expected: fn(&molten_core::protocol_oracle::OracleVerdict) -> bool| count(evidence.safety.iter().map(|result| expected(&result.verdict)));
    Ok(ProtocolStatus {
        evidence_ref: evidence_ref.to_string(),
        verdict: verdict.to_string(),
        projections: count(evidence.projection_refs.iter().map(|_| true))?,
        gaps: count(evidence.gaps.iter().map(|_| true))?,
        safety_pass: safety(|verdict| *verdict == molten_core::protocol_oracle::OracleVerdict::Pass)?,
        safety_fail: safety(|verdict| matches!(verdict, molten_core::protocol_oracle::OracleVerdict::Fail(_)))?,
        safety_incomplete: safety(|verdict| *verdict == molten_core::protocol_oracle::OracleVerdict::Incomplete)?,
        safety_not_evaluated: safety(|verdict| *verdict == molten_core::protocol_oracle::OracleVerdict::NotEvaluated)?,
        liveness_pass: count(evidence.liveness.iter().map(|result| result.verdict == molten_core::protocol_oracle::LivenessVerdict::Pass))?,
        liveness_fail: count(evidence.liveness.iter().map(|result| result.verdict == molten_core::protocol_oracle::LivenessVerdict::Fail))?,
        liveness_open: count(evidence.liveness.iter().map(|result| {
            matches!(result.verdict, molten_core::protocol_oracle::LivenessVerdict::NotEvaluated(_) | molten_core::protocol_oracle::LivenessVerdict::Incomplete(_))
        }))?,
        novelty: count(evidence.novelty_refs.iter().map(|_| true))?,
        first_failure,
    })
}

fn protocol_status_value(status: &ProtocolStatus) -> preserves::IOValue {
    crate::preserves_rail::record(PROTOCOL_STATUS_LABEL, vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_PROTOCOL_STATUS_SCHEMA),
        field("evidence-ref", crate::preserves_rail::string(&status.evidence_ref)),
        field("verdict", crate::preserves_rail::string(&status.verdict)),
        field("projections", crate::preserves_rail::u64_value(status.projections)),
        field("gaps", crate::preserves_rail::u64_value(status.gaps)),
        field("safety-pass", crate::preserves_rail::u64_value(status.safety_pass)),
        field("safety-fail", crate::preserves_rail::u64_value(status.safety_fail)),
        field("safety-incomplete", crate::preserves_rail::u64_value(status.safety_incomplete)),
        field("safety-not-evaluated", crate::preserves_rail::u64_value(status.safety_not_evaluated)),
        field("liveness-pass", crate::preserves_rail::u64_value(status.liveness_pass)),
        field("liveness-fail", crate::preserves_rail::u64_value(status.liveness_fail)),
        field("liveness-open", crate::preserves_rail::u64_value(status.liveness_open)),
        field("novelty", crate::preserves_rail::u64_value(status.novelty)),
        field("first-failure", optional_string(status.first_failure.as_deref())),
    ])
}

fn parse_protocol_status(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<ProtocolStatus> {
    let status = named_field_value(value, "protocol")?;
    let fields = status
        .collect_simple_record(PROTOCOL_STATUS_LABEL, Some(PROTOCOL_STATUS_FIELD_COUNT))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected canonical protocol status"))?;
    if required_string(&fields[0], "protocol status schema")? != FABRIC_SIMULATION_PROTOCOL_STATUS_SCHEMA {
        return Err(crate::error::Failure::invalid_harness("protocol status schema mismatch"));
    }
    let evidence_ref = record_string_field(&fields[1], "evidence-ref")?;
    crate::preserves_rail::validate_content_ref(&evidence_ref)?;
    let verdict = record_string_field(&fields[2], "verdict")?;
    if !matches!(verdict.as_str(), "pass" | "fail" | "incomplete") {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported protocol verdict: {verdict}")));
    }
    let failure = named_field_value(&fields[13], "first-failure")?;
    let first_failure = if failure.collect_simple_record("none", Some(0)).is_some() {
        None
    } else {
        let some = failure
            .collect_simple_record("some", Some(1))
            .ok_or_else(|| crate::error::Failure::invalid_harness("expected optional protocol failure"))?;
        Some(required_string(&some[0], "first-failure")?)
    };
    Ok(ProtocolStatus {
        evidence_ref,
        verdict,
        projections: record_u64_field(&fields[3], "projections")?,
        gaps: record_u64_field(&fields[4], "gaps")?,
        safety_pass: record_u64_field(&fields[5], "safety-pass")?,
        safety_fail: record_u64_field(&fields[6], "safety-fail")?,
        safety_incomplete: record_u64_field(&fields[7], "safety-incomplete")?,
        safety_not_evaluated: record_u64_field(&fields[8], "safety-not-evaluated")?,
        liveness_pass: record_u64_field(&fields[9], "liveness-pass")?,
        liveness_fail: record_u64_field(&fields[10], "liveness-fail")?,
        liveness_open: record_u64_field(&fields[11], "liveness-open")?,
        novelty: record_u64_field(&fields[12], "novelty")?,
        first_failure,
    })
}

/// Everything one canonical run record binds.
pub struct RunInput<'a> {
    pub world_ref: &'a str,
    pub profile: SimulationClaimProfile,
    pub summary: SimulationRunSummary,
    pub observation_refs: Vec<String>,
    pub port_event_refs: Vec<String>,
    pub protocol: &'a ProtocolStatus,
}
