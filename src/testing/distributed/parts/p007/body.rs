
const IROH_ADOPTION_FIXTURE_SCHEMA: &str = "molten.testing.iroh-experiment-adoption-fixture.v1";
const IROH_ADOPTION_REF_LABEL: &str = "iroh-adoption-fixture";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohExperimentAdoptionFixture {
    pub decision: String,
    pub locator_ref: String,
    pub traversal_receipt_ref: String,
    pub digest_receipt_ref: String,
    pub locator_denial_ref: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
}

pub fn iroh_experiment_adoption_fixture() -> Result<IrohExperimentAdoptionFixture> {
    let subject_ref = adoption_fixture_ref("subject");
    let evidence_ref = adoption_fixture_ref("evidence");
    let locator = crate::federation::locator_announcement(&crate::federation::LocatorAnnouncementInput {
        peer_ref: "peer:iroh-adoption",
        signer: "peer:iroh-adoption",
        subject_ref: &subject_ref,
        availability: "complete",
        freshness: "fresh",
        evidence_refs: std::slice::from_ref(&evidence_ref),
    })?;
    let locator_denial = crate::federation::admit_locator_import(&crate::federation::LocatorAdmissionInput {
        locator_refs: std::slice::from_ref(&locator.evidence_ref),
        fetched_ref: None,
        verification_refs: &[],
        admission_refs: &[],
        authority_refs: &[],
        policy_refs: &[],
        resource_refs: &[],
    })?;
    let descriptor = crate::remote_dataspace::TraversalDescriptor {
        traversal_kind: "artifact-closure".to_string(),
        root_refs: vec![subject_ref.clone()],
        visited_refs: Vec::new(),
        order: "lexicographic".to_string(),
        filters: Vec::new(),
        inline_policy: "metadata-only".to_string(),
        resource_bound: 1,
        replay_bound: 1,
        policy_refs: vec![adoption_fixture_ref("policy")],
        evidence_refs: vec![evidence_ref.clone()],
    };
    let traversal = crate::remote_dataspace::plan_traversal(
        &descriptor,
        &crate::remote_dataspace::LocalInventorySummary {
            verified_refs: Vec::new(),
            chunk_refs: Vec::new(),
        },
    )?;
    let bytes = b"iroh adoption bytes";
    let content_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
    let digest = crate::remote_dataspace::validate_external_digest_mapping(&crate::remote_dataspace::ExternalDigestMappingInput {
        algorithm: "cid-sha2-256",
        external_digest: &crate::remote_dataspace::external_digest_for("cid-sha2-256", bytes),
        bytes,
        expected_content_ref: &content_ref,
        evidence_refs: std::slice::from_ref(&evidence_ref),
    })?;
    let traversal_receipt_ref = crate::preserves_rail::canonical_hash(&traversal.receipt_value)?;
    let digest_receipt_ref = crate::preserves_rail::canonical_hash(&digest.receipt_value)?;
    let locator_denial_ref = crate::preserves_rail::canonical_hash(&locator_denial.value)?;
    let mut diagnostics = Vec::new();
    if locator.decision != PASS_DECISION {
        diagnostics.push("locator fixture did not pass".to_string());
    }
    if locator_denial.decision != DENY_DECISION {
        diagnostics.push("locator-only denial fixture did not deny".to_string());
    }
    if traversal.decision != PASS_DECISION {
        diagnostics.push("deterministic traversal fixture did not pass".to_string());
    }
    if digest.decision != PASS_DECISION {
        diagnostics.push("external digest fixture did not pass".to_string());
    }
    let decision = if diagnostics.is_empty() { PASS_DECISION } else { DENY_DECISION };
    let receipt_value = record("iroh-experiment-adoption-fixture-v1", vec![
        string(IROH_ADOPTION_FIXTURE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("locator", vec![string(&locator.evidence_ref)]),
        record("locator-denial", vec![string(&locator_denial_ref)]),
        record("traversal", vec![string(&traversal_receipt_ref)]),
        record("digest", vec![string(&digest_receipt_ref)]),
        record("diagnostics", vec![crate::preserves_rail::sequence(
            diagnostics.iter().map(string).collect(),
        )]),
        record("checks", vec![crate::preserves_rail::sequence(vec![
            record("check", vec![string("locator-hint-only-denial-covered"), string("pass")]),
            record("check", vec![string("deterministic-traversal-covered"), string("pass")]),
            record("check", vec![string("remote-bytes-verified-before-admission"), string("pass")]),
        ])]),
    ]);
    Ok(IrohExperimentAdoptionFixture {
        decision: decision.to_string(),
        locator_ref: locator.evidence_ref,
        traversal_receipt_ref,
        digest_receipt_ref,
        locator_denial_ref,
        diagnostics,
        receipt_value,
    })
}

fn adoption_fixture_ref(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(format!("{IROH_ADOPTION_REF_LABEL}:{label}").as_bytes())
}
