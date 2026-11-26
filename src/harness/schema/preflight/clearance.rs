use preserves::ValueImpl;

pub fn parse_executor_preflights(
    value: &preserves::IOValue,
) -> crate::error::Result<super::observed::ExecutorPreflightsEvidence> {
    let preflights = super::fields::simple_record(value, "executor-preflights-v1", 2)?;
    let schema = super::fields::required_string(&preflights[0], "executor preflights schema")?;
    if schema != crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported executor preflights schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA
        )));
    }
    let preflight_values = super::fields::required_sequence(&preflights[1], "executor preflight entries")?;
    let mut entries = Vec::with_capacity(preflight_values.len());
    for preflight in preflight_values.iter() {
        entries.push(parse_executor_preflight(&crate::preserves_rail::value_to_iovalue(&preflight))?);
    }
    Ok(super::observed::ExecutorPreflightsEvidence {
        value: value.clone(),
        preflights: entries,
    })
}

fn parse_executor_preflight(
    value: &preserves::IOValue,
) -> crate::error::Result<super::observed::ExecutorPreflightEvidence> {
    let preflight = value
        .collect_simple_record("executor-preflight-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <executor-preflight-v1 ...>"))?;
    let arity = preflight.fields_iter().count();
    if arity != 8 && arity != 9 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <executor-preflight-v1 ...> with arity 8 or 9, got {arity}"
        )));
    }
    let schema = super::fields::required_string(&preflight[0], "executor preflight schema")?;
    if schema != crate::preserves_rail::RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported executor preflight schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA
        )));
    }
    let actor_id = super::fields::required_record_string(&preflight[1], "actor", "executor preflight actor")?;
    let kind = super::roster::parse_actor_kind(&super::fields::required_record_string(
        &preflight[2],
        "kind",
        "executor preflight kind",
    )?)?;
    let artifact_ref = optional_executor_hash(&preflight[3], "artifact-ref", "executor artifact ref")?;
    let sandbox_ref = super::fields::required_record_hash(&preflight[4], "sandbox-ref", "executor sandbox ref")?;
    let allowed_hostcalls = super::fields::required_record_string_sequence(
        &preflight[5],
        "allowed-hostcalls",
        "executor allowed hostcalls",
    )?;
    let conformance_refs = super::fields::required_record_hash_sequence(
        &preflight[6],
        "conformance-suites",
        "executor conformance suites",
    )?;
    let (executor_receipts, checks_index) = if arity == 9 {
        let receipts =
            super::fields::required_record_iovalue_sequence(&preflight[7], "executor-receipts", "executor receipts")?;
        (receipts, 8)
    } else {
        (Vec::new(), 7)
    };
    let steel_review = parse_optional_steel_review_receipt(&executor_receipts)?;
    let wasm_inspection = parse_optional_wasm_inspection_receipt(&executor_receipts)?;
    let checks = parse_executor_preflight_checks(&preflight[checks_index])?;
    require_executor_preflight_check(&checks, "actor-kind-binding")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-binding")?;
    require_executor_preflight_check(&checks, "no-ambient-executor-io")?;
    Ok(super::observed::ExecutorPreflightEvidence {
        value: value.clone(),
        actor_id,
        kind,
        artifact_ref,
        sandbox_ref,
        allowed_hostcalls,
        conformance_refs,
        executor_receipts,
        steel_review,
        wasm_inspection,
        checks,
    })
}

fn parse_optional_steel_review_receipt(
    receipts: &[preserves::IOValue],
) -> crate::error::Result<Option<super::observed::SteelReviewReceipt>> {
    let mut parsed = None;
    for receipt in receipts {
        if receipt.collect_simple_record("steel-review-receipt-v1", None).is_some() {
            if parsed.is_some() {
                return Err(crate::error::Failure::invalid_harness(
                    "duplicate Steel review receipt in executor preflight",
                ));
            }
            parsed = Some(parse_steel_review_receipt(receipt)?);
        }
    }
    Ok(parsed)
}

fn parse_steel_review_receipt(value: &preserves::IOValue) -> crate::error::Result<super::observed::SteelReviewReceipt> {
    let receipt = super::fields::simple_record(value, "steel-review-receipt-v1", 6)?;
    let schema = super::fields::required_string(&receipt[0], "Steel review receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Steel review receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&receipt[1], "decision", "Steel review receipt decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Steel review receipt decision {decision}"
        )));
    }
    let source_ref = super::fields::required_record_hash(&receipt[2], "source-ref", "Steel review receipt source ref")?;
    let callable = super::fields::required_record_string(&receipt[3], "callable", "Steel review receipt callable")?;
    let allowed_hostcalls = super::fields::required_record_string_sequence(
        &receipt[4],
        "allowed-hostcalls",
        "Steel review receipt allowed hostcalls",
    )?;
    let checks = parse_executor_preflight_checks(&receipt[5])?;
    require_executor_preflight_check(&checks, "source-ref-binding")?;
    require_executor_preflight_check(&checks, "reviewed-callable")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-contract")?;
    require_executor_preflight_check(&checks, "no-ambient-steel-io")?;
    Ok(super::observed::SteelReviewReceipt {
        value: value.clone(),
        source_ref,
        callable,
        allowed_hostcalls,
        checks,
    })
}

fn parse_optional_wasm_inspection_receipt(
    receipts: &[preserves::IOValue],
) -> crate::error::Result<Option<super::observed::WasmInspectionReceipt>> {
    let mut parsed = None;
    for receipt in receipts {
        if receipt.collect_simple_record("wasm-inspection-receipt-v1", None).is_some() {
            if parsed.is_some() {
                return Err(crate::error::Failure::invalid_harness(
                    "duplicate Wasm inspection receipt in executor preflight",
                ));
            }
            parsed = Some(parse_wasm_inspection_receipt(receipt)?);
        }
    }
    Ok(parsed)
}

fn parse_wasm_inspection_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::observed::WasmInspectionReceipt> {
    let receipt = super::fields::simple_record(value, "wasm-inspection-receipt-v1", 8)?;
    let schema = super::fields::required_string(&receipt[0], "Wasm inspection receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Wasm inspection receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&receipt[1], "decision", "Wasm inspection receipt decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Wasm inspection receipt decision {decision}"
        )));
    }
    let module_ref =
        super::fields::required_record_hash(&receipt[2], "module-ref", "Wasm inspection receipt module ref")?;
    let module_kind =
        super::fields::required_record_string(&receipt[3], "module-kind", "Wasm inspection receipt module kind")?;
    if !matches!(module_kind.as_str(), "core-module" | "component") {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Wasm inspection module kind {module_kind}"
        )));
    }
    let import_values = super::fields::required_record_sequence(&receipt[4], "imports", "Wasm inspection imports")?;
    let mut imports = Vec::with_capacity(import_values.len());
    for import_value in import_values {
        imports.push(parse_wasm_import(&crate::preserves_rail::value_to_iovalue(&import_value))?);
    }
    let wit_ref = super::fields::required_record_hash(&receipt[5], "wit-ref", "Wasm inspection WIT ref")?;
    let allowed_hostcalls = super::fields::required_record_string_sequence(
        &receipt[6],
        "allowed-hostcalls",
        "Wasm inspection allowed hostcalls",
    )?;
    let checks = parse_executor_preflight_checks(&receipt[7])?;
    require_executor_preflight_check(&checks, "module-ref-binding")?;
    require_executor_preflight_check(&checks, "wasmparser-validated")?;
    require_executor_preflight_check(&checks, "deny-by-default-wasi")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-contract")?;
    require_executor_preflight_check(&checks, "wit-interface-binding")?;
    Ok(super::observed::WasmInspectionReceipt {
        value: value.clone(),
        module_ref,
        module_kind,
        imports,
        wit_ref,
        allowed_hostcalls,
        checks,
    })
}

fn parse_wasm_import(value: &preserves::IOValue) -> crate::error::Result<super::observed::WasmImportEvidence> {
    let import = super::fields::simple_record(value, "import", 3)?;
    Ok(super::observed::WasmImportEvidence {
        module: super::fields::required_string(&import[0], "Wasm import module")?,
        name: super::fields::required_string(&import[1], "Wasm import name")?,
        kind: super::fields::required_string(&import[2], "Wasm import kind")?,
    })
}

fn optional_executor_hash(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    field: &str,
) -> crate::error::Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = super::fields::simple_record(&value, label, 1)?;
    let parsed = super::fields::optional_request_string(&record[0], field)?;
    if let Some(hash) = parsed.as_deref() {
        super::fields::required_hash(&crate::preserves_rail::string(hash), field)?;
    }
    Ok(parsed)
}

pub(super) fn parse_executor_preflight_checks(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks_record = super::fields::simple_record(&value, "checks", 1)?;
    let check_values = super::fields::required_sequence(&checks_record[0], "executor preflight checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(&check_value);
        let check = super::fields::simple_record(&check_value, "check", 2)?;
        let name = super::fields::required_string(&check[0], "executor preflight check name")?;
        let status = super::fields::required_string(&check[1], "executor preflight check status")?;
        if status != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!(
                "executor preflight check {name} status is {status}"
            )));
        }
        checks.push(name);
    }
    Ok(checks)
}

pub(super) fn require_executor_preflight_check(checks: &[String], expected: &str) -> crate::error::Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("executor preflight missing {expected} check")))
    }
}
