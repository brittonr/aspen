
impl<'a> RunCtx<'a> {
    fn new(
        suite: &'a Suite,
        manifests: &'a OrderedMap<String, crate::service_records::ServiceManifest>,
    ) -> Result<Self> {
        let ready_statuses = ready_status_map(&suite.statuses)?;
        let statuses = suite.statuses.iter().map(|status| status.value.clone()).collect::<Vec<_>>();
        Ok(Self {
            evidence: &suite.evidence,
            manifests,
            ready_statuses,
            artifacts: Artifacts::new(statuses),
            runtime: crate::runtime::RuntimeState::new(1),
        })
    }

    fn run_demands(
        &mut self,
        mut pending: Vec<crate::service_records::ServiceDemand>,
        is_cycle_present: bool,
    ) -> Result<()> {
        let mut passes = 0usize;
        while !pending.is_empty() && !is_cycle_present {
            passes = next_pass_count(passes)?;
            let pass = self.run_pass(pending)?;
            pending = pass.pending;
            if !pass.is_progress_made {
                break;
            }
        }
        self.finish_pending(pending, is_cycle_present)
    }

    fn run_pass(&mut self, pending: Vec<crate::service_records::ServiceDemand>) -> Result<PassResult> {
        let mut next_pending = Vec::with_capacity(pending.len());
        let mut is_progress_made = false;
        for demand in pending {
            match self.step(demand)? {
                StepOutcome::Started => {
                    is_progress_made = true;
                }
                StepOutcome::Finished => {}
                StepOutcome::Pending(demand) => next_pending.push(demand),
            }
        }
        Ok(PassResult {
            pending: next_pending,
            is_progress_made,
        })
    }

    fn step(&mut self, demand: crate::service_records::ServiceDemand) -> Result<StepOutcome> {
        let Some(manifest) = self.manifests.get(&demand.service_id) else {
            self.artifacts.push_outcome(missing_manifest_outcome(&demand)?)?;
            return Ok(StepOutcome::Finished);
        };
        if manifest_ref_mismatch(&demand, manifest) {
            self.artifacts.push_outcome(deny_outcome(
                &demand,
                Some(manifest),
                "demand manifest ref does not match resolved manifest",
            )?)?;
            return Ok(StepOutcome::Finished);
        }
        let dependency_refs = dependency_status_refs(manifest, &self.ready_statuses);
        if dependency_refs.len() != manifest.dependencies.len() {
            return Ok(StepOutcome::Pending(demand));
        }
        let admission_diagnostics = startup_admission_diagnostics(self.evidence);
        if admission_diagnostics.is_empty() {
            let outcome = start_outcome(&mut self.runtime, self.evidence, &demand, manifest, dependency_refs)?;
            self.track_ready_status(&outcome)?;
            self.artifacts.push_outcome(outcome)?;
            Ok(StepOutcome::Started)
        } else {
            self.artifacts
                .push_outcome(deny_outcome(&demand, Some(manifest), &admission_diagnostics.join("; "))?)?;
            Ok(StepOutcome::Finished)
        }
    }

    fn track_ready_status(&mut self, outcome: &DemandOutcome) -> Result<()> {
        if let Some(status) = outcome.status.as_ref() {
            let parsed = crate::service_records::parse_service_status(status)?;
            self.ready_statuses.insert(parsed.service_id.clone(), parsed.status_ref);
        }
        Ok(())
    }

    fn finish_pending(
        &mut self,
        pending: Vec<crate::service_records::ServiceDemand>,
        is_cycle_present: bool,
    ) -> Result<()> {
        for demand in pending {
            let manifest = self.manifests.get(&demand.service_id);
            let diagnostic = if is_cycle_present {
                "dependency cycle detected"
            } else {
                "required service dependency is not ready"
            };
            let outcome = if is_cycle_present {
                dependency_deny_outcome(&demand, manifest, diagnostic)?
            } else {
                dependency_wait_outcome(&demand, manifest, diagnostic)?
            };
            self.artifacts.push_outcome(outcome)?;
        }
        Ok(())
    }

    fn into_artifacts(self) -> Artifacts {
        self.artifacts
    }
}

pub fn suite_value(input: &SuiteInput) -> Result<preserves::IOValue> {
    validate_suite_input(input)?;
    Ok(record("service-runtime-suite-v1", vec![
        string(RUNTIME_SUITE_SCHEMA),
        record("manifests", vec![sequence(input.manifests.clone())]),
        record("demands", vec![sequence(input.demands.clone())]),
        record("statuses", vec![sequence(input.statuses.clone())]),
        evidence_value(&input.evidence),
        checks_value(&[
            "canonical-service-runtime-suite",
            "explicit-startup-evidence",
            "no-ambient-supervisor",
            "bounded-inputs",
        ]),
    ]))
}

pub fn parse_suite(value: &preserves::IOValue) -> Result<Suite> {
    let fields = value
        .collect_simple_record("service-runtime-suite-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-runtime-suite-v1 ...>"))?;
    require_schema(&fields[0], RUNTIME_SUITE_SCHEMA, "service runtime suite schema")?;
    let checks = parse_checks(&fields[5])?;
    require_check(&checks, "canonical-service-runtime-suite", "service runtime suite")?;
    let manifest_values = parse_iovalue_sequence(&fields[1], "manifests")?;
    let demand_values = parse_iovalue_sequence(&fields[2], "demands")?;
    let status_values = parse_iovalue_sequence(&fields[3], "statuses")?;
    let manifests = manifest_values
        .iter()
        .map(crate::service_records::parse_service_manifest)
        .collect::<Result<Vec<_>>>()?;
    let demands = demand_values.iter().map(crate::service_records::parse_service_demand).collect::<Result<Vec<_>>>()?;
    let statuses =
        status_values.iter().map(crate::service_records::parse_service_status).collect::<Result<Vec<_>>>()?;
    let evidence = parse_evidence(&fields[4])?;
    validate_runtime_evidence(&evidence)?;
    Ok(Suite {
        suite_ref: canonical_hash(value)?,
        manifests,
        demands,
        statuses,
        evidence,
        value: value.clone(),
    })
}

pub fn run_suite_value(value: &preserves::IOValue) -> Result<Run> {
    let suite = parse_suite(value)?;
    run_suite(&suite)
}

pub fn run_suite(suite: &Suite) -> Result<Run> {
    let manifests = manifest_map(&suite.manifests)?;
    let is_cycle_present = dependency_cycle_exists(&manifests)?;
    let mut context = RunCtx::new(suite, &manifests)?;
    context.run_demands(sorted_demands(&suite.demands), is_cycle_present)?;
    finish_runtime_run(suite, context.into_artifacts())
}

fn manifest_map(
    manifests: &[crate::service_records::ServiceManifest],
) -> Result<OrderedMap<String, crate::service_records::ServiceManifest>> {
    let mut mapped = OrderedMap::new();
    for manifest in manifests {
        if mapped.insert(manifest.service_id.clone(), manifest.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate service manifest for {}",
                manifest.service_id
            )));
        }
    }
    Ok(mapped)
}

fn sorted_demands(demands: &[crate::service_records::ServiceDemand]) -> Vec<crate::service_records::ServiceDemand> {
    let mut sorted = demands.to_vec();
    sorted.sort_by(|left, right| {
        left.service_id.cmp(&right.service_id).then_with(|| left.demand_ref.cmp(&right.demand_ref))
    });
    sorted
}

fn next_pass_count(passes: usize) -> Result<usize> {
    let next = passes
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("service dependency pass count overflow"))?;
    if next > MAX_DEPENDENCY_PASSES {
        return Err(MoltenError::invalid_harness("service dependency evaluation exceeded pass bound"));
    }
    Ok(next)
}

fn finish_runtime_run(suite: &Suite, artifacts: Artifacts) -> Result<Run> {
    let report_value = report_value(ReportValueInput {
        suite_value: &suite.value,
        lifecycle_receipts: artifacts.lifecycle_receipts.as_slice(),
        statuses: artifacts.statuses.as_slice(),
        readiness_assertions: artifacts.readiness_assertions.as_slice(),
        replay_identities: artifacts.replay_identities.as_slice(),
        turn_contexts: artifacts.turn_contexts.as_slice(),
    })?;
    let Artifacts {
        lifecycle_receipts,
        statuses,
        readiness_assertions,
        replay_identities,
        turn_contexts,
    } = artifacts;
    Ok(Run {
        suite_ref: suite.suite_ref.clone(),
        suite_value: suite.value.clone(),
        report_ref: canonical_hash(&report_value)?,
        lifecycle_receipts: lifecycle_receipts.into_values(),
        statuses: statuses.into_values(),
        readiness_assertions: readiness_assertions.into_values(),
        replay_identities: replay_identities.into_values(),
        turn_contexts: turn_contexts.into_values(),
        value: report_value,
    })
}

pub fn replay_report(value: &preserves::IOValue) -> Result<Replay> {
    let report = parse_report(value)?;
    let rerun = run_suite_value(&report.suite_value)?;
    let expected_report_ref = canonical_hash(value)?;
    let decision = if expected_report_ref == rerun.report_ref {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    if decision == "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "service runtime replay divergence: expected {expected_report_ref}, got {}",
            rerun.report_ref
        )));
    }
    Ok(Replay {
        expected_report_ref,
        actual_report_ref: rerun.report_ref,
        decision,
    })
}

pub fn parse_report(value: &preserves::IOValue) -> Result<Run> {
    let fields = value
        .collect_simple_record("service-runtime-report-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-runtime-report-v1 ...>"))?;
    require_schema(&fields[0], RUNTIME_REPORT_SCHEMA, "service runtime report schema")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-service-runtime-report", "service runtime report")?;
    let suite_value = record_iovalue(&fields[1], "suite")?;
    let suite_ref = canonical_hash(&suite_value)?;
    let lifecycle_receipts = parse_iovalue_sequence(&fields[2], "lifecycle")?;
    let statuses = parse_iovalue_sequence(&fields[3], "statuses")?;
    let readiness_assertions = parse_iovalue_sequence(&fields[4], "readiness")?;
    let replay_identities = parse_iovalue_sequence(&fields[5], "replay-identities")?;
    let turn_contexts = parse_iovalue_sequence(&fields[6], "turn-contexts")?;
    Ok(Run {
        suite_ref,
        suite_value,
        report_ref: canonical_hash(value)?,
        lifecycle_receipts,
        statuses,
        readiness_assertions,
        replay_identities,
        turn_contexts,
        value: value.clone(),
    })
}
