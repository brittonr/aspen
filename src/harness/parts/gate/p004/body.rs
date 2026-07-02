
fn validate_gate_chain_verify_receipt(
    value: &IoValue,
    link: &crate::evidence_chain::ChainLink,
    range_predicate_ref: &str,
    predicate_receipt_refs: &[String],
) -> Result<()> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("gate chain evidence missing chain verify receipt"))?;
    let schema = required_string(&receipt[0], "chain verify receipt schema")?;
    if schema != EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chain verify receipt schema {schema}; expected {EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "chain verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "gate chain verify receipt decision must be pass, got {decision}"
        )));
    }
    let anchor_ref = required_record_optional_hash(&receipt[3], "anchor", "chain verify anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("gate chain verify receipt missing anchor"))?;
    let expected_head = required_record_optional_hash(&receipt[4], "expected-head", "chain verify expected head")?
        .ok_or_else(|| MoltenError::invalid_harness("gate chain verify receipt missing expected head"))?;
    if anchor_ref != link.link_ref || expected_head != link.link_ref {
        return Err(MoltenError::invalid_harness("gate chain verify receipt does not bind the anchored head"));
    }
    let discovered_heads = required_record_hash_sequence(&receipt[5], "discovered-heads")?;
    let verified_links = required_record_hash_sequence(&receipt[6], "verified-links")?;
    let payload_refs = required_record_hash_sequence(&receipt[7], "payloads")?;
    let predicate_refs = required_record_hash_sequence(&receipt[8], "predicates")?;
    if discovered_heads != vec![link.link_ref.clone()] || verified_links != vec![link.link_ref.clone()] {
        return Err(MoltenError::invalid_harness(
            "gate chain verify receipt must cover exactly the anchored report link",
        ));
    }
    if payload_refs != vec![link.payload.artifact_ref.clone()] {
        return Err(MoltenError::invalid_harness(
            "gate chain verify receipt payload refs do not bind the report payload",
        ));
    }
    if predicate_refs != predicate_receipt_refs {
        return Err(MoltenError::invalid_harness(
            "gate chain verify receipt predicate refs do not match embedded predicate receipts",
        ));
    }
    if !predicate_refs.iter().any(|predicate_ref| predicate_ref == range_predicate_ref) {
        return Err(MoltenError::invalid_harness("gate chain verify receipt does not bind checkpoint range predicate"));
    }
    Ok(())
}

fn require_chain_predicate<'a>(
    predicates: &'a [crate::evidence_chain::ChainPredicateReceipt],
    expected_ref: &str,
    expected_kind: &str,
) -> Result<&'a crate::evidence_chain::ChainPredicateReceipt> {
    let predicate = predicates
        .iter()
        .find(|predicate| predicate.receipt_ref == expected_ref)
        .ok_or_else(|| MoltenError::invalid_harness("gate chain evidence missing checkpoint range predicate"))?;
    if predicate.predicate != expected_kind || predicate.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "gate chain predicate {expected_ref} must be a passing {expected_kind} receipt"
        )));
    }
    Ok(predicate)
}

fn require_chain_predicate_kind(
    predicates: &[crate::evidence_chain::ChainPredicateReceipt],
    expected_kind: &str,
) -> Result<()> {
    if predicates
        .iter()
        .any(|predicate| predicate.predicate == expected_kind && predicate.decision == "pass")
    {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "gate chain evidence missing passing {expected_kind} predicate receipt"
        )))
    }
}

#[derive(Debug, Clone)]
struct TurnJournalBuilder {
    actor_id: String,
    chain: crate::evidence_chain::ChainScope,
    links: Vec<crate::evidence_chain::ChainLink>,
    link_values: Vec<IoValue>,
    payload_refs: Vec<String>,
}

struct LinkEnds<'a> {
    anchor_ref: &'a String,
    head_ref: &'a String,
}

fn build_turn_journals(report: &super::schema::Report) -> Result<TurnJournalEvidence> {
    let suite = super::schema::parse_suite(&report.suite_value)?;
    if suite.steps.len() != report.observations.len() {
        return Err(MoltenError::invalid_harness("turn journal evidence requires one observation per suite step"));
    }
    let mut builders: OrderedMap<String, TurnJournalBuilder> = OrderedMap::new();
    for (position, observation) in report.observations.iter().enumerate() {
        append_turn_journal_observation(&mut builders, report, observation, &suite.steps[position])?;
    }

    let mut journals = Vec::with_capacity(builders.len());
    for builder in builders.into_values() {
        journals.push(build_turn_journal_chain(builder, report)?);
    }
    let mut evidence = TurnJournalEvidence {
        aggregate_ref: String::new(),
        journals,
    };
    evidence.aggregate_ref = canonical_hash(&turn_journals_value(&evidence))?;
    Ok(evidence)
}

fn append_turn_journal_observation(
    builders: &mut OrderedMap<String, TurnJournalBuilder>,
    report: &super::schema::Report,
    observation: &super::schema::Observation,
    step: &super::core::CoreStep,
) -> Result<()> {
    let actor_id = step.primary_actor().to_string();
    let computed_step_ref = canonical_hash(&super::schema::step_value(step))?;
    if observation.step_ref != computed_step_ref {
        return Err(MoltenError::invalid_harness(format!(
            "turn journal observation {} step ref does not match embedded suite step",
            observation.index
        )));
    }
    let builder = builders.entry(actor_id.clone()).or_insert_with(|| TurnJournalBuilder {
        actor_id: actor_id.clone(),
        chain: crate::evidence_chain::ChainScope::new(
            "harness-turn-journal",
            actor_id.clone(),
            report.report_ref.clone(),
        ),
        links: Vec::new(),
        link_values: Vec::new(),
        payload_refs: Vec::new(),
    });
    let observation_ref = observation.observation_ref.clone();
    let payload = crate::evidence_chain::ChainPayload::new(
        "turn-observation",
        observation_ref.clone(),
        HARNESS_OBSERVATION_SCHEMA,
    );
    let context_refs = turn_journal_context_refs(report, observation, &actor_id)?;
    let trellis_input_ref = turn_journal_trellis_input_ref(observation, &actor_id)?;
    let producer = turn_journal_producer()?;
    let input = if let Some(previous) = builder.links.last() {
        crate::evidence_chain::ChainLinkInput::append(previous, payload, context_refs, producer, trellis_input_ref)
    } else {
        crate::evidence_chain::ChainLinkInput::genesis(
            builder.chain.clone(),
            payload,
            context_refs,
            producer,
            trellis_input_ref,
        )
    };
    let link_value = crate::evidence_chain::chain_link_value(&input);
    let link = crate::evidence_chain::parse_chain_link(&link_value)?;
    builder.payload_refs.push(observation_ref);
    builder.link_values.push(link_value);
    builder.links.push(link);
    Ok(())
}

fn turn_journal_trellis_input_ref(observation: &super::schema::Observation, actor_id: &str) -> Result<String> {
    canonical_hash(&record("turn-journal-input", vec![
        string(actor_id),
        u64_value(observation.index),
        record("observation", vec![string(&observation.observation_ref)]),
        string(&observation.step_ref),
        string(&observation.before_state_hash),
        string(&observation.after_state_hash),
        record("event-refs", vec![sequence(observation.event_refs.iter().map(string).collect())]),
    ]))
}

fn build_turn_journal_chain(
    builder: TurnJournalBuilder,
    report: &super::schema::Report,
) -> Result<TurnJournalChainEvidence> {
    let link_refs = builder.links.iter().map(|link| link.link_ref.clone()).collect::<Vec<_>>();
    let ends = link_ends(&link_refs)?;
    let context_refs = actor_refs(report, &builder.actor_id)?;
    let predicate_values = predicate_values(&link_refs, &builder.payload_refs, &context_refs, &ends);
    let predicate_receipt_refs = predicate_refs(&predicate_values)?;
    let verify_receipt_value =
        verify_value(&builder.chain, &link_refs, &builder.payload_refs, &ends, &predicate_receipt_refs);
    let verify_receipt_ref = canonical_hash(&verify_receipt_value)?;
    Ok(TurnJournalChainEvidence {
        actor_id: builder.actor_id,
        link_refs,
        payload_refs: builder.payload_refs,
        verify_receipt_ref,
        predicate_receipt_refs,
        link_values: builder.link_values,
        verify_receipt_value,
        predicate_values,
    })
}

fn link_ends(link_refs: &[String]) -> Result<LinkEnds<'_>> {
    let Some(anchor_ref) = link_refs.first() else {
        return Err(MoltenError::invalid_harness("turn journal chain must contain at least one link"));
    };
    let Some(head_ref) = link_refs.last() else {
        return Err(MoltenError::invalid_harness("turn journal chain must contain a head link"));
    };
    Ok(LinkEnds { anchor_ref, head_ref })
}

fn actor_refs(report: &super::schema::Report, actor_id: &str) -> Result<Vec<String>> {
    Ok(vec![
        report.report_ref.clone(),
        report.suite_ref.clone(),
        canonical_hash(&record("turn-journal-actor", vec![string(actor_id)]))?,
    ])
}

fn predicate_values(
    link_refs: &[String],
    payload_refs: &[String],
    context_refs: &[String],
    ends: &LinkEnds<'_>,
) -> Vec<IoValue> {
    let segment_checks = vec![
        crate::evidence_chain::ChainCheck::pass("segment-contiguity"),
        crate::evidence_chain::ChainCheck::pass("canonical-link-order"),
    ];
    let fork_checks = vec![
        crate::evidence_chain::ChainCheck::pass("fork-policy-profile"),
        crate::evidence_chain::ChainCheck::pass("fork-evidence-binding"),
    ];
    let anchor_subject_refs = vec![ends.anchor_ref.clone(), ends.head_ref.clone()];
    let anchor_checks = vec![
        crate::evidence_chain::ChainCheck::pass("anchor-descent"),
        crate::evidence_chain::ChainCheck::pass("head-binding"),
    ];
    vec![
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
            decision: "pass",
            subject_refs: link_refs,
            input_refs: payload_refs,
            context_refs,
            checks: &segment_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
            decision: "pass",
            subject_refs: std::slice::from_ref(ends.head_ref),
            input_refs: link_refs,
            context_refs,
            checks: &fork_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
            decision: "pass",
            subject_refs: &anchor_subject_refs,
            input_refs: link_refs,
            context_refs,
            checks: &anchor_checks,
        }),
    ]
}

fn predicate_refs(values: &[IoValue]) -> Result<Vec<String>> {
    Ok(values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|receipt| receipt.receipt_ref)
        .collect())
}
