use preserves::ValueImpl;

pub struct UcanProofsetEvidence {
    pub(super) proofset_ref: String,
    pub(super) verification_receipt_refs: Vec<String>,
    pub(super) derived_grant_refs: Vec<String>,
}

pub(super) fn ucan_proofset_value() -> preserves::IOValue {
    crate::preserves_rail::record("ucan-proofset-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA),
        crate::preserves_rail::sequence(Vec::new()),
    ])
}

pub(super) fn parse_ucan_proofset_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<UcanProofsetEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let proofset = value
        .collect_simple_record("ucan-proofset-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected UCAN proofset evidence"))?;
    let arity = proofset.fields_iter().count();
    if arity != UCAN_PROOFSET_EMPTY_ARITY && arity != UCAN_PROOFSET_VERIFIED_ARITY {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected UCAN proofset arity {UCAN_PROOFSET_EMPTY_ARITY} or {UCAN_PROOFSET_VERIFIED_ARITY}, got {arity}"
        )));
    }
    let schema = super::fields::required_string(&proofset[0], "UCAN proofset schema")?;
    if schema != crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported UCAN proofset schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA
        )));
    }
    let proof_values = super::fields::required_sequence(&proofset[1], "UCAN proofset refs")?;
    let material_value = crate::preserves_rail::record("ucan-proofset-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA),
        crate::preserves_rail::value_to_iovalue(&proofset[1]),
    ]);
    let proofset_ref = crate::preserves_rail::canonical_hash(&material_value)?;
    if proof_values.is_empty() {
        return Ok(UcanProofsetEvidence {
            proofset_ref,
            verification_receipt_refs: Vec::new(),
            derived_grant_refs: Vec::new(),
        });
    }
    if arity != UCAN_PROOFSET_VERIFIED_ARITY {
        return Err(crate::error::Failure::invalid_harness(
            "UCAN proof refs require matching UCAN verification receipts bound to the proofset",
        ));
    }
    let proof_refs = parse_ucan_proof_refs(&proofset[1])?;
    let receipts = parse_ucan_verification_receipts(&proofset[2], &proofset_ref, &proof_refs)?;
    Ok(UcanProofsetEvidence {
        proofset_ref,
        verification_receipt_refs: receipts.verification_receipt_refs,
        derived_grant_refs: receipts.derived_grant_refs,
    })
}

const UCAN_PROOFSET_EMPTY_ARITY: usize = 2;

const UCAN_PROOFSET_VERIFIED_ARITY: usize = 3;

struct ParsedUcanVerificationReceipts {
    verification_receipt_refs: Vec<String>,
    derived_grant_refs: Vec<String>,
}

fn parse_ucan_proof_refs(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let proofs = super::fields::required_sequence(value, "UCAN proofset refs")?;
    let mut refs = Vec::with_capacity(proofs.len());
    for proof in proofs.as_ref() {
        let proof_value = crate::preserves_rail::value_to_iovalue(proof);
        let proof_ref = if let Some(record) = proof_value.collect_simple_record("proof-ref", Some(1)) {
            super::fields::required_hash(&record[0], "UCAN proof ref")?
        } else {
            super::fields::required_hash(proof, "UCAN proof ref")?
        };
        refs.push(proof_ref);
    }
    Ok(refs)
}

fn parse_ucan_verification_receipts(
    value: &preserves::Value<preserves::IOValue>,
    proofset_ref: &str,
    proof_refs: &[String],
) -> crate::error::Result<ParsedUcanVerificationReceipts> {
    let receipts = super::fields::required_sequence(value, "UCAN verification receipts")?;
    if receipts.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "non-empty UCAN proofset requires at least one verification receipt",
        ));
    }
    let mut verification_receipt_refs = Vec::with_capacity(receipts.len());
    let mut derived_grant_ref_sets = Vec::with_capacity(receipts.len());
    for receipt in receipts.as_ref() {
        let receipt_value = crate::preserves_rail::value_to_iovalue(receipt);
        let parsed = crate::capability_tokens::parse_ucan_verification_receipt_value(&receipt_value)?;
        if parsed.decision != "pass" {
            return Err(crate::error::Failure::invalid_harness(
                "UCAN verification receipt must pass for harness proofset",
            ));
        }
        if parsed.proofset_ref != proofset_ref {
            return Err(crate::error::Failure::invalid_harness(
                "UCAN verification receipt proofset ref does not match harness proofset",
            ));
        }
        for proof_ref in proof_refs {
            if !parsed.proof_refs.as_slice().iter().any(|candidate| candidate == proof_ref) {
                return Err(crate::error::Failure::invalid_harness(
                    "UCAN verification receipt does not bind every harness proof ref",
                ));
            }
        }
        if parsed.derived_grant_refs.is_empty() {
            return Err(crate::error::Failure::invalid_harness(
                "UCAN verification receipt must derive at least one grant ref",
            ));
        }
        verification_receipt_refs.push(parsed.receipt_ref);
        derived_grant_ref_sets.push(parsed.derived_grant_refs);
    }
    Ok(ParsedUcanVerificationReceipts {
        verification_receipt_refs,
        derived_grant_refs: derived_grant_ref_sets.into_iter().flatten().collect(),
    })
}

pub fn admission_authority_evidence(
    capabilities: &crate::runtime::CapabilityContext,
    request: &crate::harness::nucleus::AdmissionRequest,
) -> crate::error::Result<super::observed::AdmissionAuthorityEvidence> {
    let authorization = capabilities.authorize(request);
    let grant_ref = authorization
        .grant
        .as_ref()
        .map(|grant| crate::preserves_rail::canonical_hash(&super::grants::capability_grant_value(grant)))
        .transpose()?;
    let capability_ref = crate::preserves_rail::canonical_hash(&super::grants::capabilities_value(capabilities))?;
    let request_ref = crate::preserves_rail::canonical_hash(&super::events::admission_request_value(request))?;
    let proofset_ref = crate::preserves_rail::canonical_hash(&ucan_proofset_value())?;
    let derived_grant_refs = grant_ref.iter().cloned().collect::<Vec<_>>();
    let enforcement_value = harness_basalt_authority_receipt_value(HarnessBasaltAuthorityReceiptInput {
        source: "local-fixture",
        capability_ref: &capability_ref,
        request_ref: &request_ref,
        proofset_ref: &proofset_ref,
        verification_receipt_refs: &[],
        derived_grant_refs: &derived_grant_refs,
        decision: if authorization.authorized { "pass" } else { "deny" },
        diagnostic: if authorization.authorized {
            "local fixture grant admitted as harness evidence candidate"
        } else {
            "missing local fixture grant; verified UCAN/Basalt authority absent"
        },
    });
    let basalt_enforcement_receipt_ref = crate::preserves_rail::canonical_hash(&enforcement_value)?;
    Ok(super::observed::AdmissionAuthorityEvidence {
        source: "local-fixture".to_string(),
        capability_ref,
        authorized: authorization.authorized,
        grant_ref,
        request_ref,
        proofset_ref,
        ucan_verification_receipt_refs: Vec::new(),
        derived_grant_refs,
        basalt_enforcement_receipt_ref,
        basalt_enforcement_receipt_value: enforcement_value,
    })
}

struct HarnessBasaltAuthorityReceiptInput<'a> {
    source: &'a str,
    capability_ref: &'a str,
    request_ref: &'a str,
    proofset_ref: &'a str,
    verification_receipt_refs: &'a [String],
    derived_grant_refs: &'a [String],
    decision: &'a str,
    diagnostic: &'a str,
}

fn harness_basalt_authority_receipt_value(input: HarnessBasaltAuthorityReceiptInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::record("basalt-ucan-authority-receipt-v1", vec![
        crate::preserves_rail::string("molten.runtime.basalt-ucan-authority-receipt.v1"),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("source", vec![crate::preserves_rail::string(input.source)]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(input.capability_ref)]),
        crate::preserves_rail::record("request-ref", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("ucan-proofset-ref", vec![crate::preserves_rail::string(input.proofset_ref)]),
        crate::preserves_rail::record("ucan-verification-receipt-refs", vec![crate::preserves_rail::sequence(
            input.verification_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("derived-grant-refs", vec![crate::preserves_rail::sequence(
            input.derived_grant_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::string(input.diagnostic),
        ])]),
        crate::preserves_rail::record("evidence-only", vec![crate::preserves_rail::string(
            "harness-fixture-does-not-grant-production-authority",
        )]),
    ])
}
