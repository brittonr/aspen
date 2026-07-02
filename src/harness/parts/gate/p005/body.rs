
fn verify_value(
    chain: &crate::evidence_chain::ChainScope,
    link_refs: &[String],
    payload_refs: &[String],
    ends: &LinkEnds<'_>,
    predicate_receipt_refs: &[String],
) -> IoValue {
    let verify_diagnostics = Vec::new();
    let verify_receipt = crate::evidence_chain::ChainVerifyReceiptValueInput {
        decision: "pass",
        chain,
        anchor_ref: Some(ends.anchor_ref.as_str()),
        expected_head: Some(ends.head_ref.as_str()),
        discovered_heads: std::slice::from_ref(ends.head_ref),
        verified_links: link_refs,
        payload_refs,
        diagnostics: &verify_diagnostics,
    };
    crate::evidence_chain::chain_verify_receipt_value_with_policy(
        &crate::evidence_chain::ChainVerifyReceiptPolicyValueInput {
            receipt: verify_receipt,
            predicate_receipt_refs,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        },
    )
}

fn turn_journal_context_refs(
    report: &super::schema::Report,
    observation: &super::schema::Observation,
    actor_id: &str,
) -> Result<Vec<crate::evidence_chain::ChainContextRef>> {
    let mut refs = vec![
        crate::evidence_chain::ChainContextRef::new("report", report.report_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("suite", report.suite_ref.clone()),
        crate::evidence_chain::ChainContextRef::new(
            "actor",
            canonical_hash(&record("turn-journal-actor", vec![string(actor_id)]))?,
        ),
        crate::evidence_chain::ChainContextRef::new("observation", observation.observation_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("step", observation.step_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("before-state", observation.before_state_hash.clone()),
        crate::evidence_chain::ChainContextRef::new("after-state", observation.after_state_hash.clone()),
    ];
    for (event, event_ref) in observation.events.iter().zip(observation.event_refs.iter()) {
        let computed_event_ref = canonical_hash(event)?;
        if computed_event_ref != *event_ref {
            return Err(MoltenError::invalid_harness(
                "turn journal observation event refs do not match canonical events",
            ));
        }
        let label = match super::schema::event_boundary(event) {
            super::schema::EventBoundary::PolicyDecision => "admission",
            super::schema::EventBoundary::EffectRequest | super::schema::EventBoundary::EffectResponse => "effect-log",
            _ => "trace",
        };
        refs.push(crate::evidence_chain::ChainContextRef::new(label, event_ref.clone()));
    }
    Ok(refs)
}

fn turn_journal_producer() -> Result<crate::evidence_chain::ChainProducer> {
    Ok(crate::evidence_chain::ChainProducer::new(
        "molten-turn-journal",
        canonical_hash(&record("turn-journal-producer-key", vec![string("molten")]))?,
    ))
}

fn turn_journals_value(evidence: &TurnJournalEvidence) -> IoValue {
    record("turn-journals", vec![
        record("profile", vec![string("per-actor-local-turn-journal")]),
        record("journals", vec![sequence(evidence.journals.iter().map(turn_journal_value).collect())]),
        record("checks", vec![sequence(
            [
                "turn-journal-chains",
                "turn-journal-input-binding",
                "turn-journal-admission-binding",
                "turn-journal-state-binding",
                "turn-journal-no-global-head",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

fn turn_journal_value(journal: &TurnJournalChainEvidence) -> IoValue {
    record("turn-journal", vec![
        record("actor", vec![string(&journal.actor_id)]),
        record("links", vec![sequence(journal.link_values.clone())]),
        record("verify-receipt", vec![journal.verify_receipt_value.clone()]),
        record("predicates", vec![sequence(journal.predicate_values.clone())]),
        record("checks", vec![sequence(
            [
                "turn-journal-chains",
                "turn-journal-input-binding",
                "turn-journal-admission-binding",
                "turn-journal-state-binding",
                "turn-journal-no-global-head",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

fn parse_turn_journals(value: &Value<IoValue>, report_ref: &str, suite_ref: &str) -> Result<TurnJournalEvidence> {
    let value = value_to_iovalue(value);
    let journals_record = simple_record(&value, "turn-journals", 3)?;
    let profile = required_record_string(&journals_record[0], "profile", "turn journal profile")?;
    if profile != "per-actor-local-turn-journal" {
        return Err(MoltenError::invalid_harness(format!("unsupported turn journal profile {profile}")));
    }
    let journal_values = required_record_values(&journals_record[1], "journals")?;
    let checks = parse_checks(&journals_record[2])?;
    require_turn_journal_checks(&checks)?;
    let journals = parse_turn_journal_set(&journal_values, report_ref, suite_ref)?;
    Ok(TurnJournalEvidence {
        aggregate_ref: canonical_hash(&value)?,
        journals,
    })
}

fn require_turn_journal_checks(checks: &[String]) -> Result<()> {
    require_check(checks, "turn-journal-chains")?;
    require_check(checks, "turn-journal-input-binding")?;
    require_check(checks, "turn-journal-admission-binding")?;
    require_check(checks, "turn-journal-state-binding")?;
    require_check(checks, "turn-journal-no-global-head")?;
    Ok(())
}

fn parse_turn_journal_set(
    journal_values: &[IoValue],
    report_ref: &str,
    suite_ref: &str,
) -> Result<Vec<TurnJournalChainEvidence>> {
    let mut journals = Vec::with_capacity(journal_values.len());
    let mut actor_ids = OrderedMap::new();
    for journal_value in journal_values {
        let journal = parse_turn_journal(journal_value, report_ref, suite_ref)?;
        if actor_ids.insert(journal.actor_id.clone(), ()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate turn journal for actor {}", journal.actor_id)));
        }
        journals.push(journal);
    }
    if journals.is_empty() {
        return Err(MoltenError::invalid_harness("turn journal evidence must contain at least one actor journal"));
    }
    Ok(journals)
}

fn parse_turn_journal(value: &IoValue, report_ref: &str, suite_ref: &str) -> Result<TurnJournalChainEvidence> {
    let journal_record = simple_record(value, "turn-journal", 5)?;
    let actor_id = required_record_string(&journal_record[0], "actor", "turn journal actor")?;
    let link_values = required_record_values(&journal_record[1], "links")?;
    let verify_receipt_value = required_record_value(&journal_record[2], "verify-receipt")?;
    let predicate_values = required_record_values(&journal_record[3], "predicates")?;
    let checks = parse_checks(&journal_record[4])?;
    require_turn_journal_checks(&checks)?;
    let parsed_links = parse_turn_journal_links(&link_values, &actor_id, report_ref, suite_ref)?;
    let predicate_receipts = parse_turn_journal_predicates(&predicate_values)?;
    let predicate_receipt_refs =
        predicate_receipts.iter().map(|receipt| receipt.receipt_ref.clone()).collect::<Vec<_>>();
    validate_turn_journal_verify_receipt(
        &verify_receipt_value,
        &parsed_links.links[0].chain,
        &parsed_links.link_refs,
        &parsed_links.payload_refs,
        &predicate_receipt_refs,
    )?;
    Ok(TurnJournalChainEvidence {
        actor_id,
        link_refs: parsed_links.link_refs,
        payload_refs: parsed_links.payload_refs,
        verify_receipt_ref: canonical_hash(&verify_receipt_value)?,
        predicate_receipt_refs,
        link_values,
        verify_receipt_value,
        predicate_values,
    })
}

struct ParsedTurnJournalLinks {
    links: Vec<crate::evidence_chain::ChainLink>,
    link_refs: Vec<String>,
    payload_refs: Vec<String>,
}

fn parse_turn_journal_links(
    link_values: &[IoValue],
    actor_id: &str,
    report_ref: &str,
    suite_ref: &str,
) -> Result<ParsedTurnJournalLinks> {
    if link_values.is_empty() {
        return Err(MoltenError::invalid_harness("turn journal must contain at least one link"));
    }
    let mut links = Vec::with_capacity(link_values.len());
    let mut link_refs = Vec::with_capacity(link_values.len());
    let mut payload_refs = Vec::with_capacity(link_values.len());
    for (position, link_value) in link_values.iter().enumerate() {
        let link = crate::evidence_chain::parse_chain_link(link_value)?;
        validate_turn_journal_link(TurnJournalLinkValidation {
            link: &link,
            position,
            link_refs: &link_refs,
            actor_id,
            report_ref,
            suite_ref,
        })?;
        payload_refs.push(link.payload.artifact_ref.clone());
        link_refs.push(link.link_ref.clone());
        links.push(link);
    }
    Ok(ParsedTurnJournalLinks {
        links,
        link_refs,
        payload_refs,
    })
}

struct TurnJournalLinkValidation<'a> {
    link: &'a crate::evidence_chain::ChainLink,
    position: usize,
    link_refs: &'a [String],
    actor_id: &'a str,
    report_ref: &'a str,
    suite_ref: &'a str,
}

fn validate_turn_journal_link(input: TurnJournalLinkValidation<'_>) -> Result<()> {
    if input.link.chain.scope != "harness-turn-journal"
        || input.link.chain.id != input.actor_id
        || input.link.chain.epoch != input.report_ref
    {
        return Err(MoltenError::invalid_harness(
            "turn journal link scope must be per actor and per report, not global",
        ));
    }
    if input.link.sequence != input.position as u64 {
        return Err(MoltenError::invalid_harness("turn journal link sequence is not contiguous"));
    }
    validate_turn_journal_previous_ref(input.link, input.position, input.link_refs)?;
    require_context_ref(&input.link.context_refs, "report", input.report_ref)?;
    require_context_ref(&input.link.context_refs, "suite", input.suite_ref)?;
    require_context_ref_kind(&input.link.context_refs, "step")?;
    require_context_ref_kind(&input.link.context_refs, "before-state")?;
    require_context_ref_kind(&input.link.context_refs, "after-state")?;
    require_context_ref_kind(&input.link.context_refs, "admission")?;
    require_context_ref_kind(&input.link.context_refs, "trace")?;
    Ok(())
}

fn validate_turn_journal_previous_ref(
    link: &crate::evidence_chain::ChainLink,
    position: usize,
    link_refs: &[String],
) -> Result<()> {
    if position == 0 {
        if link.previous_link_ref.is_some() {
            return Err(MoltenError::invalid_harness("turn journal genesis link must not name a previous link"));
        }
        return Ok(());
    }
    if link.previous_link_ref.as_deref() != link_refs.get(position - 1).map(String::as_str) {
        return Err(MoltenError::invalid_harness("turn journal link does not bind previous actor-local turn"));
    }
    Ok(())
}

fn parse_turn_journal_predicates(
    predicate_values: &[IoValue],
) -> Result<Vec<crate::evidence_chain::ChainPredicateReceipt>> {
    let receipts = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?;
    require_chain_predicate_kind(&receipts, crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE)?;
    require_chain_predicate_kind(&receipts, crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE)?;
    require_chain_predicate_kind(&receipts, crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE)?;
    Ok(receipts)
}
