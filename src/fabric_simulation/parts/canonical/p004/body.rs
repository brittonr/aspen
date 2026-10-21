// r[impl molten.fabric_simulation.live_differential]
// r[impl molten.fabric_simulation.differential_results]
pub fn execution_differential(
    input: ExecutionDifferentialInput<'_>,
) -> crate::error::Result<ExecutionDifferential> {
    let ExecutionDifferentialInput {
        workload,
        simulation_side_ref,
        live_side_ref,
        simulated,
        live,
    } = input;
    for reference in [workload.workload_ref.as_str(), simulation_side_ref, live_side_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let decision = match live {
        Some(classes) => evaluate_live_differential(workload, simulated, classes),
        None => adapter_unavailable(workload),
    };
    let steps = execution_steps_value(workload, simulated, live);
    let parts = execution_decision_parts(&decision)?;
    let value = crate::preserves_rail::record("fabric-simulation-execution-differential-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_EXECUTION_DIFFERENTIAL_SCHEMA),
        field("result-kind", crate::preserves_rail::string(decision.result_kind())),
        field("workload-ref", crate::preserves_rail::string(&workload.workload_ref)),
        field("simulation-side-ref", crate::preserves_rail::string(simulation_side_ref)),
        field("live-side-ref", crate::preserves_rail::string(live_side_ref)),
        field("steps", crate::preserves_rail::sequence(steps)),
        field("divergences", crate::preserves_rail::sequence(parts.divergences)),
        field("ordering-divergences", crate::preserves_rail::sequence(parts.ordering)),
        field("incomplete", crate::preserves_rail::sequence(parts.incomplete)),
        field("failure", parts.failure),
        checks(&EXECUTION_DIFFERENTIAL_NON_CLAIMS),
    ]);
    Ok(ExecutionDifferential {
        report_ref: crate::preserves_rail::canonical_hash(&value)?,
        result_kind: decision.result_kind(),
        workload_ref: workload.workload_ref.clone(),
        simulation_side_ref: simulation_side_ref.to_string(),
        live_side_ref: live_side_ref.to_string(),
        decision,
        value,
    })
}

fn execution_steps_value(
    workload: &LiveWorkload,
    simulated: &[Option<OutcomeClass>],
    live: Option<&[Option<OutcomeClass>]>,
) -> Vec<preserves::IOValue> {
    workload
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            crate::preserves_rail::record("fabric-simulation-execution-step-v1", vec![
                field("sequence", crate::preserves_rail::u64_value(step.sequence)),
                field("class", crate::preserves_rail::string(step.class.as_str())),
                field("permitted", strings_value(step.permitted.iter().map(|class| class.as_str()))),
                field("simulated", optional_class(simulated.get(index).copied().flatten())),
                field("live", optional_class(live.and_then(|classes| classes.get(index).copied().flatten()))),
            ])
        })
        .collect()
}

struct ExecutionDecisionParts {
    divergences: Vec<preserves::IOValue>,
    ordering: Vec<preserves::IOValue>,
    incomplete: Vec<preserves::IOValue>,
    failure: preserves::IOValue,
}

fn execution_decision_parts(
    decision: &LiveDifferentialDecision,
) -> crate::error::Result<ExecutionDecisionParts> {
    let mut divergences = Vec::new();
    let mut ordering = Vec::new();
    let mut incomplete = Vec::new();
    if let LiveDifferentialDecision::PermittedDivergence {
        divergences: recorded,
        ordering: recorded_ordering,
    } = decision
    {
        for divergence in recorded {
            divergences.push(divergence_value(divergence)?);
        }
        for divergence in recorded_ordering {
            ordering.push(ordering_value(divergence));
        }
    }
    if let LiveDifferentialDecision::Incomplete {
        incomplete: recorded,
        divergences: recorded_divergences,
    } = decision
    {
        for step in recorded {
            incomplete.push(incomplete_value(step)?);
        }
        for divergence in recorded_divergences {
            divergences.push(divergence_value(divergence)?);
        }
    }
    let failure = match decision {
        LiveDifferentialDecision::Failure { outcome } => {
            crate::preserves_rail::record("some", vec![crate::preserves_rail::record(
                "unpermitted-outcome",
                vec![
                    field("step", u64_len(outcome.step)?),
                    field("simulated", crate::preserves_rail::string(outcome.simulated.as_str())),
                    field("live", crate::preserves_rail::string(outcome.live.as_str())),
                ],
            )])
        }
        _ => crate::preserves_rail::record("none", Vec::new()),
    };
    Ok(ExecutionDecisionParts {
        divergences,
        ordering,
        incomplete,
        failure,
    })
}

// r[impl molten.fabric_simulation.differential_evidence]
pub fn differential_evidence(
    input: DifferentialEvidenceInput<'_>,
) -> crate::error::Result<DifferentialEvidence> {
    let DifferentialEvidenceInput {
        structural,
        execution,
    } = input;
    if structural.report_ref == execution.report_ref {
        return Err(crate::error::Failure::invalid_harness(
            "structural and execution differentials must have distinct identities",
        ));
    }
    for reference in [
        structural.report_ref.as_str(),
        execution.report_ref.as_str(),
        execution.workload_ref.as_str(),
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let incomplete_count = match &execution.decision {
        LiveDifferentialDecision::Incomplete { incomplete, .. } => incomplete.len(),
        _ => 0,
    };
    let value = crate::preserves_rail::record("fabric-simulation-differential-evidence-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_DIFFERENTIAL_EVIDENCE_SCHEMA),
        field("structural-ref", crate::preserves_rail::string(&structural.report_ref)),
        field("execution-ref", crate::preserves_rail::string(&execution.report_ref)),
        field("workload-ref", crate::preserves_rail::string(&execution.workload_ref)),
        field(
            "structural-result",
            crate::preserves_rail::string(if structural.equivalent { "agreement" } else { "structural-difference" }),
        ),
        field("execution-result", crate::preserves_rail::string(execution.result_kind)),
        field("incomplete-count", u64_len(incomplete_count)?),
        checks(&EXECUTION_DIFFERENTIAL_NON_CLAIMS),
    ]);
    Ok(DifferentialEvidence {
        evidence_ref: crate::preserves_rail::canonical_hash(&value)?,
        structural_ref: structural.report_ref.clone(),
        execution_ref: execution.report_ref.clone(),
        workload_ref: execution.workload_ref.clone(),
        value,
    })
}

fn optional_class(class: Option<OutcomeClass>) -> preserves::IOValue {
    optional_string(class.map(OutcomeClass::as_str))
}

fn divergence_value(divergence: &PermittedDivergence) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("permitted-divergence-v1", vec![
        field("step", u64_len(divergence.step)?),
        field("simulated", crate::preserves_rail::string(divergence.simulated.as_str())),
        field("live", crate::preserves_rail::string(divergence.live.as_str())),
    ]))
}

fn ordering_value(divergence: &OrderingDivergence) -> preserves::IOValue {
    crate::preserves_rail::record("ordering-divergence-v1", vec![
        field("key", crate::preserves_rail::string(&divergence.key)),
        field("simulated", strings_value(divergence.simulated.iter().map(|class| class.as_str()))),
        field("live", strings_value(divergence.live.iter().map(|class| class.as_str()))),
    ])
}

fn incomplete_value(step: &IncompleteStep) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("incomplete-step-v1", vec![
        field("step", u64_len(step.step)?),
        field("side", crate::preserves_rail::string(step.side.as_str())),
        field("reason", crate::preserves_rail::string(step.reason.as_str())),
    ]))
}
