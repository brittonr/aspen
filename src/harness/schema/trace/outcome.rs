use preserves::ValueImpl;

pub struct ReportValueInput<'a> {
    pub suite: &'a super::model::Suite,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub policy_gate: preserves::IOValue,
    pub capability_gate: preserves::IOValue,
    pub budget_gate: preserves::IOValue,
    pub observations: Vec<preserves::IOValue>,
    pub effect_log: Vec<super::observed::EffectLogEntry>,
    pub budget: &'a super::model::Budget,
    pub usage: &'a super::model::BudgetUsage,
}

pub fn report_value(input: ReportValueInput<'_>) -> preserves::IOValue {
    let executor_preflights = match super::readiness::executor_preflights_value(input.suite) {
        Ok(value) => value,
        Err(error) => crate::preserves_rail::record("executor-preflights-invalid-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA),
            crate::preserves_rail::record("error", vec![crate::preserves_rail::string(error.to_string())]),
        ]),
    };
    crate::preserves_rail::record("harness-report-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REPORT_SCHEMA),
        crate::preserves_rail::string("pass"),
        crate::preserves_rail::string("deterministic"),
        crate::preserves_rail::string("local-deterministic"),
        crate::preserves_rail::string(crate::preserves_rail::HASH_ALGORITHM),
        crate::preserves_rail::string(input.suite_ref),
        crate::preserves_rail::string(input.initial_state_hash),
        crate::preserves_rail::string(input.final_state_hash),
        input.suite.source_value.clone(),
        input.policy_gate,
        input.capability_gate,
        input.budget_gate,
        super::roster::actor_registry_value(&input.suite.actors),
        executor_preflights,
        crate::preserves_rail::sequence(input.observations),
        super::tape::effect_log_value(&input.effect_log),
        super::allowance::budget_value(input.budget, input.usage),
    ])
}

pub fn failure_value(
    phase: &str,
    error: &crate::error::Failure,
    mut diagnostics: Vec<preserves::IOValue>,
) -> preserves::IOValue {
    diagnostics.extend(error_diagnostics(error));
    crate::preserves_rail::record("harness-failure-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_FAILURE_SCHEMA),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(phase)]),
        crate::preserves_rail::record("kind", vec![crate::preserves_rail::string(error_kind(error))]),
        crate::preserves_rail::record("message", vec![crate::preserves_rail::string(error.to_string())]),
        crate::preserves_rail::sequence(diagnostics),
    ])
}

pub fn suite_failure_value(
    phase: &str,
    error: &crate::error::Failure,
    suite_value: &preserves::IOValue,
) -> crate::error::Result<preserves::IOValue> {
    Ok(failure_value(phase, error, vec![
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(suite_value)?,
        )]),
        crate::preserves_rail::record("suite", vec![suite_value.clone()]),
    ]))
}

pub fn report_failure_value(
    phase: &str,
    error: &crate::error::Failure,
    report_value: &preserves::IOValue,
) -> crate::error::Result<preserves::IOValue> {
    Ok(failure_value(phase, error, vec![
        crate::preserves_rail::record("report-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(report_value)?,
        )]),
        crate::preserves_rail::record("report", vec![report_value.clone()]),
    ]))
}

pub fn parse_failure(failure_value: &preserves::IOValue) -> crate::error::Result<super::model::Failure> {
    let failure = super::fields::simple_record(failure_value, "harness-failure-v1", 5)?;
    let schema = super::fields::required_string(&failure[0], "failure schema")?;
    if schema != crate::preserves_rail::HARNESS_FAILURE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported failure schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_FAILURE_SCHEMA
        )));
    }
    let phase_value = crate::preserves_rail::value_to_iovalue(&failure[1]);
    let phase_record = super::fields::simple_record(&phase_value, "phase", 1)?;
    let phase = super::fields::required_string(&phase_record[0], "failure phase")?;
    if !matches!(phase.as_str(), "preflight" | "execute" | "replay" | "validate" | "export" | "verify" | "unpack") {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported failure phase {phase}")));
    }
    let kind_value = crate::preserves_rail::value_to_iovalue(&failure[2]);
    let kind_record = super::fields::simple_record(&kind_value, "kind", 1)?;
    let kind = super::fields::required_string(&kind_record[0], "failure kind")?;
    if kind.is_empty() {
        return Err(crate::error::Failure::invalid_harness("failure kind must not be empty"));
    }
    let message_value = crate::preserves_rail::value_to_iovalue(&failure[3]);
    let message_record = super::fields::simple_record(&message_value, "message", 1)?;
    let message = super::fields::required_string(&message_record[0], "failure message")?;
    let diagnostic_values = super::fields::required_sequence(&failure[4], "failure diagnostics")?;
    let mut diagnostics = Vec::with_capacity(diagnostic_values.len());
    for diagnostic in diagnostic_values.iter() {
        diagnostics.push(crate::preserves_rail::value_to_iovalue(&diagnostic));
    }
    Ok(super::model::Failure {
        failure_ref: crate::preserves_rail::canonical_hash(failure_value)?,
        phase,
        kind,
        message,
        diagnostics,
    })
}

pub fn failure_summary(failure_value: &preserves::IOValue) -> crate::error::Result<String> {
    let failure = parse_failure(failure_value)?;
    Ok(format!(
        "failure {}\nstatus=fail\nphase={}\nkind={}\nmessage={}\ndiagnostics={}",
        failure.failure_ref,
        failure.phase,
        failure.kind,
        failure.message,
        failure.diagnostics.len()
    ))
}

fn error_kind(error: &crate::error::Failure) -> String {
    match error {
        crate::error::Failure::Io(_) => "io".to_string(),
        crate::error::Failure::NodeStateWriteOpenDenied { .. }
        | crate::error::Failure::NodeStatusHealthOpenDenied { .. } => {
            crate::error::FailureClass::StorageFailure.code().to_string()
        }
        crate::error::Failure::Preserves(_) => "preserves".to_string(),
        crate::error::Failure::InvalidHarness(_) => "invalid-harness".to_string(),
        crate::error::Failure::Classified { class, .. } => class.code().to_string(),
        crate::error::Failure::HarnessDivergence(divergence) => divergence.kind.clone(),
    }
}

fn error_diagnostics(error: &crate::error::Failure) -> Vec<preserves::IOValue> {
    match error {
        crate::error::Failure::HarnessDivergence(divergence) => {
            let mut diagnostics = Vec::new();
            if let Some(step) = divergence.step {
                diagnostics.push(crate::preserves_rail::record("step", vec![crate::preserves_rail::u64_value(step)]));
            }
            diagnostics.push(crate::preserves_rail::record("expected", vec![crate::preserves_rail::string(
                &divergence.expected,
            )]));
            diagnostics
                .push(crate::preserves_rail::record("actual", vec![crate::preserves_rail::string(&divergence.actual)]));
            diagnostics
                .push(crate::preserves_rail::record("detail", vec![crate::preserves_rail::string(&divergence.detail)]));
            diagnostics
        }
        crate::error::Failure::NodeStateWriteOpenDenied { path, errno }
        | crate::error::Failure::NodeStatusHealthOpenDenied { path, errno, .. } => vec![
            crate::preserves_rail::record("path", vec![crate::preserves_rail::string(path)]),
            crate::preserves_rail::record("errno", vec![crate::preserves_rail::string(errno.to_string())]),
            crate::preserves_rail::record("phase", vec![crate::preserves_rail::string("open-before-write")]),
        ],
        crate::error::Failure::Io(_)
        | crate::error::Failure::Preserves(_)
        | crate::error::Failure::InvalidHarness(_)
        | crate::error::Failure::Classified { .. } => Vec::new(),
    }
}
