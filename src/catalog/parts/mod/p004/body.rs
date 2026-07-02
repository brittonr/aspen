
fn retention_plan_apply_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(plan) = crate::retention::parse_gc_plan(value) {
        return Ok(Some(vec![
            "retention-gc:plan".to_string(),
            "retention-gc-stage:plan".to_string(),
            format!("retention-gc-decision:{}", plan.decision),
            format!("retention-gc-subsystem:{}", plan.subsystem),
            format!("retention-gc-action:{}", plan.action),
            format!("retention-gc-object:{}", plan.object_ref),
            format!("retention-gc-class:{}", plan.retention_class),
            format!("retention-gc-plan:{}", plan.plan_ref),
        ]));
    }
    if let Ok(apply) = crate::retention::parse_gc_apply(value) {
        let mut classifications = vec![
            "retention-gc:apply".to_string(),
            "retention-gc-stage:apply".to_string(),
            format!("retention-gc-decision:{}", apply.decision),
            format!("retention-gc-subsystem:{}", apply.subsystem),
            format!("retention-gc-action:{}", apply.action),
            format!("retention-gc-object:{}", apply.object_ref),
            format!("retention-gc-class:{}", apply.retention_class),
            format!("retention-gc-plan:{}", apply.plan_ref),
            format!("retention-gc-apply:{}", apply.apply_ref),
        ];
        push_optional_classification(
            &mut classifications,
            "retention-gc-receipt",
            apply.retention_receipt_ref.as_deref(),
        )?;
        push_optional_classification(&mut classifications, "retention-gc-tombstone", apply.tombstone_ref.as_deref())?;
        return Ok(Some(classifications));
    }
    Ok(None)
}

fn retention_execute_audit_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(execute) = crate::retention::parse_gc_execution_gate(value) {
        let mut classifications = vec![
            "retention-gc:execute".to_string(),
            "retention-gc-stage:execute".to_string(),
            format!("retention-gc-decision:{}", execute.decision),
            format!("retention-gc-subsystem:{}", execute.subsystem),
            format!("retention-gc-action:{}", execute.action),
            format!("retention-gc-object:{}", execute.object_ref),
            format!("retention-gc-class:{}", execute.retention_class),
            format!("retention-gc-execution:{}", execute.execution_ref),
        ];
        push_optional_classification(&mut classifications, "retention-gc-plan", execute.plan_ref.as_deref())?;
        push_optional_classification(&mut classifications, "retention-gc-apply", execute.apply_ref.as_deref())?;
        push_optional_classification(
            &mut classifications,
            "retention-gc-receipt",
            execute.retention_receipt_ref.as_deref(),
        )?;
        push_optional_classification(&mut classifications, "retention-gc-tombstone", execute.tombstone_ref.as_deref())?;
        return Ok(Some(classifications));
    }
    if let Ok(audit) = crate::retention::parse_gc_audit(value) {
        let mut classifications = vec![
            "retention-gc:audit".to_string(),
            "retention-gc-stage:audit".to_string(),
            format!("retention-gc-decision:{}", audit.decision),
            format!("retention-gc-subsystem:{}", audit.subsystem),
            format!("retention-gc-action:{}", audit.action),
            format!("retention-gc-object:{}", audit.object_ref),
            format!("retention-gc-class:{}", audit.retention_class),
            format!("retention-gc-execution:{}", audit.execution_ref),
        ];
        push_optional_classification(&mut classifications, "retention-gc-plan", audit.plan_ref.as_deref())?;
        push_optional_classification(&mut classifications, "retention-gc-apply", audit.apply_ref.as_deref())?;
        push_optional_classification(
            &mut classifications,
            "retention-gc-receipt",
            audit.retention_receipt_ref.as_deref(),
        )?;
        push_optional_classification(&mut classifications, "retention-gc-tombstone", audit.tombstone_ref.as_deref())?;
        return Ok(Some(classifications));
    }
    Ok(None)
}

fn candidate_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(explain) = crate::retention::parse_candidate_explain(value) {
        let mut classifications = vec![
            "retention:explain".to_string(),
            "retention-candidate:explain".to_string(),
            format!("retention-object:{}", explain.object_ref),
            format!("retention-explain-pins:{}", explain.pin_refs.len()),
            format!("retention-explain-admissions:{}", explain.admission_refs.len()),
            format!("retention-explain-clearances:{}", explain.remote_clearance_refs.len()),
            format!("retention-explain-plans:{}", explain.gc_plan_refs.len()),
            format!("retention-explain-applies:{}", explain.gc_apply_refs.len()),
            format!("retention-explain-executes:{}", explain.gc_execution_refs.len()),
            format!("retention-explain-audits:{}", explain.gc_audit_refs.len()),
        ];
        push_optional_classification(&mut classifications, "retention-kind", explain.object_kind.as_deref())?;
        push_optional_classification(&mut classifications, "retention-class", explain.retention_class.as_deref())?;
        push_optional_classification(&mut classifications, "retention-action", explain.action.as_deref())?;
        push_optional_classification(&mut classifications, "retention-subsystem", explain.subsystem.as_deref())?;
        return Ok(Some(classifications));
    }
    if let Ok(bundle) = crate::retention::parse_candidate_bundle(value) {
        let mut classifications = vec![
            "retention:bundle".to_string(),
            "retention-candidate:bundle".to_string(),
            format!("retention-object:{}", bundle.object_ref),
            format!("retention-bundle-artifacts:{}", bundle.artifact_refs.len()),
            format!("retention-bundle-plans:{}", bundle.gc_plan_refs.len()),
            format!("retention-bundle-applies:{}", bundle.gc_apply_refs.len()),
            format!("retention-bundle-executes:{}", bundle.gc_execution_refs.len()),
            format!("retention-bundle-audits:{}", bundle.gc_audit_refs.len()),
        ];
        push_optional_classification(&mut classifications, "retention-kind", bundle.object_kind.as_deref())?;
        push_optional_classification(&mut classifications, "retention-class", bundle.retention_class.as_deref())?;
        push_optional_classification(&mut classifications, "retention-action", bundle.action.as_deref())?;
        push_optional_classification(&mut classifications, "retention-subsystem", bundle.subsystem.as_deref())?;
        return Ok(Some(classifications));
    }
    Ok(None)
}

fn retention_tail_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(profile) = crate::retention::parse_candidate_bundle_profile(value) {
        return Ok(Some(vec![
            "retention:bundle-profile".to_string(),
            "retention-candidate:bundle-profile".to_string(),
            format!("retention-bundle-profile:{}", profile.profile),
            format!("retention-bundle-decision:{}", profile.decision),
            format!("retention-bundle:{}", profile.bundle_ref),
            format!("retention-bundle-markers:{}", profile.marker_refs.len()),
        ]));
    }
    if let Ok(verify) = crate::retention::parse_candidate_bundle_verify(value) {
        let mut classifications = vec![
            "retention:bundle-verify".to_string(),
            "retention-candidate:bundle-verify".to_string(),
            format!("retention-bundle-decision:{}", verify.decision),
            format!("retention-object:{}", verify.object_ref),
            format!("retention-bundle:{}", verify.bundle_ref),
            format!("retention-explain:{}", verify.explain_ref),
            format!("retention-bundle-artifacts:{}", verify.artifact_refs.len()),
            format!("retention-bundle-files:{}", verify.file_refs.len()),
        ];
        push_optional_classification(&mut classifications, "retention-kind", verify.object_kind.as_deref())?;
        push_optional_classification(&mut classifications, "retention-class", verify.retention_class.as_deref())?;
        push_optional_classification(&mut classifications, "retention-action", verify.action.as_deref())?;
        push_optional_classification(&mut classifications, "retention-subsystem", verify.subsystem.as_deref())?;
        return Ok(Some(classifications));
    }
    if let Ok(receipt) = crate::retention::parse_receipt(value) {
        return Ok(Some(vec![
            "retention:receipt".to_string(),
            format!("retention-decision:{}", receipt.decision),
            format!("retention-action:{}", receipt.action),
            format!("retention-object:{}", receipt.object_ref),
            format!("retention-pins:{}", receipt.pin_refs.len()),
        ]));
    }
    if let Ok(tombstone) = crate::retention::parse_tombstone(value) {
        return Ok(Some(vec![
            "retention:tombstone".to_string(),
            format!("retention-action:{}", tombstone.action),
            format!("retention-object:{}", tombstone.object_ref),
            format!("retention-class:{}", tombstone.retention_class),
        ]));
    }
    Ok(None)
}

fn lifecycle_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if crate::transcripts::parse_transcript_artifact(value).is_ok() {
        return Ok(Some(vec![
            "transcript:artifact".to_string(),
            "transcript-status:document".to_string(),
        ]));
    }
    if let Ok(plan) = crate::upgrades::parse_upgrade_plan(value) {
        return Ok(Some(vec![
            "upgrade:plan".to_string(),
            "upgrade-status:planned".to_string(),
            format!("upgrade-session:{}", plan.session_id),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("upgrade-receipt-v1", Some(8)) {
        let decision = record_string(&fields[2], "decision")?;
        return Ok(Some(vec![
            "upgrade:receipt".to_string(),
            format!("upgrade-status:{decision}"),
            format!("receipt-decision:{decision}"),
        ]));
    }
    Ok(None)
}

fn provenance_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Ok(record) = crate::provenance::parse_record(value) {
        return Ok(Some(vec![
            "provenance:record".to_string(),
            format!("provenance-trust-state:{}", record.trust_state),
            format!("provenance-artifact:{}", record.artifact_ref),
            format!("provenance-build-records:{}", record.build_record_refs.len()),
        ]));
    }
    if let Ok(record) = crate::provenance::parse_build_record(value) {
        return Ok(Some(vec![
            "provenance:build-record".to_string(),
            format!("provenance-expected-artifact:{}", record.expected_artifact_ref),
            format!("provenance-build-sources:{}", record.source_refs.len()),
            format!("provenance-build-toolchains:{}", record.toolchain_refs.len()),
        ]));
    }
    if let Ok(receipt) = crate::provenance::parse_build_verification_receipt(value) {
        return Ok(Some(vec![
            "provenance:build-verify-receipt".to_string(),
            format!("provenance-build-decision:{}", receipt.decision),
            format!("provenance-expected-artifact:{}", receipt.expected_artifact_ref),
            format!("provenance-actual-artifact:{}", receipt.actual_artifact_ref),
            format!("receipt-decision:{}", receipt.decision),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(10)) {
        require_schema(&fields[0], crate::preserves_rail::PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        let decision = record_string(&fields[1], "decision")?;
        let operation = record_string(&fields[2], "operation")?;
        let profile = record_string(&fields[3], "profile")?;
        let trust_state = record_string(&fields[5], "trust-state")?;
        let build_verification_count = record_sequence_len(&fields[9], "build-verifications")?;
        return Ok(Some(vec![
            "provenance:receipt".to_string(),
            format!("provenance-decision:{decision}"),
            format!("provenance-operation:{operation}"),
            format!("provenance-profile:{profile}"),
            format!("provenance-trust-state:{trust_state}"),
            format!("provenance-build-verifications:{build_verification_count}"),
            format!("receipt-operation:{operation}"),
            format!("receipt-decision:{decision}"),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(9)) {
        require_schema(&fields[0], crate::preserves_rail::PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        let decision = record_string(&fields[1], "decision")?;
        let operation = record_string(&fields[2], "operation")?;
        let profile = record_string(&fields[3], "profile")?;
        let trust_state = record_string(&fields[5], "trust-state")?;
        return Ok(Some(vec![
            "provenance:receipt".to_string(),
            format!("provenance-decision:{decision}"),
            format!("provenance-operation:{operation}"),
            format!("provenance-profile:{profile}"),
            format!("provenance-trust-state:{trust_state}"),
            format!("receipt-operation:{operation}"),
            format!("receipt-decision:{decision}"),
        ]));
    }
    Ok(None)
}

fn octet_evidence_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(fields) = value.collect_simple_record("octet-structured-findings-v1", Some(7)) {
        let counts = value_to_iovalue(&fields[4]);
        let count_fields = simple_record(&counts, "counts", 4)?;
        let total = record_u64(&count_fields[0], "total")?;
        let parsed = record_u64(&count_fields[1], "parsed")?;
        let unkeyed = record_u64(&count_fields[2], "unkeyed")?;
        let critical = record_u64(&count_fields[3], "critical")?;
        return Ok(Some(vec![
            "octet-structured-findings:summary-index".to_string(),
            format!("octet-findings-total:{total}"),
            format!("octet-findings-parsed:{parsed}"),
            format!("octet-findings-unkeyed:{unkeyed}"),
            format!("octet-findings-critical:{critical}"),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("octet-fingerprint-evidence-v1", Some(7)) {
        let source_paths = record_sequence_len(&fields[3], "source-paths")?;
        let object_count = record_u64(&fields[4], "object-count")?;
        let pure_cache_blocked = record_u64(&fields[5], "pure-cache-blocked")?;
        return Ok(Some(vec![
            "octet-fingerprint-evidence:object-corpus".to_string(),
            format!("octet-fingerprint-source-paths:{source_paths}"),
            format!("octet-fingerprint-object-count:{object_count}"),
            format!("octet-fingerprint-pure-cache-blocked:{pure_cache_blocked}"),
        ]));
    }
    Ok(None)
}
