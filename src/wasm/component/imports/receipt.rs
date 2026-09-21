pub const ADMISSION_EVIDENCE_SCHEMA: &str = "molten.wasm-import-admission-receipt.v1";

pub const ADMISSION_NON_CLAIMS: &[&str] = &[
    "not-sandbox-completeness",
    "not-component-correctness",
    "not-semantic-equivalence",
    "not-guest-safety",
    "not-production-readiness",
    "not-release-eligibility",
];

pub const FORBIDDEN_ADMISSION_CLAIMS: &[&str] = &[
    "sandbox-containment",
    "sandbox-completeness",
    "component-correctness",
    "semantic-equivalence",
    "guest-safety",
    "production-readiness",
    "release-eligibility",
];

const MAX_RECEIPT_FIELD_BYTES: usize = 256;
const MAX_RECEIPT_BLOCKERS: usize = 16;

/// Bounded input for one declared-surface admission receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionInput {
    pub manifest_ref: String,
    pub wit_package: String,
    pub world: String,
    pub wit_ref: String,
    pub import_set_ref: String,
    pub extraction_tool: String,
    pub verifier: String,
    pub is_admitted: bool,
    pub blockers: Vec<String>,
    pub labels: Vec<String>,
}

/// Canonical declared-surface admission receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionEvidence {
    pub input: AdmissionInput,
    pub non_claims: Vec<String>,
    pub receipt_ref: String,
}

/// Build one bounded declared-surface admission receipt.
pub fn build_admission_evidence(input: AdmissionInput) -> super::super::model::ComponentResult<AdmissionEvidence> {
    validate_input_bounds(&input)?;
    let non_claims = ADMISSION_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect();
    let mut receipt = AdmissionEvidence {
        input: normalize_input(input),
        non_claims,
        receipt_ref: String::new(),
    };
    receipt.receipt_ref = evidence_identity(&receipt)?;
    Ok(receipt)
}

/// Validate one declared-surface admission receipt end to end.
pub fn validate_admission_evidence(receipt: &AdmissionEvidence) -> super::super::model::ComponentResult<()> {
    validate_input_bounds(&receipt.input)?;
    if receipt.non_claims != ADMISSION_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>() {
        return Err(super::super::model::ComponentDenial::new(
            "component import admission receipt non-claims were extended or weakened",
        ));
    }
    if receipt.receipt_ref != evidence_identity(receipt)? {
        return Err(super::super::model::ComponentDenial::new(
            "component import admission receipt identity is stale or tampered",
        ));
    }
    Ok(())
}

/// Reject receipts whose labels promote admission beyond its claims.
pub fn validate_non_claims(receipt: &AdmissionEvidence) -> super::super::model::ComponentResult<()> {
    validate_admission_evidence(receipt)?;
    for label in &receipt.input.labels {
        if FORBIDDEN_ADMISSION_CLAIMS.contains(&label.as_str()) {
            return Err(super::super::model::ComponentDenial::new(format!(
                "component import admission receipt overclaims {label}"
            )));
        }
    }
    Ok(())
}

/// Canonical Preserves value of one declared-surface admission receipt.
pub fn admission_evidence_value(receipt: &AdmissionEvidence) -> preserves::IOValue {
    crate::preserves_rail::record("wasm-import-admission-receipt-v1", vec![
        crate::preserves_rail::record("schema", vec![crate::preserves_rail::string(ADMISSION_EVIDENCE_SCHEMA)]),
        crate::preserves_rail::record("manifest-ref", vec![crate::preserves_rail::string(&receipt.input.manifest_ref)]),
        crate::preserves_rail::record("wit-package", vec![crate::preserves_rail::string(&receipt.input.wit_package)]),
        crate::preserves_rail::record("world", vec![crate::preserves_rail::string(&receipt.input.world)]),
        crate::preserves_rail::record("wit-ref", vec![crate::preserves_rail::string(&receipt.input.wit_ref)]),
        crate::preserves_rail::record("import-set-ref", vec![crate::preserves_rail::string(
            &receipt.input.import_set_ref,
        )]),
        crate::preserves_rail::record("extraction-tool", vec![crate::preserves_rail::string(
            &receipt.input.extraction_tool,
        )]),
        crate::preserves_rail::record("verifier", vec![crate::preserves_rail::string(&receipt.input.verifier)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(if receipt.input.is_admitted {
            "pass"
        } else {
            "deny"
        })]),
        crate::preserves_rail::record("blockers", vec![strings(&receipt.input.blockers)]),
        crate::preserves_rail::record("labels", vec![strings(&receipt.input.labels)]),
        crate::preserves_rail::record("non-claims", vec![strings(&receipt.non_claims)]),
    ])
}

fn validate_input_bounds(input: &AdmissionInput) -> super::super::model::ComponentResult<()> {
    let fields = [
        ("manifest-ref", input.manifest_ref.as_str()),
        ("wit-package", input.wit_package.as_str()),
        ("world", input.world.as_str()),
        ("wit-ref", input.wit_ref.as_str()),
        ("import-set-ref", input.import_set_ref.as_str()),
        ("extraction-tool", input.extraction_tool.as_str()),
        ("verifier", input.verifier.as_str()),
    ];
    for (label, field) in fields {
        if field.len() > MAX_RECEIPT_FIELD_BYTES {
            return Err(super::super::model::ComponentDenial::new(format!(
                "component import admission receipt {label} exceeds the bounded payload size"
            )));
        }
    }
    if input.blockers.len() > MAX_RECEIPT_BLOCKERS {
        return Err(super::super::model::ComponentDenial::new(
            "component import admission receipt blocker count exceeds the declared bound",
        ));
    }
    for blocker in &input.blockers {
        if blocker.len() > MAX_RECEIPT_FIELD_BYTES {
            return Err(super::super::model::ComponentDenial::new(
                "component import admission receipt packs raw bytes beyond the declared bound",
            ));
        }
    }
    Ok(())
}

fn normalize_input(mut input: AdmissionInput) -> AdmissionInput {
    input.blockers = super::super::model::sorted_unique(&input.blockers);
    input.labels = super::super::model::sorted_unique(&input.labels);
    input
}

fn evidence_identity(receipt: &AdmissionEvidence) -> super::super::model::ComponentResult<String> {
    crate::preserves_rail::canonical_hash(&admission_evidence_value(receipt)).map_err(|error| {
        super::super::model::ComponentDenial::new(format!("component import admission receipt hashing failed: {error}"))
    })
}

fn strings(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}
