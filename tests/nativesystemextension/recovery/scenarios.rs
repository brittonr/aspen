use crate::support::*;

pub(super) fn crash_readmission(
    twin: &mut super::twin::Live,
) -> TestResult<molten::harness::composition::ScenarioFacts> {
    twin.install_groups()?;
    let member = twin.definitions.group.members[0].member_id.clone();
    let (local, local_payload) =
        twin.submit_work(&member, molten::system_extension::WorkClass::RuntimeLocal, 3, "delayed-local")?;
    let (durable, payload) =
        twin.submit_work(&member, molten::system_extension::WorkClass::Durable, 4, "durable-restart")?;
    let old_claim = twin.readmit_work(&member, &durable.work_ref, 5)?;
    let before = twin.status(6)?;
    let has_sibling_progressed_before = twin.progress_sibling(7)?;
    let ready = twin.crash_serving_node(8)?;
    let ready_fields = molten::preserves_rail::simple_record_fields(&ready, "node-service-host-ready-v1", 4)?;
    let hosted = super::wire::Snapshot::parse(&molten::preserves_rail::value_to_iovalue(&ready_fields[3]))?;
    assert_eq!(
        hosted.members.get(&member).or_fail("actual retained supervisor")?.incarnation,
        before.members.get(&member).or_fail("source supervisor")?.incarnation
    );
    let reopened = twin.status(9)?;
    assert_eq!(
        reopened.work.get(&durable.work_ref).or_fail("durable work after SIGKILL")?.claim_ref.as_deref(),
        Some(old_claim.as_str())
    );
    assert_eq!(
        reopened.work.get(&durable.work_ref).or_fail("durable payload after SIGKILL")?.payload_ref,
        payload.value_ref
    );
    assert!(twin.supervise_group(10)?, "admitted crash recovery must execute actual native callbacks");
    let recovered = twin.status(10)?;
    let fresh_incarnation = recovered.members.get(&member).or_fail("recovered member")?.incarnation;
    assert!(fresh_incarnation > before.members.get(&member).or_fail("previous member")?.incarnation);
    let has_stale_acted = twin.deliver_work(&local, &local_payload, None, 11)?;
    let has_old_claim_acted = twin.deliver_work(&durable, &payload, Some(&old_claim), 12)?;
    let fresh_claim = twin.readmit_work(&member, &durable.work_ref, 13)?;
    let sequence = twin.status(13)?.members.get(&member).or_fail("before durable callback")?.sequence;
    let has_acted = twin.deliver_work(&durable, &payload, Some(&fresh_claim), 14)?;
    let after = twin.status(15)?;
    let retained = after.work.get(&durable.work_ref).or_fail("durable work retained after delivery")?;
    let mut facts = molten::harness::composition::ScenarioFacts::empty();
    facts.stale_delivery_acted = has_stale_acted || has_old_claim_acted;
    facts.durable_work_readmitted = fresh_claim != old_claim
        && has_acted
        && after.members.get(&member).or_fail("durable callback observation")?.sequence > sequence
        && retained.claim_ref.as_deref() == Some(fresh_claim.as_str())
        && retained.receipt_ref.is_some();
    facts.durable_work_lost = retained.payload_ref != payload.value_ref;
    assert_eq!(retained.incarnation, durable.incarnation, "restart readmission must retain logical-work provenance");
    assert!(matches!(retained.disposition.as_str(), "unknown" | "completed"));
    facts.sibling_progressed = has_sibling_progressed_before && twin.progress_sibling(16)?;
    Ok(facts)
}

pub(super) fn storm(twin: &mut super::twin::Live) -> TestResult<molten::harness::composition::ScenarioFacts> {
    twin.install_groups()?;
    let group_id = twin.definitions.group.group_id.clone();
    let mut previous = Vec::new();
    let mut is_terminated = false;
    let mut has_sibling_progressed = true;
    for tick in [10_u64, 20, 30, 40, 50] {
        twin.crash_serving_node(tick - 1)?;
        let reopened = twin.status(tick - 1)?;
        let retained = &reopened.groups.get(&group_id).or_fail("reopened group history")?.history;
        // Lost history is itself a consumer-visible durability failure; keep driving the broken rail.
        let has_history_survived = retained == &previous;
        assert!(
            twin.supervise_group(tick)?,
            "storm operation must produce an admitted terminal or restart observation"
        );
        let observed = twin.status(tick)?;
        let group = observed.groups.get(&group_id).or_fail("storm group")?;
        has_sibling_progressed &= twin.progress_sibling(tick + 1)?;
        if group.terminal {
            is_terminated = has_history_survived
                && group.pending == 0
                && group
                    .history
                    .iter()
                    .all(|(_, observed)| *observed <= tick && tick - observed <= twin.definitions.group.window_ticks);
            break;
        }
        previous = group.history.clone();
    }
    let mut facts = molten::harness::composition::ScenarioFacts::empty();
    facts.storm_terminated = is_terminated;
    facts.sibling_progressed = has_sibling_progressed;
    Ok(facts)
}

pub(super) fn upgrade(twin: &mut super::twin::Live) -> TestResult<molten::harness::composition::ScenarioFacts> {
    twin.install_groups()?;
    let member = twin.definitions.group.members[0].member_id.clone();
    let (old_local, local_payload) =
        twin.submit_work(&member, molten::system_extension::WorkClass::RuntimeLocal, 3, "upgrade-local")?;
    let (durable, source_payload) =
        twin.submit_work(&member, molten::system_extension::WorkClass::Durable, 4, "upgrade-durable")?;
    let before = twin.status(5)?;
    let prior = before.members.get(&member).or_fail("source generation")?;
    let replacement = twin
        .bundle
        .replacements
        .iter()
        .find(|replacement| replacement.previous_manifest_ref == prior.manifest_ref)
        .or_fail("exact admitted generation successor")?
        .service
        .clone();
    let has_sibling_progressed_before = crash_and_upgrade(twin, &member, &replacement)?;
    let after = twin.status(10)?;
    let active = after.members.get(&member).or_fail("actual replacement")?.clone();
    let checkpoint =
        twin.materialize(&replacement, active.checkpoint_ref.as_deref().or_fail("drained generation checkpoint")?)?;
    let transformed =
        twin.materialize(&replacement, active.state_ref.as_deref().or_fail("activated semantic state")?)?;
    let expected = [
        replacement.context.instance_id.as_bytes(),
        &active.generation.to_le_bytes(),
        &active.sequence.to_le_bytes(),
        checkpoint.as_slice(),
    ]
    .concat();
    let ready = twin.crash_serving_node(11)?;
    let ready_fields = molten::preserves_rail::simple_record_fields(&ready, "node-service-host-ready-v1", 4)?;
    let reopened = super::wire::Snapshot::parse(&molten::preserves_rail::value_to_iovalue(&ready_fields[3]))?;
    let persisted = reopened.members.get(&member).or_fail("replacement retained across actual serving-node loss")?;
    let has_stale_acted = twin.deliver_work(&old_local, &local_payload, None, 11)?;
    let (ordinary_admitted, _) = twin.execute(
        "readmit",
        super::wire::operation("readmit", vec![
            molten::preserves_rail::string(&member),
            molten::preserves_rail::string(&durable.work_ref),
        ]),
        12,
    )?;
    assert!(!ordinary_admitted, "ordinary readmission must not silently change work generation");
    let migration_ref = twin.migration_ref(&replacement, active.generation)?;
    let was_readmitted = migrate_and_deliver(twin, &member, &durable, &source_payload, &migration_ref)?;
    let final_status = twin.status(15)?;
    let retained = final_status.work.get(&durable.work_ref).or_fail("migrated logical work")?;
    let mut facts = molten::harness::composition::ScenarioFacts::empty();
    facts.upgrade_completed = active.phase == "running"
        && active.accepting
        && active.generation == prior.generation + 1
        && after.groups.get(&twin.definitions.group.group_id).or_fail("upgrade pending inventory")?.pending == 0
        && transformed == expected
        && persisted.generation == active.generation
        && persisted.state_ref == active.state_ref
        && persisted.checkpoint_ref == active.checkpoint_ref;
    facts.stale_generation_acted = has_stale_acted;
    facts.durable_work_readmitted = was_readmitted
        && retained.generation == active.generation
        && retained.lineage.iter().any(|reference| reference == &migration_ref);
    facts.durable_work_lost = retained.receipt_ref.is_none();
    facts.sibling_progressed = has_sibling_progressed_before && twin.progress_sibling(16)?;
    Ok(facts)
}

fn crash_and_upgrade(
    twin: &mut super::twin::Live,
    member: &str,
    replacement: &molten::node_service_host::codec::ExtensionInput,
) -> TestResult<bool> {
    let reference =
        molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("host-native-replacement-v1", vec![
            molten::preserves_rail::string(&replacement.executable.manifest_ref),
            molten::preserves_rail::string(&replacement.executable.executable_ref),
            molten::preserves_rail::string(&replacement.host_profile.profile_ref),
        ]))?;
    let has_sibling_progressed_before = twin.progress_sibling(6)?;
    twin.crash_serving_node(7)?;
    twin.require(
        "upgrade",
        super::wire::operation("upgrade", vec![
            molten::preserves_rail::string(member),
            molten::preserves_rail::string(reference),
        ]),
        10,
    )?;
    Ok(has_sibling_progressed_before)
}

fn migrate_and_deliver(
    twin: &mut super::twin::Live,
    member: &str,
    source: &molten::system_extension::WorkPresentation,
    source_payload: &molten::system_extension::NativeCallbackValue,
    migration_ref: &str,
) -> TestResult<bool> {
    let bytes =
        molten::preserves_rail::canonical_bytes(&molten::preserves_rail::record("explicit-migrated-work-v1", vec![
            molten::preserves_rail::string(&source.work_ref),
            molten::preserves_rail::string(&source_payload.value_ref),
            molten::preserves_rail::string(migration_ref),
        ]))?;
    let payload = molten::system_extension::NativeCallbackValue {
        value_ref: molten::preserves_rail::content_ref_from_bytes(&bytes),
        bytes,
    };
    let input = molten::preserves_rail::record("node-service-host-migrated-work-v1", vec![
        molten::preserves_rail::string(&source.work_ref),
        molten::preserves_rail::string(migration_ref),
        super::wire::payload(&payload),
    ]);
    let receipt = twin.require(
        "readmit",
        super::wire::operation("readmit", vec![molten::preserves_rail::string(member), input]),
        13,
    )?;
    let value = super::wire::outcome(&receipt)?;
    let fields = molten::preserves_rail::simple_record_fields(&value, "node-service-host-claim-v1", 4)?;
    let claim = super::wire::text(&fields[0])?;
    let current = twin.status(13)?;
    let active = current.members.get(member).or_fail("current work generation")?;
    let work = molten::system_extension::WorkPresentation {
        work_ref: source.work_ref.clone(),
        class: source.class,
        generation: active.generation,
        incarnation: active.incarnation,
    };
    let sequence = active.sequence;
    let was_delivered = twin.deliver_work(&work, &payload, Some(&claim), 14)?;
    Ok(was_delivered && twin.status(14)?.members.get(member).or_fail("transformed work callback")?.sequence > sequence)
}
