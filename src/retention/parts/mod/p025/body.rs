
fn profile(value: &IoValue) -> Option<String> {
    if let Ok(profile) = parse_candidate_bundle_profile(value) {
        return Some(format!(
            "retention candidate bundle profile ref={} decision={} profile={} loss={} bundle={} markers={} diagnostics={}",
            profile.profile_ref,
            profile.decision,
            profile.profile,
            profile.loss_classification,
            profile.bundle_ref,
            profile.marker_refs.len(),
            profile.diagnostics.join(",")
        ));
    }
    if let Ok(verify) = parse_candidate_bundle_verify(value) {
        return Some(format!(
            "retention candidate bundle verify ref={} decision={} bundle={} explain={} object={} kind={} class={} action={} subsystem={} artifacts={} files={} diagnostics={}",
            verify.verify_ref,
            verify.decision,
            verify.bundle_ref,
            verify.explain_ref,
            verify.object_ref,
            verify.object_kind.as_deref().unwrap_or("any"),
            verify.retention_class.as_deref().unwrap_or("any"),
            verify.action.as_deref().unwrap_or("any"),
            verify.subsystem.as_deref().unwrap_or("any"),
            verify.artifact_refs.len(),
            verify.file_refs.len(),
            verify.diagnostics.join(",")
        ));
    }
    None
}

fn stored(value: &IoValue) -> Option<String> {
    if let Ok(receipt) = parse_receipt(value) {
        return Some(format!(
            "retention receipt ref={} decision={} action={} object={} class={} pins={} tombstone={} diagnostics={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.action,
            receipt.object_ref,
            receipt.retention_class,
            receipt.pin_refs.len(),
            receipt.tombstone_ref.as_deref().unwrap_or("none"),
            receipt.diagnostics.join(",")
        ));
    }
    if let Ok(tombstone) = parse_tombstone(value) {
        return Some(format!(
            "retention tombstone ref={} object={} class={} action={} receipt={}",
            tombstone.tombstone_ref,
            tombstone.object_ref,
            tombstone.retention_class,
            tombstone.action,
            tombstone.receipt_ref
        ));
    }
    None
}

pub fn run_fixture(out: &Path) -> Result<Vec<(String, IoValue)>> {
    let output_root = CapabilityBundleRoot::open(out)?;
    let root = CapabilityRetentionRoot::open_bundle_state(&output_root)?;
    ensure_store_with_root(&root)?;
    let seed = seed_refs()?;
    let class = class_value(&seed)?;
    let pin = pin_step(&root, &seed)?;
    let deny = eval_step(&root, &seed, ACTION_DELETE)?;
    let unpin = unpin_object_with_root(UnpinObjectInput {
        root: &root,
        pin_ref: &pin.pin.pin_ref,
        requester_ref: &seed.owner_ref,
        policy_refs: &seed.policy_refs,
        evidence_refs: &seed.evidence_refs,
        has_authority: true,
    })?;
    let delete = eval_step(&root, &seed, ACTION_TOMBSTONE)?;
    let artifacts = output_values(OutputValues {
        class,
        pin,
        deny,
        unpin,
        delete,
    })?;
    for (name, value) in &artifacts {
        write_bundle_value(&output_root, &bundle_path(name)?, value)?;
    }
    Ok(artifacts)
}

struct SeedRefs {
    object_ref: String,
    owner_ref: String,
    policy_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

fn seed_refs() -> Result<SeedRefs> {
    Ok(SeedRefs {
        object_ref: synthetic_ref("retention-object")?,
        owner_ref: synthetic_ref("owner")?,
        policy_refs: vec![synthetic_ref("policy")?],
        evidence_refs: vec![synthetic_ref("evidence")?],
    })
}

fn class_value(seed: &SeedRefs) -> Result<IoValue> {
    class_profile_value(&ClassProfileInput {
        class_name: CLASS_PRIVATE_SECRET_REF.to_string(),
        minimum_age_seconds: 0,
        maximum_age_seconds: Some(86_400),
        deletion_authority_ref: synthetic_ref("authority")?,
        policy_refs: seed.policy_refs.clone(),
        has_secret_redaction_hook: true,
        has_remote_gc_plan: true,
        can_compact: true,
    })
}

fn pin_step(root: &CapabilityRetentionRoot, seed: &SeedRefs) -> Result<PinOperation> {
    pin_object_with_root(root, PinInput {
        object_ref: seed.object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: CLASS_PRIVATE_SECRET_REF.to_string(),
        source: SOURCE_SECRET_REDACTION.to_string(),
        reason: "private repro reveal pending".to_string(),
        owner_ref: seed.owner_ref.clone(),
        expiry_ref: None,
        policy_refs: seed.policy_refs.clone(),
        evidence_refs: seed.evidence_refs.clone(),
        has_authority: true,
    })
}

fn eval_step(root: &CapabilityRetentionRoot, seed: &SeedRefs, action: &str) -> Result<Evaluation> {
    evaluate_with_root(EvaluationInput {
        root,
        object_ref: &seed.object_ref,
        object_kind: "encrypted-ref",
        retention_class: CLASS_PRIVATE_SECRET_REF,
        action,
        requester_ref: &seed.owner_ref,
        is_reference_index_complete: true,
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: &seed.policy_refs,
        evidence_refs: &seed.evidence_refs,
        has_delete_authority: true,
        has_remote_gc_clearance: true,
    })
}

struct OutputValues {
    class: IoValue,
    pin: PinOperation,
    deny: Evaluation,
    unpin: Receipt,
    delete: Evaluation,
}

fn output_values(parts: OutputValues) -> Result<Vec<(String, IoValue)>> {
    let OutputValues {
        class,
        pin,
        deny,
        unpin,
        delete,
    } = parts;
    let PinOperation {
        pin,
        receipt: pin_receipt,
    } = pin;
    let Evaluation {
        receipt: deny_receipt, ..
    } = deny;
    let Evaluation {
        receipt: delete_receipt,
        tombstone,
        ..
    } = delete;
    let mut artifacts = Vec::new();
    push_named(&mut artifacts, "retention-class.preserves", class)?;
    push_named(&mut artifacts, "pin.preserves", pin.value)?;
    push_named(&mut artifacts, "pin-receipt.preserves", pin_receipt.value)?;
    push_named(&mut artifacts, "delete-denied.preserves", deny_receipt.value)?;
    push_named(&mut artifacts, "unpin-receipt.preserves", unpin.value)?;
    push_named(&mut artifacts, "tombstone-receipt.preserves", delete_receipt.value)?;
    if let Some(tombstone) = tombstone {
        push_named(&mut artifacts, "tombstone.preserves", tombstone.value)?;
    }
    Ok(artifacts)
}

struct ReceiptBuildInput<'a> {
    decision: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    requester_ref: &'a str,
    index_ref: &'a str,
    pin_refs: &'a [String],
    retained_refs: &'a [String],
    remote_refs: &'a [String],
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    tombstone_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

fn build_receipt(input: ReceiptBuildInput<'_>) -> Result<Receipt> {
    validate_receipt_build_input(&input)?;
    let value = crate::preserves_rail::record("retention-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("requester", vec![crate::preserves_rail::string(input.requester_ref)]),
        crate::preserves_rail::record("index", vec![crate::preserves_rail::string(input.index_ref)]),
        crate::preserves_rail::record("pins", vec![strings_sequence(input.pin_refs)]),
        crate::preserves_rail::record("retained", vec![strings_sequence(input.retained_refs)]),
        crate::preserves_rail::record("remote", vec![strings_sequence(input.remote_refs)]),
        crate::preserves_rail::record("tombstone", vec![optional_ref_value(input.tombstone_ref)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        crate::preserves_rail::record("policy", vec![strings_sequence(input.policy_refs)]),
        checks_value(&[
            ("reference-index-bound", "pass"),
            ("policy-bound", pass_or_deny(!input.policy_refs.is_empty())),
            ("authority-bound", pass_or_deny(input.decision == "pass" || input.action == ACTION_ELIGIBILITY)),
            ("mutable-name-not-gc-proof", "pass"),
            ("remote-cache-considered", "pass"),
        ]),
    ]);
    parse_receipt(&value)
}

struct TombstoneBuildInput<'a> {
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
    receipt_ref: &'a str,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
}

fn build_tombstone(input: TombstoneBuildInput<'_>) -> Result<Tombstone> {
    require_ref(input.receipt_ref, "retention tombstone receipt ref")?;
    let value = crate::preserves_rail::record("retention-tombstone-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_TOMBSTONE_SCHEMA),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        crate::preserves_rail::record("receipt", vec![crate::preserves_rail::string(input.receipt_ref)]),
        crate::preserves_rail::record("policy", vec![strings_sequence(input.policy_refs)]),
        crate::preserves_rail::record("evidence", vec![strings_sequence(input.evidence_refs)]),
        crate::preserves_rail::record("public-metadata", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("object-kind", vec![crate::preserves_rail::string(input.object_kind)]),
            crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
            crate::preserves_rail::record("content", vec![crate::preserves_rail::string("redacted-or-deleted")]),
        ])]),
        checks_value(&[
            ("audit-visible-tombstone", "pass"),
            ("secret-content-not-leaked", "pass"),
            ("deletion-not-hidden", "pass"),
        ]),
    ]);
    parse_tombstone(&value)
}

fn evaluation_diagnostics<Root: ?Sized>(
    input: &EvaluationInput<'_, Root>,
    index: &ReferenceIndex,
) -> Result<Vec<String>> {
    let is_destructive = is_destructive_action(input.action);
    let mut diagnostics = Vec::new();
    push_notes(&mut diagnostics, [
        (!input.is_reference_index_complete, "incomplete-reference-proof"),
        (!index.pin_refs.is_empty(), "active-pins-present"),
        (!input.retained_refs.is_empty(), "retained-dependencies-present"),
        (input.policy_refs.is_empty(), "retention-policy-missing"),
        (is_destructive && input.evidence_refs.is_empty(), "retention-evidence-missing"),
        (is_destructive && !input.has_delete_authority, "delete-authority-missing"),
        (
            is_destructive && !input.remote_refs.is_empty() && !input.has_remote_gc_clearance,
            "remote-cache-refs-present",
        ),
        (input.retention_class == CLASS_LEGAL_HOLD && is_destructive, "legal-hold-class-not-deletable"),
        (
            input.retention_class == CLASS_PRIVATE_SECRET_REF && input.action == ACTION_COMPACT,
            "private-secret-ref-compaction-denied",
        ),
    ])?;
    Ok(diagnostics)
}
