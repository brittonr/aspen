//! Exact-subject diagnostic execution over the existing deterministic harness.
//! Native process identity does not promote the simulation to distributed live evidence.

mod model;
mod source;
#[cfg(test)]
mod tests;

pub use model::Environment;
pub use model::ObservedReadback;
pub use model::ObservedSubject;
pub use model::Produced;
pub use model::Scenario;
pub use model::Subject;
pub use model::Validated;
pub use model::adapter_ref;
use preserves::ValueImpl as _;
pub use source::ReadbackSource;
pub use source::readback;

pub const PROFILE: &str = model::PROFILE;
pub const FAMILY: &str = model::FAMILY;
pub const REQUIREMENT: &str = model::REQUIREMENT;

pub fn fixture(scenario: Scenario) -> crate::error::Result<preserves::IOValue> {
    crate::preserves_rail::parse_text(match scenario {
        Scenario::GrantedSend => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/two-actor.preserves")),
        Scenario::DeniedSend => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/capability-qualification/denied-send.preserves"
        )),
    })
}

/// Read-only native facts, after current-process signed-source admission.
pub fn observe_subject(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<ObservedSubject> {
    let current = source::current(root)?;
    Ok(ObservedSubject {
        subject: current.source.observed_subject,
        binding_ref: current.source.binding_ref,
        validation_ref: current.source.validation_ref,
    })
}

/// Deterministic validation replay only: no files, live ports or product mutation.
/// Successful identities come from this admitted executing binary, not a wrapper.
pub fn observe_runs(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<ObservedReadback> {
    let admitted = source::current(root)?;
    let receipt_refs = std::collections::BTreeMap::from([
        observed_receipt(&admitted, Scenario::GrantedSend)?,
        observed_receipt(&admitted, Scenario::DeniedSend)?,
    ]);
    Ok(ObservedReadback {
        subject: admitted.source.observed_subject,
        binding_ref: admitted.source.binding_ref,
        validation_ref: admitted.source.validation_ref,
        receipt_refs,
    })
}

fn observed_receipt(admitted: &source::Current, scenario: Scenario) -> crate::error::Result<(String, String)> {
    let suite = fixture(scenario)?;
    let (artifact, _) = execute(&suite)?;
    let value = execution_value(
        &admitted.source.observed_subject,
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
pub fn produce(root: &crate::node_state::DirectoryRoot, scenario: Scenario) -> crate::error::Result<Produced> {
    // Authentication/measurement precedes execution; selected-output authoring is
    // deliberately insufficient to mint this current-process execution receipt.
    let admitted = source::current(root)?;
    let subject = admitted.source.observed_subject.clone();
    let suite = fixture(scenario)?;
    let (artifact, decision) = execute(&suite)?;
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

fn execute(suite: &preserves::IOValue) -> crate::error::Result<(preserves::IOValue, &'static str)> {
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
    let subject: Subject =
        serde_json::from_str(&text(&fields[1])?).map_err(|_| invalid("invalid native observed subject"))?;
    if &subject != expected
        || subject != source.observed_subject
        || subject.source_ref != source.binding.source_ref
        || subject.build_ref != source.binding.executable_ref
        || subject.dependency_closure_ref != source.binding.dependencies_ref
        || subject.execution_profile != PROFILE
        || subject.adapter_refs != [adapter_ref()?]
        || subject.environment != Environment::Simulation
    {
        return Err(invalid("runtime receipt exact subject/profile/adapter/environment mismatch"));
    }
    let scenario = Scenario::parse(&text(&fields[2])?)?;
    let validation = fields[4].clone().into();
    if crate::preserves_rail::canonical_hash(&validation)? != source.validation_ref {
        return Err(invalid("runtime receipt lacks the matching actual startup source validation"));
    }
    let suite: preserves::IOValue = fields[5].clone().into();
    if crate::preserves_rail::canonical_hash(&suite)? != crate::preserves_rail::canonical_hash(&fixture(scenario)?)? {
        return Err(invalid("runtime receipt finite fixture mismatch"));
    }
    let artifact: preserves::IOValue = fields[6].clone().into();
    let (decision, canonical_decision) = if crate::harness::parse_failure(&artifact).is_ok() {
        ("fail", "fail")
    } else {
        let native = crate::harness::validate_report_value(&artifact)?;
        if native.suite_ref != crate::preserves_rail::canonical_hash(&suite)? {
            return Err(invalid("native report executed another suite"));
        }
        crate::harness::replay_report_value(&artifact)?;
        if source.observed_receipts.get(scenario.id()) != Some(&crate::preserves_rail::canonical_hash(value)?) {
            return Err(invalid(
                "successful receipt differs from the admitted selected binary's actual deterministic runtime execution",
            ));
        }
        (
            "pass",
            if scenario == Scenario::DeniedSend {
                "expected-deny"
            } else {
                "pass"
            },
        )
    };
    Ok(Validated {
        requirement_id: REQUIREMENT,
        family: FAMILY,
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

fn text(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|text| text.to_string())
        .ok_or_else(|| invalid("expected runtime receipt string"))
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
