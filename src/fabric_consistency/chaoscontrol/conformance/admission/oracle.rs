pub(super) struct Evaluator<'a> {
    pub expected: &'a super::ExpectedCohort,
    pub bundle: &'a super::Bundle,
    pub history: &'a chaoscontrol_smr::HistoryReport,
    pub facts: &'a super::checks::Facts,
    pub liveness: &'a chaoscontrol_smr::LivenessReport,
}

impl Evaluator<'_> {
    pub(super) fn evaluate(
        &self,
        cohort: &chaoscontrol_smr::observation::CohortResult,
    ) -> Result<chaoscontrol_smr::observation::OracleResult, super::ImportIssue> {
        chaoscontrol_smr::observation::evaluate_oracle(&self.expected.protocol, cohort, self)
            .map_err(|_| super::ImportIssue::Oracle)
    }

    fn not_evaluated_reason(&self) -> Option<&'static str> {
        if self.bundle.replay.as_ref().is_some_and(|r| !r.comparison.accepted) {
            Some("semantic-replay-mismatch")
        } else if self.bundle.terminal_status != chaoscontrol_smr::TerminalStatus::Completed {
            Some("terminal run is not completed")
        } else if self.liveness.verdict == chaoscontrol_smr::LivenessVerdict::NotEvaluated {
            Some("conditional liveness was not evaluated")
        } else {
            None
        }
    }
}

impl chaoscontrol_smr::observation::ProtocolOracle for Evaluator<'_> {
    fn identity(&self) -> &str {
        &self.expected.protocol.independent_oracle_ref
    }
    fn evaluate(
        &self,
        cohort: &chaoscontrol_smr::observation::CohortResult,
    ) -> chaoscontrol_smr::SmrResult<chaoscontrol_smr::observation::OracleResult> {
        let mut diagnostics = self.facts.failures.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let is_safety_failure = self
            .history
            .violations
            .iter()
            .any(|v| v.class != chaoscontrol_smr::SafetyViolationClass::LosslessGap);
        if is_safety_failure {
            diagnostics.push("producer-safety-failure".into());
        }
        if self.liveness.verdict == chaoscontrol_smr::LivenessVerdict::Fail {
            diagnostics.push("bounded-liveness-failure".into());
        }
        let verdict = if !diagnostics.is_empty() {
            chaoscontrol_smr::observation::OracleVerdict::Fail
        } else if self.facts.is_incomplete {
            diagnostics.push("retained committed history is incomplete".into());
            chaoscontrol_smr::observation::OracleVerdict::Incomplete
        } else if let Some(diagnostic) = self.not_evaluated_reason() {
            diagnostics.push(diagnostic.into());
            chaoscontrol_smr::observation::OracleVerdict::NotEvaluated
        } else {
            chaoscontrol_smr::observation::OracleVerdict::Pass
        };
        Ok(chaoscontrol_smr::observation::OracleResult {
            schema: chaoscontrol_smr::observation::ORACLE_RESULT_SCHEMA.into(),
            oracle_ref: self.expected.protocol.independent_oracle_ref.clone(),
            cohort_ref: cohort.cohort_ref.clone(),
            verdict,
            diagnostics,
        })
    }
}

pub(super) fn status(
    bundle: &super::Bundle,
    oracle: &chaoscontrol_smr::observation::OracleResult,
    liveness: &chaoscontrol_smr::LivenessReport,
) -> &'static str {
    // Keep independently detected failures visible even when missing observations
    // prevent the producer's complete-cohort oracle from running.
    if bundle
        .history
        .violations
        .iter()
        .any(|v| v.class != chaoscontrol_smr::SafetyViolationClass::LosslessGap)
        || oracle.verdict == chaoscontrol_smr::observation::OracleVerdict::Fail
        || liveness.verdict == chaoscontrol_smr::LivenessVerdict::Fail
    {
        return "external-failure";
    }
    if oracle.verdict == chaoscontrol_smr::observation::OracleVerdict::Incomplete {
        return "observer-incomplete";
    }
    if bundle.replay.as_ref().is_some_and(|r| !r.comparison.accepted) {
        return "semantic-replay-mismatch";
    }
    match oracle.verdict {
        chaoscontrol_smr::observation::OracleVerdict::Pass => "bounded-instrumentation-valid",
        chaoscontrol_smr::observation::OracleVerdict::NotEvaluated => "not-evaluated",
        chaoscontrol_smr::observation::OracleVerdict::Fail => "external-failure",
        chaoscontrol_smr::observation::OracleVerdict::Incomplete => "observer-incomplete",
    }
}
