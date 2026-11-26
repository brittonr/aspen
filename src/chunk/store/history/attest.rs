//! Predicate and verify receipts attesting that a lineage chain is gap-free, fork-free, and
//! anchored.

pub(super) struct PassEvidence {
    pub(super) predicate_values: Vec<preserves::IOValue>,
    pub(super) predicate_refs: Vec<String>,
    pub(super) verify_value: preserves::IOValue,
    pub(super) verify_ref: String,
}

struct Ends {
    head_ref: String,
    anchor_ref: String,
}

pub(super) fn pass_evidence(
    chain: &crate::evidence_chain::ChainScope,
    manifest: &crate::blocks::ChunkManifest,
    link_refs: &[String],
    receipt_refs: &[String],
) -> crate::error::Result<PassEvidence> {
    let ends = chain_ends(link_refs)?;
    let predicate_values = predicate_set(PredicateInput {
        manifest,
        link_refs,
        receipt_refs,
        ends: &ends,
    });
    let predicate_refs = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<crate::error::Result<Vec<_>>>()?
        .into_iter()
        .map(|receipt| receipt.receipt_ref)
        .collect::<Vec<_>>();
    let verify_value = verify_value(VerifyInput {
        chain,
        link_refs,
        receipt_refs,
        ends: &ends,
        predicate_refs: &predicate_refs,
    });
    let verify_ref = crate::preserves_rail::canonical_hash(&verify_value)?;
    Ok(PassEvidence {
        predicate_values,
        predicate_refs,
        verify_value,
        verify_ref,
    })
}

fn chain_ends(link_refs: &[String]) -> crate::error::Result<Ends> {
    let head_ref = link_refs
        .last()
        .cloned()
        .ok_or_else(|| crate::error::Failure::invalid_harness("chunk lineage requires at least one chain link"))?;
    let anchor_ref = link_refs
        .first()
        .cloned()
        .ok_or_else(|| crate::error::Failure::invalid_harness("chunk lineage requires at least one chain link"))?;
    Ok(Ends { head_ref, anchor_ref })
}

struct PredicateInput<'a> {
    manifest: &'a crate::blocks::ChunkManifest,
    link_refs: &'a [String],
    receipt_refs: &'a [String],
    ends: &'a Ends,
}

fn predicate_set(input: PredicateInput<'_>) -> Vec<preserves::IOValue> {
    let context_refs = vec![
        input.manifest.manifest_ref.clone(),
        input.manifest.root_ref.clone(),
        input.manifest.metadata_ref.clone(),
    ];
    let segment_checks = vec![
        crate::evidence_chain::ChainCheck::pass("segment-contiguity"),
        crate::evidence_chain::ChainCheck::pass("canonical-link-order"),
    ];
    let fork_checks = vec![
        crate::evidence_chain::ChainCheck::pass("fork-policy-profile"),
        crate::evidence_chain::ChainCheck::pass("fork-evidence-binding"),
    ];
    let anchor_subject_refs = vec![input.ends.anchor_ref.clone(), input.ends.head_ref.clone()];
    let anchor_checks = vec![
        crate::evidence_chain::ChainCheck::pass("anchor-descent"),
        crate::evidence_chain::ChainCheck::pass("head-binding"),
    ];
    let checkpoint_checks = vec![
        crate::evidence_chain::ChainCheck::pass("checkpoint-range-coverage"),
        crate::evidence_chain::ChainCheck::pass("verified-range"),
    ];
    vec![
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
            decision: "pass",
            subject_refs: input.link_refs,
            input_refs: input.receipt_refs,
            context_refs: &context_refs,
            checks: &segment_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
            decision: "pass",
            subject_refs: std::slice::from_ref(&input.ends.head_ref),
            input_refs: input.link_refs,
            context_refs: &context_refs,
            checks: &fork_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
            decision: "pass",
            subject_refs: &anchor_subject_refs,
            input_refs: input.link_refs,
            context_refs: &context_refs,
            checks: &anchor_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
            decision: "pass",
            subject_refs: input.link_refs,
            input_refs: input.receipt_refs,
            context_refs: &context_refs,
            checks: &checkpoint_checks,
        }),
    ]
}

struct VerifyInput<'a> {
    chain: &'a crate::evidence_chain::ChainScope,
    link_refs: &'a [String],
    receipt_refs: &'a [String],
    ends: &'a Ends,
    predicate_refs: &'a [String],
}

fn verify_value(input: VerifyInput<'_>) -> preserves::IOValue {
    let verify_diagnostics = Vec::new();
    let verify_receipt = crate::evidence_chain::ChainVerifyReceiptValueInput {
        decision: "pass",
        chain: input.chain,
        anchor_ref: Some(&input.ends.anchor_ref),
        expected_head: Some(&input.ends.head_ref),
        discovered_heads: std::slice::from_ref(&input.ends.head_ref),
        verified_links: input.link_refs,
        payload_refs: input.receipt_refs,
        diagnostics: &verify_diagnostics,
    };
    crate::evidence_chain::chain_verify_receipt_value_with_policy(
        &crate::evidence_chain::ChainVerifyReceiptPolicyValueInput {
            receipt: verify_receipt,
            predicate_receipt_refs: input.predicate_refs,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        },
    )
}
