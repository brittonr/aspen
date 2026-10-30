/// Derives replay inputs only from independently emitted DAG, gate and import evidence.
/// The final live session is supplied exclusively as the comparison target.
pub fn compare(
    manifest: &crate::mesh::admission::Manifest,
    inventory: &crate::mesh::envelope::network::VerifiedRecord,
    outcome: &crate::mesh::transfer::execution::PullOutcome,
    gates: &[preserves::IOValue],
    durable: &crate::node_state::Scope,
) -> crate::error::Result<()> {
    if outcome.imports.len() != 1 {
        return Err(invalid("terminal fixture requires one actual local import receipt"));
    }
    let imported = &outcome.imports[0];
    let fields = imported
        .collect_simple_record("federation-import-complete-v1", Some(5))
        .ok_or_else(|| invalid("terminal parity lacks actual local import completion"))?;
    let session = text(&fields[0])?;
    let generation = number(&fields[1])?;
    let head = text(&fields[2])?;
    if generation != manifest.generation()
        || text(&fields[3])? != outcome.dag.canonical_receipt.record_ref
        || !outcome.dag.progress.verified.iter().any(|object| object.as_str() == head)
    {
        return Err(invalid("actual import receipt does not correlate to independently verified DAG output"));
    }
    let (prerequisites, now) = admission(gates, &session, generation, &head)?;
    let events = vec![
        molten_core::federation::SessionEvent::BeginFetch,
        molten_core::federation::SessionEvent::Verified {
            head_ref: head.clone(),
            evidence_ref: outcome.dag.canonical_receipt.record_ref.clone(),
        },
        molten_core::federation::SessionEvent::Admitted {
            head_ref: head.clone(),
            prerequisites,
        },
        molten_core::federation::SessionEvent::Committed {
            head_ref: head,
            evidence_ref: crate::preserves_rail::canonical_hash(imported)?,
        },
    ];
    let proof = crate::mesh::simulation::compare_recorded_run(crate::mesh::simulation::RecordedRun {
        template: crate::mesh::simulation::Replay {
            manifest_ref: manifest.manifest_ref(),
            bounds: manifest.bounds(),
            peers: manifest.peers(),
            local: &[],
            generation,
            session: None,
            last_query: None,
            resource_history: Vec::new(),
        },
        inventory: inventory.clone(),
        events,
        now,
        live: outcome.session.clone(),
    })?;
    super::setup::dagports::persist(durable, "terminal-parity.preserves", &proof).map(|_| ())
}
fn admission(
    gates: &[preserves::IOValue],
    session: &str,
    generation: u64,
    head: &str,
) -> crate::error::Result<(molten_core::federation::ImportPrerequisites, u64)> {
    if gates.len() != 7 {
        return Err(invalid("terminal parity requires seven actual local admission gates"));
    }
    let mut references = std::collections::BTreeMap::new();
    let mut observed_tick = None;
    for value in gates {
        let fields = value
            .collect_simple_record("federation-local-admission-v1", Some(7))
            .ok_or_else(|| invalid("terminal parity encountered malformed actual gate evidence"))?;
        let name = text(&fields[0])?;
        let now = number(&fields[4])?;
        if text(&fields[1])? != session
            || number(&fields[2])? != generation
            || text(&fields[3])? != head
            || observed_tick.is_some_and(|prior| prior != now)
        {
            return Err(invalid("actual local gate evidence differs from imported session, generation, head or tick"));
        }
        observed_tick = Some(now);
        if references.insert(name, crate::preserves_rail::canonical_hash(value)?).is_some() {
            return Err(invalid("actual local gate evidence duplicates a gate"));
        }
    }
    let prerequisites = molten_core::federation::ImportPrerequisites {
        schema_ref: reference(&references, "schema")?,
        policy_ref: reference(&references, "policy")?,
        capability_ref: reference(&references, "capability")?,
        provenance_ref: reference(&references, "provenance")?,
        resource_ref: reference(&references, "resource")?,
        retention_ref: reference(&references, "retention")?,
        import_ref: reference(&references, "import")?,
    };
    Ok((prerequisites, observed_tick.ok_or_else(|| invalid("actual gate observation tick is absent"))?))
}
fn reference(references: &std::collections::BTreeMap<String, String>, gate: &str) -> crate::error::Result<String> {
    references
        .get(gate)
        .cloned()
        .ok_or_else(|| invalid("terminal parity lacks a required actual gate observation"))
}
fn text(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| invalid("actual evidence lacks a required string"))
}
fn number(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value
        .as_u64()
        .and_then(|value| value.ok())
        .ok_or_else(|| invalid("actual evidence lacks a required integer"))
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
