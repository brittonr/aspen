
pub fn run_local_node_dogfood(input: &LocalNodeDogfoodInput<'_>) -> Result<LocalNodeDogfoodRun> {
    let state_root_ref = dogfood_ref("state-root")?;
    if let Some(dirty_reason) = dirty_state_reason(input.state_root)? {
        return dirty_state_report(&state_root_ref, dirty_reason);
    }
    let mut run = LocalRunState::new(input.state_root, state_root_ref)?;
    let start = run.record_start()?;
    let installed = run.record_install(&start.startup_ref)?;
    run.record_service()?;
    let remote = run.record_remote()?;
    let job = run.record_job()?;
    let retention_gc = run.record_gc()?;
    run.record_catalog(&installed, &remote.run.receipt_value)?;
    run.record_repro(&remote.gate_ref)?;
    run.finish(&start, &installed, &job, &retention_gc)
}

struct LocalRunState<'a> {
    state_root: &'a Path,
    state_root_ref: String,
    registry_root: std::path::PathBuf,
    ledger_root: std::path::PathBuf,
    job_source_root: std::path::PathBuf,
    job_target_root: std::path::PathBuf,
    retention_root: std::path::PathBuf,
    bundle_root: std::path::PathBuf,
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    resource_refs: Vec<String>,
    step_checkpoints: StepCheckpointBuffers,
    gate_receipt_refs: Vec<String>,
    repro_bundle_refs: Vec<String>,
    harness_gate_refs: Vec<String>,
    catalog_query_refs: Vec<String>,
    repro_verify_refs: Vec<String>,
    replay_index_refs: Vec<String>,
}

impl<'a> LocalRunState<'a> {
    fn new(state_root: &'a Path, state_root_ref: String) -> Result<Self> {
        std::fs::create_dir_all(state_root).map_err(MoltenError::from)?;
        Ok(Self {
            state_root,
            state_root_ref,
            registry_root: state_root.join("registry"),
            ledger_root: state_root.join("ledger"),
            job_source_root: state_root.join("job-source-registry"),
            job_target_root: state_root.join("job-target-registry"),
            retention_root: state_root.join("retention-store"),
            bundle_root: state_root.join("retention-bundle"),
            policy_refs: vec![dogfood_ref("operator-policy")?],
            capability_refs: vec![dogfood_ref("operator-capability")?],
            resource_refs: vec![dogfood_ref("operator-resource")?],
            step_checkpoints: StepCheckpointBuffers::default(),
            gate_receipt_refs: Vec::new(),
            repro_bundle_refs: Vec::new(),
            harness_gate_refs: Vec::new(),
            catalog_query_refs: Vec::new(),
            repro_verify_refs: Vec::new(),
            replay_index_refs: Vec::new(),
        })
    }

    fn record_start(&mut self) -> Result<StartSteps> {
        record_start_steps(StartStepInput {
            state_root: self.state_root,
            state_root_ref: &self.state_root_ref,
            policy_refs: &self.policy_refs,
            capability_refs: &self.capability_refs,
            resource_refs: &self.resource_refs,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_install(&mut self, startup_ref: &str) -> Result<crate::artifacts::ArtifactInstall> {
        record_install_step(InstallStepInput {
            registry_root: &self.registry_root,
            startup_ref,
            policy_refs: &self.policy_refs,
            capability_refs: &self.capability_refs,
            state_root_ref: &self.state_root_ref,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_service(&mut self) -> Result<()> {
        record_service_step(ServiceStepInput {
            state_root_ref: &self.state_root_ref,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_remote(&mut self) -> Result<RemoteStep> {
        let remote = record_remote_step(RemoteStepInput {
            state_root: self.state_root,
            state_root_ref: &self.state_root_ref,
            policy_refs: &self.policy_refs,
            resource_refs: &self.resource_refs,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.gate_receipt_refs
            .push_limited_value(remote.gate_ref.clone(), MAX_OPERATOR_REFS, "dogfood gate refs")?;
        Ok(remote)
    }

    fn record_job(&mut self) -> Result<JobRun> {
        record_job_step(JobStepInput {
            state_root: self.state_root,
            source: &self.job_source_root,
            target: &self.job_target_root,
            state_root_ref: &self.state_root_ref,
            policy_refs: &self.policy_refs,
            capability_refs: &self.capability_refs,
            resource_refs: &self.resource_refs,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_gc(&mut self) -> Result<GcRun> {
        let retention_gc = record_gc_steps(GcStepInput {
            root: &self.retention_root,
            bundle_dir: &self.bundle_root,
            ledger_root: &self.ledger_root,
            registry_root: &self.registry_root,
            state_root_ref: &self.state_root_ref,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.catalog_query_refs.push_limited_value(
            retention_gc.catalog_receipt_ref.clone(),
            MAX_OPERATOR_REFS,
            "catalog query refs",
        )?;
        Ok(retention_gc)
    }

    fn record_catalog(
        &mut self,
        installed: &crate::artifacts::ArtifactInstall,
        remote_gate_value: &IoValue,
    ) -> Result<()> {
        let mcp_receipt_ref = record_catalog_step(CatalogStepInput {
            ledger_root: &self.ledger_root,
            registry_root: &self.registry_root,
            state_root_ref: &self.state_root_ref,
            installed,
            remote_gate_value,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.catalog_query_refs.push_limited_value(mcp_receipt_ref, MAX_OPERATOR_REFS, "catalog query refs")
    }

    fn record_repro(&mut self, remote_gate_ref: &str) -> Result<()> {
        let repro = record_repro_steps(ReproStepInput {
            state_root_ref: &self.state_root_ref,
            remote_gate_ref,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.harness_gate_refs
            .push_limited_value(repro.gate_ref.clone(), MAX_OPERATOR_REFS, "harness gate refs")?;
        self.gate_receipt_refs
            .push_limited_value(repro.gate_ref.clone(), MAX_OPERATOR_REFS, "dogfood gate refs")?;
        self.repro_bundle_refs
            .push_limited_value(repro.bundle_ref.clone(), MAX_OPERATOR_REFS, "dogfood repro refs")?;
        self.repro_verify_refs
            .push_limited_value(repro.verify_ref, MAX_OPERATOR_REFS, "dogfood repro verify refs")
    }

    fn finish(
        self,
        start: &StartSteps,
        installed: &crate::artifacts::ArtifactInstall,
        job: &JobRun,
        retention_gc: &GcRun,
    ) -> Result<LocalNodeDogfoodRun> {
        let Self {
            state_root_ref,
            ledger_root,
            policy_refs,
            capability_refs,
            resource_refs,
            step_checkpoints,
            gate_receipt_refs,
            repro_bundle_refs,
            harness_gate_refs,
            catalog_query_refs,
            repro_verify_refs,
            replay_index_refs,
            ..
        } = self;
        finish_run(FinishInput {
            ledger_root: &ledger_root,
            state_root_ref: &state_root_ref,
            startup_ref: &start.startup_ref,
            node_started: &start.node_started,
            installed,
            job,
            retention_gc,
            step_checkpoints,
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
            gate_receipt_refs,
            repro_bundle_refs,
            harness_gate_refs,
            catalog_query_refs,
            repro_verify_refs,
            replay_index_refs,
        })
    }
}

pub fn operator_dogfood_summary(value: &IoValue) -> Result<String> {
    if let Some(summary) = base_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = evidence_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = promotion_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = export_summary(value) {
        return Ok(summary);
    }
    Err(MoltenError::invalid_harness("unsupported operator dogfood artifact for summary"))
}

fn base_summary(value: &IoValue) -> Option<String> {
    if let Ok(report) = parse_dogfood_report(value) {
        return Some(format!(
            "operator dogfood report ref={} decision={} workflow={} final_state={} steps={} gates={} repro={} diagnostics={} (summary is non-normative)",
            report.report_ref,
            report.decision,
            report.workflow_ref,
            report.final_state_ref,
            report.step_receipts.len(),
            report.gate_receipts.len(),
            report.repro_bundles.len(),
            report.diagnostics.len()
        ));
    }
    if let Ok(workflow) = parse_operator_workflow(value) {
        return Some(format!(
            "operator workflow ref={} id={} steps={} replay={} (summary is non-normative)",
            workflow.workflow_ref,
            workflow.workflow_id,
            workflow.steps.len(),
            workflow.replay_profile
        ));
    }
    if let Ok(checkpoint) = parse_operator_checkpoint(value) {
        return Some(format!(
            "operator checkpoint ref={} workflow={} sequence={} step={} receipt={} (summary is non-normative)",
            checkpoint.checkpoint_ref,
            checkpoint.workflow_id,
            checkpoint.sequence,
            checkpoint.step_ref,
            checkpoint.receipt_ref.as_deref().unwrap_or("none")
        ));
    }
    if let Ok(receipt) = parse_release_gate_receipt(value) {
        return Some(format!(
            "operator release gate receipt ref={} decision={} report={} checks={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.report_ref,
            receipt.checks.len()
        ));
    }
    None
}
