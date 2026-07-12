
pub fn publish_chain_segment(input: &PublishChainSegmentInput<'_>) -> Result<ChainSegment> {
    let root = open_capability_exchange_root(input.iroh_root)?;
    publish_chain_segment_with_root(&PublishChainSegmentInput {
        iroh_root: &root,
        ledger_root: input.ledger_root,
        chain: input.chain,
        anchor_ref: input.anchor_ref,
        expected_head: input.expected_head,
        node: input.node,
        fork_policy: input.fork_policy,
    })
}

pub fn publish_chain_segment_with_root(
    input: &PublishChainSegmentInput<'_, CapabilityExchangeRoot>,
) -> Result<ChainSegment> {
    let bundle = build_chain_segment_bundle_value(
        input.ledger_root,
        input.chain,
        input.anchor_ref,
        input.expected_head,
        input.fork_policy,
    )?;
    let parsed = parse_chain_segment_bundle(&bundle, input.fork_policy)?;
    let bytes = canonical_bytes(&bundle)?;
    let blob_ref = content_ref_from_bytes(&bytes);
    if blob_ref != parsed.bundle_ref {
        return Err(MoltenError::invalid_harness("Iroh publish chain bundle blob ref does not match bundle ref"));
    }
    input.iroh_root.root().write(&blob_store_path(&parsed.bundle_ref)?, &bytes)?;
    let ticket = format!("iroh-local-chain:{}", parsed.bundle_ref);
    let receipt_value = chain_receipt_value(&ChainReceiptValueInput {
        operation: "publish",
        decision: "pass",
        node: input.node,
        peer: "local",
        ticket: &ticket,
        bundle_ref: &parsed.bundle_ref,
        verify_receipt_refs: &parsed.verify_receipt_refs,
        checkpoint_refs: &parsed.checkpoint_refs,
    });
    Ok(ChainSegment {
        ticket,
        bundle_ref: parsed.bundle_ref,
        chain: parsed.chain,
        anchor_ref: parsed.anchor_ref,
        head_ref: parsed.head_ref,
        receipt_value,
    })
}

pub fn fetch_chain_segment(input: &FetchChainSegmentInput<'_>) -> Result<ChainSegment> {
    let root = open_capability_exchange_root(input.iroh_root)?;
    fetch_chain_segment_with_root(&FetchChainSegmentInput {
        iroh_root: &root,
        ticket: input.ticket,
        expected_bundle_ref: input.expected_bundle_ref,
        peer: input.peer,
        ledger_root: input.ledger_root,
        fork_policy: input.fork_policy,
    })
}

pub fn fetch_chain_segment_with_root(
    input: &FetchChainSegmentInput<'_, CapabilityExchangeRoot>,
) -> Result<ChainSegment> {
    let advertised_ref = input.ticket.strip_prefix("iroh-local-chain:").ok_or_else(|| {
        MoltenError::invalid_harness("unsupported Iroh chain ticket; expected iroh-local-chain:<bundle-ref>")
    })?;
    if let Some(expected_bundle_ref) = input.expected_bundle_ref
        && expected_bundle_ref != advertised_ref
    {
        return Err(MoltenError::invalid_harness(format!(
            "Iroh chain ticket advertises bundle {advertised_ref}, expected {expected_bundle_ref}"
        )));
    }
    let bytes = input.iroh_root.root().read(&blob_store_path(advertised_ref)?)?;
    let bundle = parse_canonical_bytes(&bytes)?;
    let parsed = parse_chain_segment_bundle(&bundle, input.fork_policy)?;
    if parsed.bundle_ref != advertised_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Iroh fetched chain bundle hashes to {}, expected advertised bundle {advertised_ref}",
            parsed.bundle_ref
        )));
    }
    for artifact in &parsed.artifacts {
        crate::ledger::import_artifact(input.ledger_root, &artifact.value)?;
    }
    crate::ledger::import_artifact(input.ledger_root, &bundle)?;
    let receipt_value = chain_receipt_value(&ChainReceiptValueInput {
        operation: "fetch",
        decision: "pass",
        node: "local",
        peer: input.peer,
        ticket: input.ticket,
        bundle_ref: &parsed.bundle_ref,
        verify_receipt_refs: &parsed.verify_receipt_refs,
        checkpoint_refs: &parsed.checkpoint_refs,
    });
    crate::ledger::import_artifact(input.ledger_root, &receipt_value)?;
    Ok(ChainSegment {
        ticket: input.ticket.to_string(),
        bundle_ref: parsed.bundle_ref,
        chain: parsed.chain,
        anchor_ref: parsed.anchor_ref,
        head_ref: parsed.head_ref,
        receipt_value,
    })
}

fn build_chain_segment_bundle_value(
    ledger_root: &Path,
    chain: &crate::evidence_chain::ChainScope,
    anchor_ref: Option<&str>,
    expected_head: Option<&str>,
    fork_policy: crate::evidence_chain::ChainForkPolicy,
) -> Result<IoValue> {
    let verified = crate::evidence_chain::verify_chain_segment_with_policy(
        ledger_root,
        chain,
        anchor_ref,
        expected_head,
        fork_policy,
    )?;
    let index = crate::evidence_chain::build_chain_index(ledger_root)?;
    let mut artifacts = OrderedMap::<String, IoValue>::new();
    for link_ref in &verified.verified_links {
        add_ledger_artifact(ledger_root, &mut artifacts, link_ref)?;
        if let Some(link) = index.links_by_ref.get(link_ref) {
            add_ledger_artifact(ledger_root, &mut artifacts, &link.payload.artifact_ref)?;
        }
    }
    for predicate_ref in &verified.predicate_receipt_refs {
        add_ledger_artifact(ledger_root, &mut artifacts, predicate_ref)?;
    }
    add_ledger_artifact(ledger_root, &mut artifacts, &verified.receipt_ref)?;
    for fork_ref in index.fork_evidence_for_chain(chain) {
        add_ledger_artifact(ledger_root, &mut artifacts, &fork_ref)?;
    }
    for anchor_ref in index.anchors_for_chain(chain) {
        add_ledger_artifact(ledger_root, &mut artifacts, &anchor_ref)?;
    }
    let mut checkpoint_refs = Vec::new();
    for checkpoint_ref in index.checkpoints_for_chain(chain) {
        let checkpoint_value = crate::ledger::read_artifact(ledger_root, &checkpoint_ref)?;
        let checkpoint = crate::evidence_chain::parse_chain_checkpoint(&checkpoint_value)?;
        add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint_ref)?;
        for checkpoint_link_ref in [&checkpoint.anchor_link_ref, &checkpoint.head_ref] {
            add_ledger_artifact(ledger_root, &mut artifacts, checkpoint_link_ref)?;
            let checkpoint_link_value = crate::ledger::read_artifact(ledger_root, checkpoint_link_ref)?;
            let checkpoint_link = crate::evidence_chain::parse_chain_link(&checkpoint_link_value)?;
            add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint_link.payload.artifact_ref)?;
        }
        add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint.verify_receipt_ref)?;
        add_ledger_artifact(ledger_root, &mut artifacts, &checkpoint.range_predicate_ref)?;
        push_bounded(
            &mut checkpoint_refs,
            checkpoint_ref,
            MAX_CHAIN_BUNDLE_CHECKPOINTS,
            "chain bundle checkpoint refs",
        )?;
    }
    let artifact_values = artifacts
        .iter()
        .map(|(artifact_ref, value)| {
            record("artifact", vec![
                string(crate::ledger::artifact_kind(value)),
                string(artifact_ref),
                value.clone(),
            ])
        })
        .collect::<Vec<_>>();
    Ok(chain_segment_bundle_value(&ChainSegmentBundleValueInput {
        chain,
        anchor_ref,
        head_ref: expected_head,
        artifacts: &artifact_values,
        verify_receipt_refs: std::slice::from_ref(&verified.receipt_ref),
        checkpoint_refs: &checkpoint_refs,
    }))
}

fn add_ledger_artifact(root: &Path, artifacts: &mut OrderedMap<String, IoValue>, artifact_ref: &str) -> Result<()> {
    if !artifacts.contains_key(artifact_ref) {
        artifacts.insert(artifact_ref.to_string(), crate::ledger::read_artifact(root, artifact_ref)?);
    }
    Ok(())
}

fn chain_segment_bundle_value(input: &ChainSegmentBundleValueInput<'_>) -> IoValue {
    record("chain-segment-bundle-v1", vec![
        string(EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA),
        chain_scope_value(input.chain),
        record("anchor", vec![optional_ref_value(input.anchor_ref)]),
        record("head", vec![optional_ref_value(input.head_ref)]),
        record("artifacts", vec![sequence(input.artifacts.to_vec())]),
        record("verify-receipts", vec![sequence(input.verify_receipt_refs.iter().map(string).collect())]),
        record("checkpoints", vec![sequence(input.checkpoint_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("content-addressed-artifacts"), string("pass")]),
            record("check", vec![string("chain-segment-verified"), string("pass")]),
            record("check", vec![string("predicate-receipts-bound"), string("pass")]),
            record("check", vec![string("checkpoint-binding"), string("pass")]),
            record("check", vec![string("transport-does-not-grant-trust"), string("pass")]),
        ])]),
    ])
}

fn parse_chain_segment_bundle(
    value: &IoValue,
    fork_policy: crate::evidence_chain::ChainForkPolicy,
) -> Result<ChainSegmentBundle> {
    crate::preserves_rail::validate_boundary_schema(
        value,
        &crate::preserves_rail::EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_SCHEMA,
    )?;
    let bundle = value
        .collect_simple_record("chain-segment-bundle-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-segment-bundle-v1 ...>"))?;
    require_schema(&bundle[0], EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA, "chain segment bundle schema")?;
    let chain = parse_chain_scope(&bundle[1])?;
    let anchor_ref = parse_optional_ref_field(&bundle[2], "anchor")?;
    let head_ref = parse_optional_ref_field(&bundle[3], "head")?;
    let artifacts = parse_chain_bundle_artifacts(&bundle[4])?;
    let verify_receipt_refs = parse_ref_sequence_field(&bundle[5], "verify-receipts")?;
    let checkpoint_refs = parse_ref_sequence_field(&bundle[6], "checkpoints")?;
    let checks = parse_check_names(&bundle[7])?;
    require_check(&checks, "content-addressed-artifacts")?;
    require_check(&checks, "chain-segment-verified")?;
    require_check(&checks, "predicate-receipts-bound")?;
    require_check(&checks, "transport-does-not-grant-trust")?;
    validate_chain_bundle(ValidateChainBundleInput {
        chain: &chain,
        anchor_ref: anchor_ref.as_deref(),
        head_ref: head_ref.as_deref(),
        artifacts: &artifacts,
        verify_receipt_refs: &verify_receipt_refs,
        checkpoint_refs: &checkpoint_refs,
        fork_policy,
    })?;
    Ok(ChainSegmentBundle {
        bundle_ref: canonical_hash(value)?,
        chain,
        anchor_ref,
        head_ref,
        artifacts,
        verify_receipt_refs,
        checkpoint_refs,
    })
}

struct Parts<'a> {
    artifacts: OrderedMap<String, &'a ChainBundleArtifact>,
    links: OrderedMap<String, crate::evidence_chain::ChainLink>,
    predicates: OrderedMap<String, crate::evidence_chain::ChainPredicateReceipt>,
    has_forks: bool,
}

struct PrimaryCheck<'a> {
    primary: &'a ParsedChainVerifyReceipt,
    chain: &'a crate::evidence_chain::ChainScope,
    anchor_ref: Option<&'a str>,
    head_ref: Option<&'a str>,
    fork_policy: crate::evidence_chain::ChainForkPolicy,
    has_forks: bool,
}

fn validate_chain_bundle(input: ValidateChainBundleInput<'_>) -> Result<()> {
    let parts = parsed_parts(input.artifacts)?;
    anchors(input.chain, input.artifacts)?;
    let receipts = receipts(input.verify_receipt_refs, &parts.artifacts)?;
    let primary = primary(&receipts)?;
    check_primary(PrimaryCheck {
        primary,
        chain: input.chain,
        anchor_ref: input.anchor_ref,
        head_ref: input.head_ref,
        fork_policy: input.fork_policy,
        has_forks: parts.has_forks,
    })?;
    validate_chain_links(&parts.links, primary, input.anchor_ref, input.head_ref)?;
    payloads(primary, &parts.artifacts)?;
    predicates(primary, &parts.predicates)?;
    let checkpoints = checkpoints(input.checkpoint_refs, &parts.artifacts)?;
    validate_bundle_checkpoints(input.chain, &checkpoints, &parts.artifacts, &parts.links, &parts.predicates)?;
    Ok(())
}

fn parsed_parts(artifacts: &[ChainBundleArtifact]) -> Result<Parts<'_>> {
    let parts = artifacts
        .iter()
        .map(|artifact| (artifact.artifact_ref.clone(), artifact))
        .collect::<OrderedMap<_, _>>();
    let links = artifacts
        .iter()
        .filter(|artifact| artifact.kind == "chain-link")
        .map(|artifact| {
            crate::evidence_chain::parse_chain_link(&artifact.value).map(|link| (link.link_ref.clone(), link))
        })
        .collect::<Result<OrderedMap<_, _>>>()?;
    let predicates = artifacts
        .iter()
        .filter(|artifact| artifact.kind == "chain-predicate-receipt")
        .map(|artifact| {
            crate::evidence_chain::parse_chain_predicate_receipt(&artifact.value)
                .map(|receipt| (receipt.receipt_ref.clone(), receipt))
        })
        .collect::<Result<OrderedMap<_, _>>>()?;
    let mut has_forks = false;
    for artifact in artifacts.iter().filter(|artifact| artifact.kind == "chain-fork-evidence") {
        crate::evidence_chain::parse_chain_fork_evidence(&artifact.value)?;
        has_forks = true;
    }
    Ok(Parts {
        artifacts: parts,
        links,
        predicates,
        has_forks,
    })
}

fn anchors(chain: &crate::evidence_chain::ChainScope, artifacts: &[ChainBundleArtifact]) -> Result<()> {
    for artifact in artifacts.iter().filter(|artifact| artifact.kind == "chain-anchor") {
        let anchor = crate::evidence_chain::parse_chain_anchor(&artifact.value)?;
        if anchor.chain != *chain {
            return Err(MoltenError::invalid_harness("chain bundle anchor belongs to a different chain"));
        }
    }
    Ok(())
}
