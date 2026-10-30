pub(super) fn dag_ref(value: &preserves::IOValue) -> crate::error::Result<String> {
    let fields = value
        .collect_simple_record("molten-dag-sync-receipt-v1", Some(10))
        .ok_or_else(|| invalid("bundle DAG receipt is malformed"))?;
    if text(&fields[0], "decision")? != "complete"
        || number(&fields[5], "requested")? != 1
        || number(&fields[6], "verified")? != 1
    {
        return Err(invalid("detached single-artifact proof requires complete verified DAG receipt"));
    }
    let plan = inner(&fields[1], "plan-ref")?;
    let plan = plan.collect_simple_record("some", Some(1)).ok_or_else(|| invalid("DAG receipt lacks plan"))?;
    let plan = plan[0].as_string().ok_or_else(|| invalid("invalid DAG plan reference"))?;
    let strategy = match text(&fields[4], "strategy")?.as_str() {
        "full" => molten_core::dag_sync::DagSyncStrategy::Full,
        "stem-first" => molten_core::dag_sync::DagSyncStrategy::StemFirst,
        "leaf-only" => molten_core::dag_sync::DagSyncStrategy::LeafOnly,
        "resumable" => molten_core::dag_sync::DagSyncStrategy::Resumable,
        "peer-partitioned" => molten_core::dag_sync::DagSyncStrategy::PeerPartitioned,
        _ => return Err(invalid("unknown DAG strategy")),
    };
    let receipt = molten_core::dag_sync::DagSyncReceipt {
        decision: molten_core::dag_sync::DagSyncDecision::Complete,
        plan_ref: Some(molten_core::dag_sync::DagPlanRef::new(plan.as_ref()).map_err(|_| invalid("invalid DAG plan"))?),
        epoch_ref: molten_core::dag_sync::DagEpochRef::new(text(&fields[2], "epoch-ref")?)
            .map_err(|_| invalid("invalid DAG epoch"))?,
        generation: number(&fields[3], "generation")?,
        strategy,
        requested: 1,
        verified: 1,
        missing: Vec::new(),
        issues: Vec::new(),
        non_claims: molten_core::dag_sync::DAG_SYNC_NON_CLAIMS.iter().map(|value| value.to_string()).collect(),
    };
    let canonical = crate::dag_sync::canonical_dag_receipt(&receipt)?;
    if canonical.value != *value {
        return Err(invalid("detached DAG receipt has missing, issues, or noncanonical fields"));
    }
    Ok(canonical.record_ref)
}
pub(super) fn content(fields: &[preserves::Value<preserves::IOValue>]) -> crate::error::Result<()> {
    let wire = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let wire = wire
        .collect_simple_record("federation-wire-v1", Some(6))
        .ok_or_else(|| invalid("missing signed response"))?;
    let object = crate::preserves_rail::value_to_iovalue(&wire[4]);
    let object = object
        .collect_simple_record("federation-object-v1", Some(4))
        .ok_or_else(|| invalid("missing object"))?;
    let manifest =
        crate::chunk_store::parse_manifest_value(&crate::preserves_rail::value_to_iovalue(&object[2]), None)?;
    let event = crate::preserves_rail::value_to_iovalue(&fields[6]);
    let event = event
        .collect_simple_record("content-store-adapter-event-v1", Some(11))
        .ok_or_else(|| invalid("bundle lacks actual content adapter completion event"))?;
    if text(&event[2], "manifest-ref")? != manifest.manifest_ref
        || text(&event[4], "terminal")? != "durable"
        || number(&event[6], "observed-bytes")? != manifest.total_len
    {
        return Err(invalid("detached content completion does not bind the signed manifest and verified byte length"));
    }
    Ok(())
}
pub(super) fn gates(
    gates: &[preserves::Value<preserves::IOValue>],
    imported: &[preserves::Value<preserves::IOValue>],
) -> crate::error::Result<()> {
    let expected = [
        "schema",
        "policy",
        "capability",
        "provenance",
        "resource",
        "retention",
        "import",
    ];
    if gates.len() != expected.len() {
        return Err(invalid("missing local gates"));
    }
    for (gate, name) in gates.iter().zip(expected) {
        let value = crate::preserves_rail::value_to_iovalue(gate);
        let fields = value
            .collect_simple_record("federation-local-admission-v1", Some(7))
            .ok_or_else(|| invalid("malformed local admission gate"))?;
        if fields[0].as_string().as_deref() != Some(name)
            || fields[1] != imported[0]
            || fields[2] != imported[1]
            || fields[3] != imported[2]
        {
            return Err(invalid("local admission gate does not match exact imported session, generation and head"));
        }
    }
    Ok(())
}
pub(super) fn transport(fields: &[preserves::Value<preserves::IOValue>]) -> crate::error::Result<()> {
    let frames = fields[4].collect_sequence().ok_or_else(|| invalid("missing transport frames"))?;
    let frames = frames.iter().cloned().collect::<Vec<preserves::Value<preserves::IOValue>>>();
    if frames.len() != 4 {
        return Err(invalid("detached proof lacks three actual frames and role process evidence"));
    }
    let imported = crate::preserves_rail::value_to_iovalue(&fields[8]);
    let imported = imported
        .collect_simple_record("federation-import-complete-v1", Some(5))
        .ok_or_else(|| invalid("missing imported session"))?;
    for (index, frame) in frames[..3].iter().enumerate() {
        let value = crate::preserves_rail::value_to_iovalue(frame);
        let frame = value
            .collect_simple_record("federation-live-transport-v1", Some(9))
            .ok_or_else(|| invalid("missing actual canonical transport frame"))?;
        let wire =
            crate::preserves_rail::canonical_bytes(&crate::preserves_rail::value_to_iovalue(&fields[index + 1]))?;
        let reference = crate::preserves_rail::content_ref_from_bytes(&wire);
        let frame_ref = crate::fabric_transport::cross_process::frame_ref(&reference, &wire);
        if frame[2].as_string().as_deref() != Some(reference.as_str())
            || frame[3].as_string().as_deref() != Some(frame_ref.as_str())
            || frame[4].as_string().as_deref() != Some(frame_ref.as_str())
            || frame[6].as_u64().and_then(|value| value.ok()) != u64::try_from(wire.len()).ok()
            || frame[7].as_u64().and_then(|value| value.ok()) != Some(0)
            || (index != 0 && frame[1] != imported[0])
        {
            return Err(invalid("transport evidence does not bind actual signed bytes and receiver session"));
        }
    }
    let role = crate::preserves_rail::value_to_iovalue(&frames[3]);
    let role = role
        .collect_simple_record("federation-live-role-processes-v1", Some(2))
        .ok_or_else(|| invalid("missing actual role process identities"))?;
    if role
        .fields_iter()
        .any(|value| value.as_u64().and_then(|value| value.ok()).is_none_or(|value| value == 0))
    {
        return Err(invalid("invalid actual role process identity"));
    }
    Ok(())
}
pub(super) fn supervision(
    fields: &[preserves::Value<preserves::IOValue>],
    generation: u64,
) -> crate::error::Result<()> {
    let evidence = fields[10].collect_sequence().ok_or_else(|| invalid("missing supervised admission evidence"))?;
    if evidence.len() != 7 {
        return Err(invalid("incomplete supervised admission evidence"));
    }
    let values = evidence.iter().map(crate::preserves_rail::value_to_iovalue).collect::<Vec<_>>();
    let manifest = values[0]
        .collect_simple_record("federated-pull-manifest-v1", Some(9))
        .ok_or_else(|| invalid("missing actual federation manifest"))?;
    let callback = values[5]
        .collect_simple_record("system-extension-callback-receipt-v1", Some(16))
        .ok_or_else(|| invalid("missing actual supervised callback"))?;
    let completion = values[6]
        .collect_simple_record("system-extension-effect-completion-v2", Some(11))
        .ok_or_else(|| invalid("missing actual supervised effect completion"))?;
    let extension_ref = crate::preserves_rail::canonical_hash(&values[1])?;
    let binding_ref = crate::preserves_rail::canonical_hash(&values[2])?;
    let binding = values[2]
        .collect_simple_record("fabric-port-binding-v1", Some(10))
        .ok_or_else(|| invalid("missing actual canonical DAG fabric binding"))?;
    let declared = manifest[4].collect_sequence().ok_or_else(|| invalid("manifest lacks admitted bindings"))?;
    let required = manifest[5].collect_sequence().ok_or_else(|| invalid("manifest lacks required DAG binding"))?;
    let required = required.iter().next().ok_or_else(|| invalid("manifest required DAG binding is empty"))?;
    if !declared.iter().any(|value| value.as_string().as_deref() == Some(binding_ref.as_str()))
        || required.as_string().as_deref() != Some(binding_ref.as_str())
        || text(&binding[5], "descriptor-ref")? != crate::preserves_rail::canonical_hash(&values[3])?
        || text(&binding[6], "registry-ref")? != crate::preserves_rail::canonical_hash(&values[4])?
    {
        return Err(invalid("manifest binding does not retain its actual reviewed descriptor and registry"));
    }
    if manifest[0].as_string().as_deref() != Some(extension_ref.as_str())
        || manifest[2].as_u64().and_then(|value| value.ok()) != Some(generation)
        || text(&callback[1], "manifest-ref")? != extension_ref
        || text(&callback[4], "callback")? != "request"
        || text(&callback[9], "decision")? != "succeeded"
        || number(&callback[5], "generation")? != generation
        || text(&completion[1], "callback-receipt-ref")? != crate::preserves_rail::canonical_hash(&values[5])?
        || text(&completion[2], "binding-ref")? != binding_ref
        || text(&completion[3], "port-id")? != crate::dag_sync::DAG_SYNC_PORT_ID
        || text(&completion[4], "port-version")? != crate::dag_sync::DAG_SYNC_PORT_VERSION
        || number(&completion[6], "generation")? != generation
        || text(&completion[8], "output-ref")? != dag_ref(&crate::preserves_rail::value_to_iovalue(&fields[5]))?
    {
        return Err(invalid("supervised callback, binding, generation or DAG completion chain mismatch"));
    }
    Ok(())
}
pub(super) fn correlate(
    inventory: &crate::mesh::envelope::signatures::SignedRecord,
    request: &crate::mesh::envelope::signatures::SignedRecord,
    response: &crate::mesh::envelope::signatures::SignedRecord,
    artifact_ref: &str,
) -> crate::error::Result<()> {
    let fetched = request
        .payload
        .collect_simple_record("federation-fetch-v1", Some(4))
        .ok_or_else(|| invalid("bundle lacks receiver-issued fetch"))?;
    let object = response
        .payload
        .collect_simple_record("federation-object-v1", Some(4))
        .ok_or_else(|| invalid("bundle lacks signed object response"))?;
    let request_ref = crate::preserves_rail::content_ref_from_bytes(&crate::mesh::envelope::wire::encode(
        request,
        super::MAXIMUM_BYTES,
    )?);
    if fetched[1].as_string().as_deref() != Some(artifact_ref)
        || object[1].as_string().as_deref() != Some(artifact_ref)
        || object[0].as_string().as_deref() != Some(request_ref.as_str())
    {
        return Err(invalid("unsolicited or miscorrelated detached response"));
    }
    let bytes = object[3].collect_sequence().ok_or_else(|| invalid("response lacks content bytes"))?;
    let bytes = bytes
        .iter()
        .map(|byte| {
            byte.as_u64()
                .and_then(|value| value.ok())
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(|| invalid("invalid content byte"))
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    if crate::preserves_rail::content_ref_from_bytes(&bytes) != artifact_ref {
        return Err(invalid("detached content identity mismatch"));
    }
    let fields = inventory
        .payload
        .collect_simple_record("federation-inventory-v1", Some(4))
        .ok_or_else(|| invalid("bundle lacks signed inventory"))?;
    if fields[1].as_string().as_deref() != Some(inventory.signature.metadata.signer_public_ref.as_str()) {
        return Err(invalid("detached inventory signer differs from actual verified provider"));
    }
    let entries = fields[3].collect_sequence().ok_or_else(|| invalid("inventory lacks entries"))?;
    if !entries.iter().any(|entry| {
        crate::preserves_rail::value_to_iovalue(entry)
            .collect_simple_record("federation-entry-v1", Some(7))
            .is_some_and(|entry| entry[1].as_string().as_deref() == Some(artifact_ref))
    }) {
        return Err(invalid("imported artifact absent from signed inventory"));
    }
    Ok(())
}

fn inner(
    value: &preserves::Value<preserves::IOValue>,
    label: &'static str,
) -> crate::error::Result<preserves::IOValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value.collect_simple_record(label, Some(1)).ok_or_else(|| invalid("malformed receipt field"))?;
    Ok(crate::preserves_rail::value_to_iovalue(&fields[0]))
}
fn text(value: &preserves::Value<preserves::IOValue>, label: &'static str) -> crate::error::Result<String> {
    inner(value, label)?
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| invalid("invalid receipt string"))
}
fn number(value: &preserves::Value<preserves::IOValue>, label: &'static str) -> crate::error::Result<u64> {
    inner(value, label)?
        .as_u64()
        .and_then(|value| value.ok())
        .ok_or_else(|| invalid("invalid receipt number"))
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
