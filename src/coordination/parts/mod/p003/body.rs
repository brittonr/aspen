
pub fn parse_coordination_apply_report(value: &IoValue) -> Result<CoordinationApplyReport> {
    let fields = simple_record(value, "coordination-apply-report-v1", 8)?;
    require_schema(&fields[0], COORDINATION_APPLY_REPORT_SCHEMA, "coordination apply report schema")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let manifest_ref = record_ref(&fields[2], "manifest")?;
    let final_state_ref = record_ref(&fields[3], "state")?;
    let receipt_refs = record_ref_sequence(&fields[4], "receipts")?;
    let assertion_refs = record_ref_sequence(&fields[5], "assertions")?;
    let evidence_refs = record_ref_sequence(&fields[6], "evidence")?;
    require_check(&parse_checks(&fields[7])?, "control-plane-apply-batch", "coordination apply report")?;
    Ok(CoordinationApplyReport {
        report_ref: canonical_hash(value)?,
        decision,
        manifest_ref,
        final_state_ref,
        receipt_refs,
        assertion_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn coordination_supported_services() -> Vec<String> {
    supported_services()
}

pub fn coordination_summary(value: &IoValue) -> Result<String> {
    if let Ok(report) = parse_coordination_apply_report(value) {
        return Ok(format!(
            "coordination apply report decision={} manifest={} state={} receipts={} assertions={} evidence={}",
            report.decision,
            report.manifest_ref,
            report.final_state_ref,
            report.receipt_refs.len(),
            report.assertion_refs.len(),
            report.evidence_refs.len()
        ));
    }
    if let Ok(batch) = parse_coordination_batch_envelope(value) {
        return Ok(format!(
            "coordination batch envelope ref={} requests={} compare_state={} policy_refs={}",
            batch.batch_ref,
            batch.request_refs.len(),
            batch.compare_state_ref.as_deref().unwrap_or("none"),
            batch.policy_refs.len()
        ));
    }
    if let Ok(receipt) = parse_coordination_receipt(value) {
        return Ok(format!(
            "coordination receipt decision={} service={} operation={} read_consistency={} request={} state={} diagnostics={}",
            receipt.decision,
            receipt.service,
            receipt.operation,
            receipt.read_consistency_mode,
            receipt.request_ref,
            receipt.state_ref,
            receipt.diagnostics.join(";")
        ));
    }
    if let Ok(request) = parse_coordination_request(value) {
        return Ok(format!(
            "coordination request service={} operation={} read_consistency={} key={} session={} operation_id={}",
            request.service,
            request.operation,
            request.read_consistency_mode,
            request.key,
            request.client_session,
            request.operation_id_ref
        ));
    }
    if let Ok(token) = parse_fencing_token(value) {
        return Ok(format!(
            "coordination fencing token key={} owner={} token={} commit={}",
            token.key, token.owner, token.token, token.commit_receipt_ref
        ));
    }
    if let Ok(manifest) = parse_coordination_service_manifest(value) {
        return Ok(format!(
            "coordination manifest service={} primitives={} control_group={}",
            manifest.service_id,
            manifest.services.join(","),
            manifest.control_group_ref
        ));
    }
    if let Ok(snapshot) = parse_coordination_state_snapshot(value) {
        return Ok(format!(
            "coordination state {} retention_refs={}",
            snapshot.state_ref,
            snapshot.retention_refs.len()
        ));
    }
    if let Ok(assertion) = parse_coordination_status_assertion(value) {
        return Ok(format!(
            "coordination assertion service={} key={} read_consistency={} state={} receipt={}",
            assertion.service,
            assertion.key,
            assertion.read_consistency_mode,
            assertion.state_ref,
            assertion.receipt_ref
        ));
    }
    Err(MoltenError::invalid_harness("unsupported coordination artifact"))
}

fn apply_coordination_read(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
) -> Result<CoordinationApplyResult> {
    let read =
        crate::raft_control_plane::read_control_registry(&crate::raft_control_plane::ControlRegistryReadInput {
            state: runtime.raft.state.value.clone(),
            group_ref: runtime.raft.manifest.manifest_ref.clone(),
            committed_term: runtime.raft.term,
            committed_index: runtime.raft.committed_index,
            read_index: runtime.raft.committed_index,
            read_consistency_mode: request.read_consistency_mode.clone(),
            namespace: coordination_namespace(&request.service),
            name: request.key.clone(),
            authority_refs: request.authority_refs.clone(),
            resource_refs: request.resource_refs.clone(),
        })?;
    let snapshot = snapshot_from_state(&runtime.state)?;
    let decision = if read.decision == "pass" { "pass" } else { "deny" };
    let fact = status_fact_for(&runtime.state, &runtime.manifest, &request.service, &request.key)?;
    let fact = engine_status_fact(&runtime.raft.manifest, active_engine_epoch(runtime), &fact);
    let (assertion_value, diagnostics) = read_assertion_value(ReadAssertionInput {
        service: &request.service,
        key: &request.key,
        fact: &fact,
        snapshot_ref: &snapshot.state_ref,
        decision,
        read_consistency_mode: &request.read_consistency_mode,
        diagnostics: read.diagnostics.clone(),
    })?;
    let assertion = match assertion_value {
        Some(value) => Some(parse_coordination_status_assertion(&value)?),
        None => None,
    };
    let assertion_refs = assertion.as_ref().map_or_else(Vec::new, |item| vec![item.assertion_ref.clone()]);
    let receipt_value = coordination_receipt_value(ReceiptValueInput {
        decision,
        service: &request.service,
        operation: &request.operation,
        read_consistency_mode: &request.read_consistency_mode,
        request_ref: &request.request_ref,
        raft_receipt_ref: Some(&read.receipt_ref),
        token_ref: None,
        state_ref: &snapshot.state_ref,
        dataspace_assertion_refs: &assertion_refs,
        diagnostics: &diagnostics,
        checks: &[
            ("coordination-request-bound", "pass"),
            ("read-consistency-declared", "pass"),
            ("read-index-bound", if request.read_consistency_mode == READ_CONSISTENCY_LINEARIZABLE { "pass" } else { "diagnostic" }),
            ("local-stale-non-authoritative", if request.read_consistency_mode == READ_CONSISTENCY_LOCAL_STALE { "pass" } else { "diagnostic" }),
            ("control-plane-command", "pass"),
            ("normalized-consensus-evidence", "pass"),
            ("active-engine-epoch-bound", "pass"),
        ],
    })?;
    let receipt = parse_coordination_receipt(&receipt_value)?;
    let assertions = corrected_read_assertions(assertion, &fact, &receipt.receipt_ref)?;
    Ok(finish_read(ReadFinishInput {
        runtime,
        request,
        read,
        snapshot,
        receipt,
        assertions,
    }))
}

struct ReadAssertionInput<'a> {
    service: &'a str,
    key: &'a str,
    fact: &'a IoValue,
    snapshot_ref: &'a str,
    decision: &'a str,
    read_consistency_mode: &'a str,
    diagnostics: Vec<String>,
}

fn read_assertion_value(input: ReadAssertionInput<'_>) -> Result<(Option<IoValue>, Vec<String>)> {
    let mut diagnostics = input.diagnostics;
    if input.decision == "pass" {
        let placeholder_receipt_ref = fixture_ref("coordination-read-placeholder");
        let value = coordination_status_assertion_value(StatusAssertionInput {
            service: input.service,
            key: input.key,
            read_consistency_mode: input.read_consistency_mode,
            fact: input.fact,
            state_ref: input.snapshot_ref,
            receipt_ref: &placeholder_receipt_ref,
        })?;
        return Ok((Some(value), diagnostics));
    }
    if diagnostics.is_empty() {
        diagnostics.push_limited(
            "coordination read-index denied".to_string(),
            MAX_COORDINATION_DIAGNOSTICS,
            "coordination diagnostics",
        )?;
    }
    Ok((None, diagnostics))
}

fn corrected_read_assertions(
    assertion: Option<CoordinationStatusAssertion>,
    fact: &IoValue,
    receipt_ref: &str,
) -> Result<Vec<CoordinationStatusAssertion>> {
    if let Some(assertion) = assertion {
        let corrected = coordination_status_assertion_value(StatusAssertionInput {
            service: &assertion.service,
            key: &assertion.key,
            read_consistency_mode: &assertion.read_consistency_mode,
            fact,
            state_ref: &assertion.state_ref,
            receipt_ref,
        })?;
        return Ok(vec![parse_coordination_status_assertion(&corrected)?]);
    }
    Ok(Vec::new())
}

struct ReadFinishInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: CoordinationRequest,
    read: crate::raft_control_plane::RaftReadReceipt,
    snapshot: CoordinationStateSnapshot,
    receipt: CoordinationReceipt,
    assertions: Vec<CoordinationStatusAssertion>,
}

fn finish_read(input: ReadFinishInput<'_>) -> CoordinationApplyResult {
    let ReadFinishInput {
        runtime,
        request,
        read,
        snapshot,
        receipt,
        assertions,
    } = input;
    let evidence_values = evidence_values_for(EvidenceValuesInput {
        request: &request,
        receipt: &receipt,
        token: None,
        snapshot: &snapshot,
        assertions: &assertions,
        read: Some(&read),
    });
    let result = CoordinationApplyResult {
        receipt: receipt.clone(),
        request: request.clone(),
        token: None,
        state_snapshot: snapshot,
        assertions: assertions.clone(),
        raft_commit_ref: None,
        raft_read_receipt: Some(read),
        evidence_values,
    };
    runtime.receipts.push(receipt);
    runtime.assertions.extend(assertions);
    runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    result
}

type Proposal = crate::raft_control_plane::ControlRegistryProposal;

struct ChangeInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: &'a CoordinationRequest,
    snapshot: &'a CoordinationStateSnapshot,
}

struct PartsInput<'a> {
    prepared: &'a PreparedMutation,
    request: &'a CoordinationRequest,
    manifest: &'a CoordinationServiceManifest,
    engine_manifest: &'a crate::raft_control_plane::RaftGroupManifest,
    engine_epoch: u64,
    snapshot: &'a CoordinationStateSnapshot,
    proposal_ref: &'a str,
}

struct PassReceiptInput<'a> {
    request: &'a CoordinationRequest,
    proposal_ref: &'a str,
    token_ref: Option<&'a str>,
    state_ref: &'a str,
    assertion_refs: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct SuccessParts {
    token: Option<FencingToken>,
    receipt: CoordinationReceipt,
    assertion: CoordinationStatusAssertion,
}

struct ValuesInput<'a> {
    proposal: &'a Proposal,
    request: &'a CoordinationRequest,
    receipt: &'a CoordinationReceipt,
    token: Option<&'a FencingToken>,
    snapshot: &'a CoordinationStateSnapshot,
    assertion: &'a CoordinationStatusAssertion,
}

struct SuccessInput<'a> {
    runtime: &'a mut CoordinationRuntime,
    request: CoordinationRequest,
    prepared: PreparedMutation,
    snapshot: CoordinationStateSnapshot,
    proposal: Proposal,
}
