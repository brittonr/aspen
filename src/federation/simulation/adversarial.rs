/// Named deterministic cases use an actually verified inventory and the production planner.
/// Crypto/content boundary faults remain separate actual-adapter evidence, never inferred here.
pub fn planner_campaign(
    template: &super::Replay<'_>,
    record: &super::super::envelope::network::VerifiedRecord,
    now: u64,
) -> crate::error::Result<preserves::IOValue> {
    let observed = super::super::envelope::records::verified_inventory(record, template.bounds)?;
    if observed.entries.is_empty() {
        return Err(crate::error::Failure::invalid_harness("adversarial campaign needs a nonempty admitted inventory"));
    }
    let mut cases = Vec::new();
    let base = super::Replay {
        manifest_ref: template.manifest_ref,
        bounds: template.bounds,
        peers: template.peers,
        local: &[],
        generation: template.generation,
        session: None,
        last_query: None,
        resource_history: Vec::new(),
    };
    cases.push(planner_case(("no-fault", None), &base, record, now, true)?);
    cases.push(planner_case(("unsolicited-push", Some("Unsolicited")), &base, record, now, false)?);
    let stale = observed
        .expires_at
        .checked_add(1)
        .ok_or_else(|| crate::error::Failure::invalid_harness("campaign stale tick overflow"))?;
    cases.push(planner_case(("stale-inventory", Some("Stale")), &base, record, stale, true)?);
    let unknown = super::Replay {
        peers: &[],
        ..base.clone()
    };
    cases.push(planner_case(("malicious-peer", Some("UnknownPeer")), &unknown, record, now, true)?);
    let mut peers = template.peers.to_vec();
    for peer in &mut peers {
        peer.available = false;
    }
    let partition = super::Replay {
        peers: &peers,
        ..base.clone()
    };
    cases.push(planner_case(("partition", Some("Unavailable")), &partition, record, now, true)?);
    let rate = super::Replay {
        last_query: Some(now),
        ..base.clone()
    };
    cases.push(planner_case(("restart-rate-recovery", Some("RateLimited")), &rate, record, now, true)?);
    let mut bounds = template.bounds.clone();
    bounds.fetch_bytes = 0;
    let exhausted = super::Replay {
        bounds: &bounds,
        ..base.clone()
    };
    cases.push(planner_case(("resource-exhaustion", Some("Resource")), &exhausted, record, now, true)?);
    let mut local = observed.entries.clone();
    let conflict = crate::preserves_rail::record("federation-conflict-input-v1", vec![crate::preserves_rail::string(
        &local[0].head_ref,
    )]);
    local[0].head_ref = crate::preserves_rail::canonical_hash(&conflict)?;
    let conflicting = super::Replay {
        local: &local,
        ..base.clone()
    };
    cases.push(planner_case(("conflict-preserved", None), &conflicting, record, now, true)?);
    cases.push(session_case(&base, record, now)?);
    cases.push(resource_case(&base, record, now)?);
    Ok(crate::preserves_rail::record("federation-planner-campaign-v1", vec![
        crate::preserves_rail::string(template.manifest_ref),
        crate::preserves_rail::sequence(cases),
        crate::preserves_rail::string(
            "planner/session evidence only; crypto and content faults require actual adapter evidence",
        ),
    ]))
}

fn planner_case(
    case: (&str, Option<&str>),
    template: &super::Replay<'_>,
    record: &super::super::envelope::network::VerifiedRecord,
    now: u64,
    requested: bool,
) -> crate::error::Result<preserves::IOValue> {
    let (name, expected) = case;
    let mut state = (*template).clone();
    let outcome = super::inventory(&mut state, record, now, requested);
    if outcome.as_ref().err().map(String::as_str) != expected {
        return Err(crate::error::Failure::invalid_harness("federation campaign violated expected planner outcome"));
    }
    if name == "conflict-preserved"
        && state
            .session
            .as_ref()
            .is_none_or(|session| session.plan.conflicts.len() != 1 || !session.plan.missing.is_empty())
    {
        return Err(crate::error::Failure::invalid_harness(
            "federation campaign overwrote conflict or planned it for transfer",
        ));
    }
    let trace = super::replay(&mut (*template).clone(), vec![
        super::Input::Inventory {
            record: Box::new(record.clone()),
            now,
            requested,
        },
        super::Input::Resume,
    ])?;
    Ok(crate::preserves_rail::record("federation-planner-case-v1", vec![
        crate::preserves_rail::string(name),
        crate::preserves_rail::string(expected.unwrap_or("allowed")),
        trace,
    ]))
}

fn session_case(
    template: &super::Replay<'_>,
    record: &super::super::envelope::network::VerifiedRecord,
    now: u64,
) -> crate::error::Result<preserves::IOValue> {
    let invalid = || crate::error::Failure::invalid_harness("restart campaign violated session invariant");
    let mut state = (*template).clone();
    super::inventory(&mut state, record, now, true).map_err(|_| invalid())?;
    super::transition(&mut state, &molten_core::federation::SessionEvent::BeginFetch, template.generation, now)
        .map_err(|_| invalid())?;
    let original = state.session.clone().ok_or_else(invalid)?;
    let restored = super::super::lifecycle::checkpoint::decode(
        &super::super::lifecycle::checkpoint::encode(&original),
        template.bounds,
    )?;
    if restored != original || restored.attempts != 1 {
        return Err(invalid());
    }
    state.session = Some(restored);
    let stale_generation = template.generation.checked_add(1).ok_or_else(invalid)?;
    let late = original.plan.deadline.checked_add(1).ok_or_else(invalid)?;
    let unadmitted = molten_core::federation::SessionEvent::Committed {
        head_ref: original.plan.missing[0].head_ref.clone(),
        evidence_ref: record.verification().outcome_ref.clone(),
    };
    if super::transition(&mut state, &molten_core::federation::SessionEvent::BeginFetch, stale_generation, now)
        .err()
        .as_deref()
        != Some("Fence")
        || super::transition(&mut state, &molten_core::federation::SessionEvent::BeginFetch, template.generation, late)
            .err()
            .as_deref()
            != Some("Deadline")
        || super::transition(&mut state, &unadmitted, template.generation, now).err().as_deref() != Some("Unadmitted")
        || state.session.as_ref() != Some(&original)
    {
        return Err(invalid());
    }
    let trace = super::replay(&mut (*template).clone(), vec![
        super::Input::Inventory {
            record: Box::new(record.clone()),
            now,
            requested: true,
        },
        super::Input::Transition {
            event: molten_core::federation::SessionEvent::BeginFetch,
            generation: template.generation,
            now,
        },
        super::Input::Resume,
        super::Input::Transition {
            event: molten_core::federation::SessionEvent::BeginFetch,
            generation: stale_generation,
            now,
        },
        super::Input::Transition {
            event: molten_core::federation::SessionEvent::BeginFetch,
            generation: template.generation,
            now: late,
        },
        super::Input::Transition {
            event: unadmitted,
            generation: template.generation,
            now,
        },
    ])?;
    Ok(crate::preserves_rail::record("federation-session-recovery-case-v1", vec![
        crate::preserves_rail::string("restart-resume-stale-generation-deadline-unadmitted-commit"),
        trace,
        crate::preserves_rail::string("retry count and session preserved; denied effects do not mutate state"),
    ]))
}

fn resource_case(
    template: &super::Replay<'_>,
    record: &super::super::envelope::network::VerifiedRecord,
    now: u64,
) -> crate::error::Result<preserves::IOValue> {
    let mut initial = (*template).clone();
    let first = super::replay(&mut initial, vec![super::Input::Inventory {
        record: Box::new(record.clone()),
        now,
        requested: true,
    }])?;
    let history = super::super::lifecycle::resources::decode(&super::super::lifecycle::resources::encode(
        &initial.resource_history,
    )?)?;
    if history != initial.resource_history || history.is_empty() {
        return Err(crate::error::Failure::invalid_harness("resource campaign did not retain reservation"));
    }
    let recovered = super::Replay {
        session: None,
        last_query: None,
        resource_history: history,
        ..(*template).clone()
    };
    let blocked = planner_case(
        ("resource-restart-new-session-rate", Some("ResourceRateLimited")),
        &recovered,
        record,
        now,
        true,
    )?;
    Ok(crate::preserves_rail::record("federation-resource-recovery-case-v1", vec![
        first,
        blocked,
        crate::preserves_rail::string("retained content-keyed reservation denies even with empty peer query history"),
    ]))
}
