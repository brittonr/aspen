use preserves::ValueImpl;

pub fn boundary_coverage_value(report_value: &preserves::IOValue) -> crate::error::Result<preserves::IOValue> {
    let report = super::reading::parse_report(report_value)?;
    let suite = super::intake::parse_suite(&report.suite_value)?;
    let mut coverage = Vec::new();
    push_boundary_coverage(
        &mut coverage,
        "envelope-routes",
        suite.steps.iter().any(|step| matches!(step, crate::harness::nucleus::CoreStep::Send { .. })),
    );
    push_boundary_coverage(
        &mut coverage,
        "dataspace-semantics",
        suite.steps.iter().any(|step| {
            matches!(
                step,
                crate::harness::nucleus::CoreStep::Observe { .. }
                    | crate::harness::nucleus::CoreStep::Assert { .. }
                    | crate::harness::nucleus::CoreStep::Retract { .. }
            )
        }),
    );
    push_boundary_coverage(
        &mut coverage,
        "policy-gates",
        report.policy_gate.is_some() && report.capability_gate.is_some(),
    );
    push_boundary_coverage(&mut coverage, "policy-denials", report_has_denied_admission(&report)?);
    push_boundary_coverage(&mut coverage, "effects", !report.effect_log.is_empty());
    push_boundary_coverage(
        &mut coverage,
        "receipts",
        report.policy_gate.is_some() && report.capability_gate.is_some() && report.budget_gate.is_some(),
    );
    push_boundary_coverage(&mut coverage, "traces", !report.observations.is_empty());
    push_boundary_coverage(&mut coverage, "storage-paths", false);
    push_boundary_coverage(&mut coverage, "resources", report.budget_gate.is_some());
    push_boundary_coverage(
        &mut coverage,
        "replay-branches",
        matches!(report.replay_status.as_str(), "deterministic" | "replay" | "record"),
    );
    push_boundary_coverage(
        &mut coverage,
        "adapters",
        report.actors.iter().any(|actor| !matches!(actor.kind, super::model::ActorKind::Native)),
    );
    push_boundary_coverage(
        &mut coverage,
        "confidentiality-paths",
        report_value_contains_label(report_value, "redaction-gate-v1"),
    );

    let unexercised = coverage
        .iter()
        .filter_map(|value| {
            let fields = value.collect_simple_record("boundary", Some(2))?;
            let name = super::fields::required_string(&fields[0], "coverage boundary name").ok()?;
            let status = super::fields::required_string(&fields[1], "coverage boundary status").ok()?;
            (status == "unexercised").then_some(crate::preserves_rail::string(name))
        })
        .collect::<Vec<_>>();

    Ok(crate::preserves_rail::record("harness-boundary-coverage-v1", vec![
        crate::preserves_rail::string("molten.harness.boundary-coverage.v1"),
        crate::preserves_rail::record("report-ref", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&report.suite_ref)]),
        crate::preserves_rail::sequence(coverage),
        crate::preserves_rail::record("unexercised", vec![crate::preserves_rail::sequence(unexercised)]),
    ]))
}

fn push_boundary_coverage(out: &mut impl crate::bounded::VecSink<preserves::IOValue>, name: &str, exercised: bool) {
    out.push_item(crate::preserves_rail::record("boundary", vec![
        crate::preserves_rail::string(name),
        crate::preserves_rail::string(if exercised { "exercised" } else { "unexercised" }),
    ]));
}

fn report_has_denied_admission(report: &super::model::Report) -> crate::error::Result<bool> {
    for observation in &report.observations {
        for event in &observation.events {
            if event.collect_simple_record("admission-decision-v1", None).is_some()
                && !super::vetting::parse_admission_decision_event(event)?.decision.is_allowed()
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn report_value_contains_label(value: &preserves::IOValue, label: &str) -> bool {
    crate::preserves_rail::to_text(value).is_ok_and(|text| text.contains(label))
}
