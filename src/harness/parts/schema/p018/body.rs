
fn parse_wasm_inspection_receipt(value: &IoValue) -> Result<WasmInspectionReceipt> {
    let receipt = simple_record(value, "wasm-inspection-receipt-v1", 8)?;
    let schema = required_string(&receipt[0], "Wasm inspection receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Wasm inspection receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Wasm inspection receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Wasm inspection receipt decision {decision}")));
    }
    let module_ref = required_record_hash(&receipt[2], "module-ref", "Wasm inspection receipt module ref")?;
    let module_kind = required_record_string(&receipt[3], "module-kind", "Wasm inspection receipt module kind")?;
    if !matches!(module_kind.as_str(), "core-module" | "component") {
        return Err(MoltenError::invalid_harness(format!("unsupported Wasm inspection module kind {module_kind}")));
    }
    let import_values = required_record_sequence(&receipt[4], "imports", "Wasm inspection imports")?;
    let mut imports = Vec::with_capacity(import_values.len());
    for import_value in import_values {
        imports.push(parse_wasm_import(&value_to_iovalue(&import_value))?);
    }
    let wit_ref = required_record_hash(&receipt[5], "wit-ref", "Wasm inspection WIT ref")?;
    let allowed_hostcalls =
        required_record_string_sequence(&receipt[6], "allowed-hostcalls", "Wasm inspection allowed hostcalls")?;
    let checks = parse_executor_preflight_checks(&receipt[7])?;
    require_executor_preflight_check(&checks, "module-ref-binding")?;
    require_executor_preflight_check(&checks, "wasmparser-validated")?;
    require_executor_preflight_check(&checks, "deny-by-default-wasi")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-contract")?;
    require_executor_preflight_check(&checks, "wit-interface-binding")?;
    Ok(WasmInspectionReceipt {
        value: value.clone(),
        module_ref,
        module_kind,
        imports,
        wit_ref,
        allowed_hostcalls,
        checks,
    })
}

fn parse_wasm_import(value: &IoValue) -> Result<WasmImportEvidence> {
    let import = simple_record(value, "import", 3)?;
    Ok(WasmImportEvidence {
        module: required_string(&import[0], "Wasm import module")?,
        name: required_string(&import[1], "Wasm import name")?,
        kind: required_string(&import[2], "Wasm import kind")?,
    })
}

fn optional_executor_hash(value: &Value<IoValue>, label: &str, field: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let parsed = optional_request_string(&record[0], field)?;
    if let Some(hash) = parsed.as_deref() {
        required_hash(&string(hash), field)?;
    }
    Ok(parsed)
}

fn parse_executor_preflight_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "executor preflight checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "executor preflight check name")?;
        let status = required_string(&check[1], "executor preflight check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("executor preflight check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_executor_preflight_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("executor preflight missing {expected} check")))
    }
}

fn validate_denied_observation_events(position: usize, events: &[IoValue]) -> Result<()> {
    let mut has_rollback_event = false;
    for event in events {
        match event_boundary(event) {
            EventBoundary::EffectRequest | EventBoundary::EffectResponse => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied effect emitted effect request/response at observation {position}"
                )));
            }
            EventBoundary::SteelExecution => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied turn emitted Steel execution evidence at observation {position}"
                )));
            }
            EventBoundary::WasmExecution => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied turn emitted Wasm execution evidence at observation {position}"
                )));
            }
            EventBoundary::PolicyDecision => {
                return Err(MoltenError::invalid_harness(format!(
                    "duplicate admission decision at observation {position}"
                )));
            }
            EventBoundary::ActorInput
            | EventBoundary::HostcallRequest
            | EventBoundary::HostcallDecision
            | EventBoundary::RuntimePredicate
            | EventBoundary::ActorOutput => {}
            EventBoundary::Trace if is_turn_rolled_back(event) => {
                has_rollback_event = true;
            }
            EventBoundary::Trace if is_turn_journal(event) => {}
            EventBoundary::Trace => {
                return Err(MoltenError::invalid_harness(format!(
                    "denied turn committed action or non-rollback trace at observation {position}"
                )));
            }
        }
    }
    if !has_rollback_event {
        return Err(MoltenError::invalid_harness(format!(
            "denied turn missing rollback evidence at observation {position}"
        )));
    }
    Ok(())
}

fn is_turn_rolled_back(value: &IoValue) -> bool {
    value.collect_simple_record("turn-rolled-back", Some(2)).is_some()
}

fn is_turn_journal(value: &IoValue) -> bool {
    value.collect_simple_record("turn-journal-v1", None).is_some()
}

fn parse_admission_request(value: &Value<IoValue>) -> Result<super::core::AdmissionRequest> {
    let request_value = value_to_iovalue(value);
    let request = simple_record(&request_value, "request", 5)?;
    Ok(super::core::AdmissionRequest {
        actor: required_string(&request[0], "admission request actor")?,
        action: parse_admission_action(&required_string(&request[1], "admission request action")?)?,
        target: optional_request_string(&request[2], "admission request target")?,
        value: optional_request_runtime_value(&request[3], "admission request value")?,
        upper: optional_request_u64(&request[4], "admission request upper")?,
    })
}

fn required_record_iovalue(value: &Value<IoValue>, label: &str, _field: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(value_to_iovalue(&record[0]))
}

fn parse_admission_authority(value: &Value<IoValue>) -> Result<AdmissionAuthorityEvidence> {
    let authority_value = value_to_iovalue(value);
    let authority = simple_record(&authority_value, "authority", 10)?;
    let source = required_record_string(&authority[0], "source", "admission authority source")?;
    let capability_ref = required_record_hash(&authority[1], "capability-ref", "admission authority capability ref")?;
    let authorized = required_record_bool(&authority[2], "authorized", "admission authority authorized")?;
    let grant_ref = optional_request_string(&authority[3], "admission authority grant ref")?;
    let request_ref = required_record_hash(&authority[4], "request-ref", "admission authority request ref")?;
    let proofset_ref = required_record_hash(&authority[5], "ucan-proofset-ref", "admission authority UCAN proofset ref")?;
    let ucan_verification_receipt_refs = required_record_hash_sequence(
        &authority[6],
        "ucan-verification-receipt-refs",
        "admission authority UCAN verification receipt refs",
    )?;
    let derived_grant_refs = required_record_hash_sequence(
        &authority[7],
        "derived-grant-refs",
        "admission authority derived grant refs",
    )?;
    let basalt_enforcement_receipt_ref = required_record_hash(
        &authority[8],
        "basalt-enforcement-receipt-ref",
        "admission authority Basalt enforcement receipt ref",
    )?;
    let basalt_enforcement_receipt_value = required_record_iovalue(
        &authority[9],
        "basalt-enforcement-receipt",
        "admission authority Basalt enforcement receipt",
    )?;
    let actual_receipt_ref = canonical_hash(&basalt_enforcement_receipt_value)?;
    if actual_receipt_ref != basalt_enforcement_receipt_ref {
        return Err(MoltenError::invalid_harness(
            "admission authority Basalt enforcement receipt ref does not match embedded receipt",
        ));
    }
    Ok(AdmissionAuthorityEvidence {
        source,
        capability_ref,
        authorized,
        grant_ref,
        request_ref,
        proofset_ref,
        ucan_verification_receipt_refs,
        derived_grant_refs,
        basalt_enforcement_receipt_ref,
        basalt_enforcement_receipt_value,
    })
}

fn parse_admission_decision(value: &Value<IoValue>) -> Result<crate::runtime::AdmissionDecision> {
    let decision_value = value_to_iovalue(value);
    let decision = simple_record(&decision_value, "decision", 2)?;
    let status = required_string(&decision[0], "admission decision status")?;
    let reason = required_string(&decision[1], "admission decision reason")?;
    match status.as_str() {
        "allow" => Ok(crate::runtime::AdmissionDecision::Allow { reason }),
        "deny" => Ok(crate::runtime::AdmissionDecision::Deny { reason }),
        other => Err(MoltenError::invalid_harness(format!("unknown admission decision status {other}"))),
    }
}

pub fn policy_value(policy: &crate::runtime::AdmissionPolicy) -> IoValue {
    record("policy-v1", vec![
        string(crate::preserves_rail::HARNESS_POLICY_SCHEMA),
        sequence(policy.deny_rules().iter().map(deny_rule_value).collect()),
    ])
}

pub fn policy_gate_value(policy: &crate::runtime::AdmissionPolicy) -> Result<IoValue> {
    let preflight = policy_preflight_material(policy)?;
    Ok(record("policy-gate-v1", vec![
        string(crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("policy-ref", vec![string(&preflight.policy_ref)]),
        preflight.nickel_source_value,
        preflight.nickel_contract_value,
        preflight.basalt_preflight_value,
        record("steel-predicates", vec![sequence(Vec::new())]),
        policy_gate_checks_value(),
    ]))
}

pub fn parse_policy_gate(value: &IoValue) -> Result<PolicyGateEvidence> {
    let gate = simple_record(value, "policy-gate-v1", 8)?;
    let schema = required_string(&gate[0], "policy gate schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "policy gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported policy gate decision {decision}")));
    }
    let policy_ref = required_record_hash(&gate[2], "policy-ref", "policy gate policy ref")?;
    let nickel_source = parse_nickel_source_evidence(&gate[3])?;
    let nickel_contract = parse_nickel_contract_evidence(&gate[4])?;
    let basalt_preflight = parse_basalt_policy_preflight_evidence(&gate[5])?;
    if nickel_source.policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("Nickel source policy ref does not match policy gate ref"));
    }
    if nickel_contract.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "Nickel contract normalized source ref does not match Nickel source evidence",
        ));
    }
    if basalt_preflight.policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("Basalt policy preflight policy ref does not match policy gate ref"));
    }
    if basalt_preflight.envelope_ref != nickel_contract.envelope_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt policy preflight envelope ref does not match Nickel contract envelope",
        ));
    }
    if basalt_preflight.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt policy preflight source ref does not match Nickel source evidence",
        ));
    }
    let steel_predicates = required_record_sequence(&gate[6], "steel-predicates", "policy gate Steel predicates")?;
    if !steel_predicates.is_empty() {
        return Err(MoltenError::invalid_harness(
            "Steel predicates require reviewed callable receipts and are disabled in local harness policy gates",
        ));
    }
    let checks = parse_policy_gate_checks(&gate[7])?;
    require_policy_gate_check(&checks, "policy-schema")?;
    require_policy_gate_check(&checks, "canonical-policy-snapshot")?;
    require_policy_gate_check(&checks, "nickel-static-boundary")?;
    require_policy_gate_check(&checks, "nickel-policy-source")?;
    require_policy_gate_check(&checks, "nickel-export-normalization")?;
    require_policy_gate_check(&checks, "basalt-preflight")?;
    require_policy_gate_check(&checks, "basalt-receipt-binding")?;
    require_policy_gate_check(&checks, "steel-predicate-review")?;
    Ok(PolicyGateEvidence {
        value: value.clone(),
        policy_ref,
        nickel_source_ref: nickel_source.source_ref,
        nickel_export_ref: nickel_source.export_ref,
        basalt_preflight_ref: basalt_preflight.receipt_ref,
        checks,
    })
}

pub fn validate_policy_gate_evidence(suite: &Suite, policy_gate: Option<&PolicyGateEvidence>) -> Result<()> {
    let policy_gate = policy_gate.ok_or_else(|| {
        MoltenError::invalid_harness("missing policy gate evidence; policy must pass preflight before side effects")
    })?;
    let expected_ref = canonical_hash(&policy_value(&suite.policy))?;
    if policy_gate.policy_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "policy gate ref mismatch: gate has {}, embedded policy hashes to {expected_ref}",
            policy_gate.policy_ref
        )));
    }
    let expected_gate = policy_gate_value(&suite.policy)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(&policy_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(MoltenError::invalid_harness(format!(
            "policy gate evidence does not match embedded suite policy preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

struct PolicyPreflightMaterial {
    policy_ref: String,
    nickel_source_value: IoValue,
    nickel_contract_value: IoValue,
    basalt_preflight_value: IoValue,
}

struct NickelSourceEvidence {
    source_ref: String,
    export_ref: String,
    policy_ref: String,
}
