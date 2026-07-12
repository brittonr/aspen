
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinOperation {
    pub pin: Pin,
    pub receipt: Receipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub receipt: Receipt,
    pub index: ReferenceIndex,
    pub tombstone: Option<Tombstone>,
}

pub fn class_profile_value(input: &ClassProfileInput) -> Result<IoValue> {
    validate_class_profile_input(input)?;
    let diagnostics = class_profile_diagnostics(input)?;
    Ok(crate::preserves_rail::record("retention-class-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_CLASS_SCHEMA),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(&input.class_name)]),
        crate::preserves_rail::record("minimum-age-seconds", vec![crate::preserves_rail::u64_value(
            input.minimum_age_seconds,
        )]),
        crate::preserves_rail::record("maximum-age-seconds", vec![optional_u64_value(input.maximum_age_seconds)]),
        crate::preserves_rail::record("deletion-authority", vec![crate::preserves_rail::string(
            &input.deletion_authority_ref,
        )]),
        crate::preserves_rail::record("policy", vec![strings_sequence(&input.policy_refs)]),
        crate::preserves_rail::record("capabilities", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("secret-redaction-hook", vec![crate::preserves_rail::string(pass_or_deny(
                input.has_secret_redaction_hook,
            ))]),
            crate::preserves_rail::record("remote-gc-plan", vec![crate::preserves_rail::string(pass_or_deny(
                input.has_remote_gc_plan,
            ))]),
            crate::preserves_rail::record("compaction", vec![crate::preserves_rail::string(pass_or_deny(
                input.can_compact,
            ))]),
        ])]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("class-known", "pass"),
            ("policy-bound", "pass"),
            ("mutable-name-not-gc-proof", "pass"),
        ]),
    ]))
}

pub fn parse_class_profile(value: &IoValue) -> Result<ClassProfile> {
    let fields = value
        .collect_simple_record("retention-class-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-class-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_CLASS_SCHEMA, "retention class schema")?;
    let class_name = record_string(&fields[1], "class")?;
    let minimum_age_seconds = record_u64(&fields[2], "minimum-age-seconds")?;
    let maximum_age_seconds = record_optional_u64(&fields[3], "maximum-age-seconds")?;
    let deletion_authority_ref = record_ref(&fields[4], "deletion-authority")?;
    let policy_refs = record_ref_sequence(&fields[5], "policy")?;
    let diagnostics = record_string_sequence(&fields[7], "diagnostics")?;
    validate_class(&class_name)?;
    require_check(&parse_checks(&fields[8])?, "mutable-name-not-gc-proof", "retention class profile")?;
    Ok(ClassProfile {
        profile_ref: crate::preserves_rail::canonical_hash(value)?,
        class_name,
        minimum_age_seconds,
        maximum_age_seconds,
        deletion_authority_ref,
        policy_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn pin_value(input: &PinInput) -> Result<IoValue> {
    validate_pin_input(input)?;
    let authority_status = if input.has_authority { "pass" } else { "deny" };
    Ok(crate::preserves_rail::record("retention-pin-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_PIN_SCHEMA),
        object_value(&input.object_ref, &input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(&input.retention_class)]),
        crate::preserves_rail::record("source", vec![crate::preserves_rail::string(&input.source)]),
        crate::preserves_rail::record("reason", vec![crate::preserves_rail::string(&input.reason)]),
        crate::preserves_rail::record("owner", vec![crate::preserves_rail::string(&input.owner_ref)]),
        crate::preserves_rail::record("expiry", vec![optional_ref_value(input.expiry_ref.as_deref())]),
        crate::preserves_rail::record("policy", vec![strings_sequence(&input.policy_refs)]),
        crate::preserves_rail::record("evidence", vec![strings_sequence(&input.evidence_refs)]),
        checks_value(&[
            ("object-ref-bound", "pass"),
            ("pin-source-bound", "pass"),
            ("authority-bound", authority_status),
            ("mutable-name-not-gc-proof", "pass"),
        ]),
    ]))
}

pub fn parse_pin(value: &IoValue) -> Result<Pin> {
    let fields = value
        .collect_simple_record("retention-pin-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-pin-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RETENTION_PIN_SCHEMA, "retention pin schema")?;
    let (object_ref, object_kind) = parse_object_value(&fields[1])?;
    let retention_class = record_string(&fields[2], "class")?;
    let source = record_string(&fields[3], "source")?;
    let reason = record_string(&fields[4], "reason")?;
    let owner_ref = record_ref(&fields[5], "owner")?;
    let expiry_ref = record_optional_ref(&fields[6], "expiry")?;
    let policy_refs = record_ref_sequence(&fields[7], "policy")?;
    let evidence_refs = record_ref_sequence(&fields[8], "evidence")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "object-ref-bound", "retention pin")?;
    require_check(&checks, "pin-source-bound", "retention pin")?;
    validate_class(&retention_class)?;
    validate_pin_source(&source)?;
    Ok(Pin {
        pin_ref: crate::preserves_rail::canonical_hash(value)?,
        object_ref,
        object_kind,
        retention_class,
        source,
        reason,
        owner_ref,
        expiry_ref,
        policy_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn pin_object(root: &Path, input: PinInput) -> Result<PinOperation> {
    let root = open_capability_retention_root(root)?;
    pin_object_with_root(&root, input)
}

pub fn pin_object_with_root(root: &CapabilityRetentionRoot, input: PinInput) -> Result<PinOperation> {
    ensure_store_with_root(root)?;
    let pin_value = pin_value(&input)?;
    let pin = parse_pin(&pin_value)?;
    write_store_value_with_root(root, &capability_ref_path(PIN_DIR, &pin.pin_ref)?, &pin.value)?;
    let index = reference_index_for_object_with_root(ReferenceIndexForObjectInput {
        root,
        object_ref: &pin.object_ref,
        object_kind: &pin.object_kind,
        retained_refs: &[],
        remote_refs: &[],
        is_complete: true,
    })?;
    let diagnostics = if input.has_authority {
        Vec::new()
    } else {
        vec!["pin-authority-missing".to_string()]
    };
    let decision = if input.has_authority { "pass" } else { "deny" };
    let receipt = build_receipt(ReceiptBuildInput {
        decision,
        action: ACTION_PIN,
        object_ref: &pin.object_ref,
        object_kind: &pin.object_kind,
        retention_class: &pin.retention_class,
        requester_ref: &pin.owner_ref,
        index_ref: &index.index_ref,
        pin_refs: std::slice::from_ref(&pin.pin_ref),
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: &pin.policy_refs,
        evidence_refs: &pin.evidence_refs,
        tombstone_ref: None,
        diagnostics: &diagnostics,
    })?;
    write_store_value_with_root(root, &capability_ref_path(RECEIPT_DIR, &receipt.receipt_ref)?, &receipt.value)?;
    Ok(PinOperation { pin, receipt })
}

pub fn unpin_object(input: UnpinObjectInput<'_>) -> Result<Receipt> {
    let root = open_capability_retention_root(input.root)?;
    unpin_object_with_root(UnpinObjectInput {
        root: &root,
        pin_ref: input.pin_ref,
        requester_ref: input.requester_ref,
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        has_authority: input.has_authority,
    })
}

pub fn unpin_object_with_root(input: UnpinObjectInput<'_, CapabilityRetentionRoot>) -> Result<Receipt> {
    ensure_store_with_root(input.root)?;
    require_ref(input.pin_ref, "pin ref")?;
    require_ref(input.requester_ref, "requester ref")?;
    validate_refs(input.policy_refs, "unpin policy ref")?;
    validate_refs(input.evidence_refs, "unpin evidence ref")?;
    let pin_file = capability_ref_path(PIN_DIR, input.pin_ref)?;
    let pin_result = read_store_value_with_root(input.root, &pin_file).and_then(|value| parse_pin(&value));
    let (decision, object_ref, object_kind, retention_class, diagnostics) = match pin_result {
        Ok(pin) if input.has_authority => {
            input.root.root().remove_file(&pin_file)?;
            ("pass", pin.object_ref, pin.object_kind, pin.retention_class, Vec::new())
        }
        Ok(pin) => ("deny", pin.object_ref, pin.object_kind, pin.retention_class, vec![
            "unpin-authority-missing".to_string(),
        ]),
        Err(_) => ("deny", input.pin_ref.to_string(), "unknown".to_string(), CLASS_AUDIT_RECEIPT.to_string(), vec![
            "pin-ref-not-found".to_string(),
        ]),
    };
    let index = reference_index_for_object_with_root(ReferenceIndexForObjectInput {
        root: input.root,
        object_ref: &object_ref,
        object_kind: &object_kind,
        retained_refs: &[],
        remote_refs: &[],
        is_complete: true,
    })?;
    let receipt = build_receipt(ReceiptBuildInput {
        decision,
        action: ACTION_UNPIN,
        object_ref: &object_ref,
        object_kind: &object_kind,
        retention_class: &retention_class,
        requester_ref: input.requester_ref,
        index_ref: &index.index_ref,
        pin_refs: &[input.pin_ref.to_string()],
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: input.policy_refs,
        evidence_refs: input.evidence_refs,
        tombstone_ref: None,
        diagnostics: &diagnostics,
    })?;
    write_store_value_with_root(
        input.root,
        &capability_ref_path(RECEIPT_DIR, &receipt.receipt_ref)?,
        &receipt.value,
    )?;
    Ok(receipt)
}

pub fn reference_index_value(input: &ReferenceIndexInput) -> Result<IoValue> {
    validate_reference_index_input(input)?;
    Ok(crate::preserves_rail::record("retention-reference-index-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_REFERENCE_INDEX_SCHEMA),
        object_value(&input.object_ref, &input.object_kind),
        crate::preserves_rail::record("pins", vec![strings_sequence(&input.pin_refs)]),
        crate::preserves_rail::record("retained", vec![strings_sequence(&input.retained_refs)]),
        crate::preserves_rail::record("tombstones", vec![strings_sequence(&input.tombstone_refs)]),
        crate::preserves_rail::record("remote", vec![strings_sequence(&input.remote_refs)]),
        crate::preserves_rail::record("proof", vec![crate::preserves_rail::string(if input.is_complete {
            "complete"
        } else {
            "incomplete"
        })]),
        checks_value(&[
            ("active-pins-indexed", "pass"),
            ("receipt-dependencies-indexed", "pass"),
            ("mutable-name-not-gc-proof", "pass"),
            ("remote-cache-considered", pass_or_deny(input.is_complete)),
        ]),
    ]))
}

pub fn parse_reference_index(value: &IoValue) -> Result<ReferenceIndex> {
    let fields = value
        .collect_simple_record("retention-reference-index-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-reference-index-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_REFERENCE_INDEX_SCHEMA,
        "retention reference index schema",
    )?;
    let (object_ref, object_kind) = parse_object_value(&fields[1])?;
    let pin_refs = record_ref_sequence(&fields[2], "pins")?;
    let retained_refs = record_ref_sequence(&fields[3], "retained")?;
    let tombstone_refs = record_ref_sequence(&fields[4], "tombstones")?;
    let remote_refs = record_ref_sequence(&fields[5], "remote")?;
    let proof = record_string(&fields[6], "proof")?;
    require_check(&parse_checks(&fields[7])?, "mutable-name-not-gc-proof", "retention reference index")?;
    Ok(ReferenceIndex {
        index_ref: crate::preserves_rail::canonical_hash(value)?,
        object_ref,
        object_kind,
        pin_refs,
        retained_refs,
        tombstone_refs,
        remote_refs,
        is_complete: proof == "complete",
        value: value.clone(),
    })
}

pub fn reference_index_for_object(input: ReferenceIndexForObjectInput<'_>) -> Result<ReferenceIndex> {
    let root = open_capability_retention_root(input.root)?;
    reference_index_for_object_with_root(ReferenceIndexForObjectInput {
        root: &root,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retained_refs: input.retained_refs,
        remote_refs: input.remote_refs,
        is_complete: input.is_complete,
    })
}

pub fn reference_index_for_object_with_root(
    input: ReferenceIndexForObjectInput<'_, CapabilityRetentionRoot>,
) -> Result<ReferenceIndex> {
    ensure_store_with_root(input.root)?;
    let pins = pins_for_object_with_root(input.root, input.object_ref)?;
    let mut pin_refs = Vec::with_capacity(pins.len());
    for pin in &pins {
        push_bounded(&mut pin_refs, pin.pin_ref.clone(), MAX_RETENTION_REFS, "retention index pin refs")?;
    }
    let tombstone_refs = tombstone_refs_for_object_with_root(input.root, input.object_ref)?;
    let value = reference_index_value(&ReferenceIndexInput {
        object_ref: input.object_ref.to_string(),
        object_kind: input.object_kind.to_string(),
        pin_refs,
        retained_refs: input.retained_refs.to_vec(),
        tombstone_refs,
        remote_refs: input.remote_refs.to_vec(),
        is_complete: input.is_complete,
    })?;
    parse_reference_index(&value)
}
