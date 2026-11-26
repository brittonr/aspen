//! Exact-subject execution of fixed qualification scenarios.
//! The harness scenarios run the deterministic simulation harness; product
//! scenarios run the actual runtime path of one reviewed product capability.
//! A native process identity never promotes evidence beyond its declared
//! environment (simulation, same-process or distinct-process on one host).

#[cfg(test)]
pub(crate) mod matrix;
mod model;
mod products;
mod source;
#[cfg(test)]
mod tests;

pub use model::CAPABILITIES;
pub use model::Definition;
pub use model::Environment;
pub use model::NativeProducer;
pub use model::ObservedReadback;
pub use model::ObservedSubject;
pub use model::Produced;
pub use model::Scenario;
pub use model::Subject;
pub use model::Validated;
pub use model::adapter_ref;
use preserves::ValueImpl as _;
pub use products::Launcher;
pub use source::ReadbackSource;
pub use source::readback;

pub fn fixture(scenario: Scenario) -> crate::error::Result<preserves::IOValue> {
    match scenario {
        Scenario::GrantedSend => crate::preserves_rail::parse_text(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/two-actor.preserves"
        ))),
        Scenario::DeniedSend => crate::preserves_rail::parse_text(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/capability-qualification/denied-send.preserves"
        ))),
        _ => Ok(products::plan(scenario)),
    }
}

/// Read-only native facts, after current-process signed-source admission.
pub fn observe_subject(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<ObservedSubject> {
    let current = source::current(root)?;
    Ok(ObservedSubject {
        subjects: current.source.observed_subjects,
        binding_ref: current.source.binding_ref,
        validation_ref: current.source.validation_ref,
    })
}

/// Independent re-execution of every fixed scenario by this admitted binary.
/// Successful identities come from this executing binary, not a wrapper.
pub fn observe_runs(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<ObservedReadback> {
    let admitted = source::current(root)?;
    let launcher = Launcher::current()?;
    let receipt_refs = Scenario::ALL
        .into_iter()
        .map(|scenario| observed_receipt(&admitted, scenario, &launcher))
        .collect::<crate::error::Result<std::collections::BTreeMap<_, _>>>()?;
    Ok(ObservedReadback {
        subjects: admitted.source.observed_subjects,
        binding_ref: admitted.source.binding_ref,
        validation_ref: admitted.source.validation_ref,
        receipt_refs,
    })
}

fn observed_receipt(
    admitted: &source::Current,
    scenario: Scenario,
    launcher: &Launcher,
) -> crate::error::Result<(String, String)> {
    let suite = fixture(scenario)?;
    let (artifact, _) = execute(scenario, &suite, launcher)?;
    let value = execution_value(
        admitted.source.subject_for(scenario)?,
        scenario,
        &admitted.source.binding_ref,
        ExecutionArtifacts {
            validation: admitted.validation_value.clone(),
            suite,
            artifact,
        },
    )?;
    Ok((scenario.id().to_string(), crate::preserves_rail::canonical_hash(&value)?))
}

// r[impl molten.testing.capability_qualification.exact_evidence]
// r[impl molten.testing.capability_context.request_binding]
// r[impl molten.testing.evidence_matrix_native_producers]
pub fn produce(root: &crate::node_state::DirectoryRoot, scenario: Scenario) -> crate::error::Result<Produced> {
    // Authentication/measurement precedes execution; selected-output authoring is
    // deliberately insufficient to mint this current-process execution receipt.
    let admitted = source::current(root)?;
    let subject = admitted.source.subject_for(scenario)?.clone();
    let suite = fixture(scenario)?;
    let (artifact, decision) = execute(scenario, &suite, &Launcher::current()?)?;
    let value = execution_value(&subject, scenario, &admitted.source.binding_ref, ExecutionArtifacts {
        validation: admitted.validation_value,
        suite,
        artifact,
    })?;
    Ok(Produced {
        receipt_ref: crate::preserves_rail::canonical_hash(&value)?,
        value,
        subject,
        decision,
    })
}

/// Entry for a distinct-process peer started by a product producer.
pub fn run_peer(role: &str, run_directory: &std::path::Path) -> crate::error::Result<()> {
    products::peer::run(role, run_directory)
}

fn execute(
    scenario: Scenario,
    suite: &preserves::IOValue,
    launcher: &Launcher,
) -> crate::error::Result<(preserves::IOValue, &'static str)> {
    if !scenario.is_harness() {
        return match products::execute(scenario, launcher) {
            Ok(observation) => match products::verify_observation(scenario, &observation) {
                Ok(()) => Ok((observation, "pass")),
                Err(error) => Ok((crate::harness::report_failure_value("validate", &error, &observation)?, "fail")),
            },
            Err(error) => Ok((crate::harness::suite_failure_value("execute", &error, suite)?, "fail")),
        };
    }
    match crate::harness::run_suite_value(suite) {
        Ok(run) => {
            if let Err(error) = crate::harness::validate_report_value(&run.report_value) {
                return Ok((crate::harness::report_failure_value("validate", &error, &run.report_value)?, "fail"));
            }
            if let Err(error) = crate::harness::replay_report_value(&run.report_value) {
                return Ok((crate::harness::report_failure_value("replay", &error, &run.report_value)?, "fail"));
            }
            Ok((run.report_value, "pass"))
        }
        Err(error) => Ok((crate::harness::suite_failure_value("execute", &error, suite)?, "fail")),
    }
}

struct ExecutionArtifacts {
    validation: preserves::IOValue,
    suite: preserves::IOValue,
    artifact: preserves::IOValue,
}

fn execution_value(
    subject: &Subject,
    scenario: Scenario,
    binding_ref: &str,
    artifacts: ExecutionArtifacts,
) -> crate::error::Result<preserves::IOValue> {
    let subject = serde_json::to_string(subject).map_err(|_| invalid("cannot encode observed subject"))?;
    Ok(crate::preserves_rail::record("capability-runtime-execution-v1", vec![
        crate::preserves_rail::string(model::SCHEMA),
        crate::preserves_rail::string(subject),
        crate::preserves_rail::string(scenario.id()),
        crate::preserves_rail::string(binding_ref),
        artifacts.validation,
        artifacts.suite,
        artifacts.artifact,
    ]))
}

// r[impl molten.testing.capability_qualification.exact_evidence]
// r[impl molten.testing.evidence_matrix_native_producers]
pub fn validate(
    source: &ReadbackSource,
    value: &preserves::IOValue,
    expected: &Subject,
) -> crate::error::Result<Validated> {
    let fields = value
        .collect_simple_record("capability-runtime-execution-v1", Some(7))
        .ok_or_else(|| invalid("expected native capability runtime execution receipt"))?;
    if text(&fields[0])? != model::SCHEMA || text(&fields[3])? != source.binding_ref {
        return Err(invalid("runtime receipt schema/source binding mismatch"));
    }
    let scenario = Scenario::parse(&text(&fields[2])?)?;
    let definition = scenario.definition();
    let capability = definition.capability;
    let subject: Subject =
        serde_json::from_str(&text(&fields[1])?).map_err(|_| invalid("invalid native observed subject"))?;
    if &subject != expected
        || &subject != source.subject_for(scenario)?
        || subject.source_ref != source.binding.source_ref
        || subject.build_ref != source.binding.executable_ref
        || subject.dependency_closure_ref != source.binding.dependencies_ref
        || subject.execution_profile != capability.execution_profile
        || subject.adapter_refs != [adapter_ref(&capability)?]
        || subject.environment != capability.environment
    {
        return Err(invalid("runtime receipt exact subject/profile/adapter/environment mismatch"));
    }
    let validation = fields[4].clone().into();
    if crate::preserves_rail::canonical_hash(&validation)? != source.validation_ref {
        return Err(invalid("runtime receipt lacks the matching actual startup source validation"));
    }
    let suite: preserves::IOValue = fields[5].clone().into();
    if crate::preserves_rail::canonical_hash(&suite)? != crate::preserves_rail::canonical_hash(&fixture(scenario)?)? {
        return Err(invalid("runtime receipt finite fixture mismatch"));
    }
    let artifact: preserves::IOValue = fields[6].clone().into();
    let decision = if crate::harness::parse_failure(&artifact).is_ok() {
        "fail"
    } else {
        if scenario.is_harness() {
            let native = crate::harness::validate_report_value(&artifact)?;
            if native.suite_ref != crate::preserves_rail::canonical_hash(&suite)? {
                return Err(invalid("native report executed another suite"));
            }
            crate::harness::replay_report_value(&artifact)?;
        } else {
            products::verify_observation(scenario, &artifact)?;
        }
        if source.observed_receipts.get(scenario.id()) != Some(&crate::preserves_rail::canonical_hash(value)?) {
            return Err(invalid(
                "successful receipt differs from the admitted selected binary's actual runtime execution",
            ));
        }
        "pass"
    };
    let canonical_decision = match (decision, definition.coverage_kind) {
        ("pass", "negative") => "expected-deny",
        ("pass", _) => "pass",
        _ => "fail",
    };
    Ok(Validated {
        requirement_id: definition.requirement,
        family: definition.family,
        scenario,
        subject,
        decision,
        canonical_decision,
    })
}

/// Diagnostic-only failure extraction. This cannot admit successful execution
/// or an exact subject, even when source authentication is unavailable.
pub fn failure(value: &preserves::IOValue) -> crate::error::Result<crate::harness::Failure> {
    let fields = value
        .collect_simple_record("capability-runtime-execution-v1", Some(7))
        .ok_or_else(|| invalid("expected native runtime failure carrier"))?;
    if text(&fields[0])? != model::SCHEMA {
        return Err(invalid("unsupported native runtime failure schema"));
    }
    crate::harness::parse_failure(&crate::preserves_rail::value_to_iovalue(&fields[6]))
}

/// Canonical identity of the subject-independent runtime artifact one fixed
/// scenario's actual execution produces in this process (the native harness
/// report or the product observation). Evidence-matrix rows bind this ref.
pub fn runtime_artifact(
    scenario: Scenario,
    launcher: &Launcher,
) -> crate::error::Result<(preserves::IOValue, &'static str)> {
    execute(scenario, &fixture(scenario)?, launcher)
}

fn text(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|text| text.to_string())
        .ok_or_else(|| invalid("expected runtime receipt string"))
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
