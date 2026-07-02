
struct FinishReport {
    workflow_value: IoValue,
    report_value: IoValue,
    report: DogfoodReport,
}

fn build_finish_report(input: FinishReportInput<'_>) -> Result<FinishReport> {
    let workflow_value = operator_workflow_value(&OperatorWorkflowInput {
        workflow_id: LOCAL_NODE_WORKFLOW_ID,
        steps: &input.checkpoints.steps,
        policy_refs: input.policy_refs,
        capability_refs: input.capability_refs,
        resource_refs: input.resource_refs,
        replay_profile: "recorded",
    })?;
    let final_state_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("operator-dogfood-final-state", vec![
            crate::preserves_rail::string(input.state_root_ref),
            crate::preserves_rail::string(input.shutdown_ref),
            crate::preserves_rail::string(input.health_ref),
        ]))?;
    let report_value = dogfood_report_value(&DogfoodReportInput {
        workflow_value: &workflow_value,
        checkpoint_values: &input.checkpoints.checkpoints,
        gate_receipt_refs: input.gate_receipt_refs,
        repro_bundle_refs: input.repro_bundle_refs,
        final_state_ref: &final_state_ref,
        diagnostics: &[],
    })?;
    let report = parse_dogfood_report(&report_value)?;
    Ok(FinishReport {
        workflow_value,
        report_value,
        report,
    })
}

struct ReleaseValueInput<'a> {
    report: &'a DogfoodReport,
    report_value: &'a IoValue,
    startup_ref: &'a str,
    shutdown_ref: &'a str,
    harness_gate_refs: &'a [String],
    catalog_query_refs: &'a [String],
    repro_verify_refs: &'a [String],
    replay_index_refs: &'a [String],
    retention_gc: &'a GcRun,
}

fn build_release_value(input: ReleaseValueInput<'_>) -> Result<Option<IoValue>> {
    let validation_command_refs = vec![dogfood_ref("cargo-nextest-ci")?];
    let gc_release_refs = vec![
        input.retention_gc.audit_ref.clone(),
        input.retention_gc.bundle_verify_ref.clone(),
        input.retention_gc.catalog_receipt_ref.clone(),
    ];
    if input.report.decision == "pass" {
        Ok(Some(release_gate_receipt_value(&ReleaseGateInput {
            report_value: input.report_value,
            node_startup_ref: input.startup_ref,
            node_shutdown_ref: input.shutdown_ref,
            harness_gate_refs: input.harness_gate_refs,
            catalog_query_refs: input.catalog_query_refs,
            repro_verify_refs: input.repro_verify_refs,
            replay_index_refs: input.replay_index_refs,
            gc_refs: &gc_release_refs,
            validation_command_refs: &validation_command_refs,
        })?))
    } else {
        Ok(None)
    }
}

struct FinishReplay {
    replay_verify: crate::deterministic_replay::ReplayVerifyReceipt,
    replay_index: crate::deterministic_replay::ReplayIndexReceipt,
    shutdown_ref: String,
    health_ref: String,
}

struct FinishState<'a> {
    input: FinishInput<'a>,
}

impl<'a> FinishState<'a> {
    fn record_replay(&mut self) -> Result<FinishReplay> {
        let replay = record_replay_shutdown(ReplayShutdownInput {
            state_root_ref: self.input.state_root_ref,
            startup_ref: self.input.startup_ref,
            node_started: self.input.node_started,
            installed: self.input.installed,
            job: self.input.job,
            step_checkpoints: std::mem::take(&mut self.input.step_checkpoints),
            replay_index_refs: std::mem::take(&mut self.input.replay_index_refs),
        })?;
        self.input.step_checkpoints = replay.step_checkpoints;
        self.input.replay_index_refs = replay.replay_index_refs;
        Ok(FinishReplay {
            replay_verify: replay.replay_verify,
            replay_index: replay.replay_index,
            shutdown_ref: replay.shutdown_ref,
            health_ref: replay.health_ref,
        })
    }

    fn build_report(&self, replay: &FinishReplay) -> Result<FinishReport> {
        build_finish_report(FinishReportInput {
            state_root_ref: self.input.state_root_ref,
            shutdown_ref: &replay.shutdown_ref,
            health_ref: &replay.health_ref,
            checkpoints: &self.input.step_checkpoints,
            policy_refs: self.input.policy_refs,
            capability_refs: self.input.capability_refs,
            resource_refs: self.input.resource_refs,
            gate_receipt_refs: &self.input.gate_receipt_refs,
            repro_bundle_refs: &self.input.repro_bundle_refs,
        })
    }

    fn build_release(&self, replay: &FinishReplay, finish_report: &FinishReport) -> Result<Option<IoValue>> {
        build_release_value(ReleaseValueInput {
            report: &finish_report.report,
            report_value: &finish_report.report_value,
            startup_ref: self.input.startup_ref,
            shutdown_ref: &replay.shutdown_ref,
            harness_gate_refs: &self.input.harness_gate_refs,
            catalog_query_refs: &self.input.catalog_query_refs,
            repro_verify_refs: &self.input.repro_verify_refs,
            replay_index_refs: &self.input.replay_index_refs,
            retention_gc: self.input.retention_gc,
        })
    }

    fn import_evidence(
        &self,
        replay: &FinishReplay,
        finish_report: &FinishReport,
        release_gate_value: Option<&IoValue>,
    ) -> Result<Vec<String>> {
        import_dogfood_evidence(DogfoodEvidenceImportInput {
            ledger_root: self.input.ledger_root,
            workflow_value: &finish_report.workflow_value,
            step_values: &self.input.step_checkpoints.steps,
            checkpoint_values: &self.input.step_checkpoints.checkpoints,
            report_value: &finish_report.report_value,
            release_gate_value,
            replay_verify_value: &replay.replay_verify.value,
            replay_index_value: &replay.replay_index.value,
        })
    }

    fn complete(
        self,
        replay: FinishReplay,
        finish_report: FinishReport,
        release_gate_value: Option<IoValue>,
        import_refs: Vec<String>,
    ) -> Result<LocalNodeDogfoodRun> {
        let release_gate_ref = release_gate_value.as_ref().map(crate::preserves_rail::canonical_hash).transpose()?;
        let StepCheckpointBuffers { steps, checkpoints } = self.input.step_checkpoints;
        Ok(LocalNodeDogfoodRun {
            decision: finish_report.report.decision,
            workflow_ref: crate::preserves_rail::canonical_hash(&finish_report.workflow_value)?,
            workflow_value: finish_report.workflow_value,
            step_values: steps,
            checkpoint_values: checkpoints,
            report_ref: finish_report.report.report_ref,
            report_value: finish_report.report_value,
            release_gate_ref,
            release_gate_value,
            replay_verify_ref: Some(replay.replay_verify.receipt_ref),
            replay_verify_value: Some(replay.replay_verify.value),
            replay_index_ref: Some(replay.replay_index.index_ref),
            replay_index_value: Some(replay.replay_index.value),
            ledger_import_receipt_refs: import_refs,
        })
    }
}

fn finish_run(input: FinishInput<'_>) -> Result<LocalNodeDogfoodRun> {
    let mut finish = FinishState { input };
    let replay = finish.record_replay()?;
    let finish_report = finish.build_report(&replay)?;
    let release_gate_value = finish.build_release(&replay, &finish_report)?;
    let import_refs = finish.import_evidence(&replay, &finish_report, release_gate_value.as_ref())?;
    finish.complete(replay, finish_report, release_gate_value, import_refs)
}

struct StartStepInput<'a> {
    state_root: &'a Path,
    state_root_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    checkpoints: &'a mut StepCheckpointBuffers,
}

struct StartSteps {
    node_started: crate::node_runtime::NodeRuntimeStart,
    startup_ref: String,
}

fn record_start_steps(input: StartStepInput<'_>) -> Result<StartSteps> {
    let StartStepInput {
        state_root,
        state_root_ref,
        policy_refs,
        capability_refs,
        resource_refs,
        checkpoints,
    } = input;
    let identity_resolution = resolve_identity(state_root, policy_refs)?;
    let identity = identity_resolution
        .identity
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("local dogfood identity resolution denied"))?;
    let identity_startup =
        crate::node_identity::startup_evidence_value(&identity.identity_ref, &identity_resolution.receipt_ref)?;
    let identity_startup_ref = crate::preserves_rail::canonical_hash(&identity_startup)?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "clean-state",
        request_ref: Some(state_root_ref),
        receipt_ref: Some(&identity_resolution.receipt_ref),
        result_ref: Some(&identity_startup_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[identity.identity_ref.clone(), identity_startup_ref.clone()],
        diagnostics: &[],
        state_root_ref,
    })?;

    let node_started =
        start_node(&identity, &identity_resolution.receipt_ref, policy_refs, capability_refs, resource_refs)?;
    let startup_ref = node_started.startup_receipt.receipt_ref.clone();
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "start-node",
        request_ref: Some(&node_started.config.config_ref),
        receipt_ref: Some(&startup_ref),
        result_ref: Some(&startup_ref),
        decision: &node_started.decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&node_started.config.config_ref),
        diagnostics: &node_started.startup_receipt.diagnostics,
        state_root_ref,
    })?;
    Ok(StartSteps {
        node_started,
        startup_ref,
    })
}

struct InstallStepInput<'a> {
    registry_root: &'a Path,
    startup_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    state_root_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_install_step(input: InstallStepInput<'_>) -> Result<crate::artifacts::ArtifactInstall> {
    let InstallStepInput {
        registry_root,
        startup_ref,
        policy_refs,
        capability_refs,
        state_root_ref,
        checkpoints,
    } = input;
    let installed = crate::artifacts::install_artifact(registry_root, &crate::artifacts::ArtifactInstallInput {
        kind: "operator-artifact".to_string(),
        payload: crate::preserves_rail::record("dogfood-artifact", vec![crate::preserves_rail::string("local-node")]),
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![startup_ref.to_string()],
        installer_ref: capability_refs[0].clone(),
        capability_refs: capability_refs.to_vec(),
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "install-artifact",
        request_ref: Some(startup_ref),
        receipt_ref: Some(&crate::preserves_rail::canonical_hash(&installed.receipt_value)?),
        result_ref: Some(&installed.artifact_ref),
        decision: &installed.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&installed.artifact_ref),
        diagnostics: &[],
        state_root_ref,
    })?;
    Ok(installed)
}
