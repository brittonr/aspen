
const LOCATOR_ANNOUNCEMENT_SCHEMA: &str = "molten.iroh-discovery.locator-announcement.v1";
const LOCATOR_ADMISSION_RECEIPT_SCHEMA: &str = "molten.iroh-discovery.locator-admission-receipt.v1";
const PKARR_LOCATOR_RECEIPT_SCHEMA: &str = "molten.iroh-discovery.pkarr-locator-receipt.v1";
const LOCATOR_NON_AUTHORITY_CAVEAT: &str = "locator evidence is hint-only and does not import, pin, install, expose, execute, or grant authority";
const LOCATOR_COMPLETE: &str = "complete";
const LOCATOR_PARTIAL: &str = "partial";
const LOCATOR_UNKNOWN: &str = "unknown";
const LOCATOR_FRESH: &str = "fresh";
const LOCATOR_STALE: &str = "stale";
const LOCATOR_DIAGNOSTIC_CAPACITY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatorAnnouncementInput<'a> {
    pub peer_ref: &'a str,
    pub signer: &'a str,
    pub subject_ref: &'a str,
    pub availability: &'a str,
    pub freshness: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatorAdmissionInput<'a> {
    pub locator_refs: &'a [String],
    pub fetched_ref: Option<&'a str>,
    pub verification_refs: &'a [String],
    pub admission_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkarrLocatorInput<'a> {
    pub key_ref: &'a str,
    pub signer: &'a str,
    pub resolved_subject_ref: &'a str,
    pub freshness: &'a str,
    pub signature_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatorEvidence {
    pub decision: String,
    pub evidence_ref: String,
    pub value: IoValue,
    pub diagnostics: Vec<String>,
    pub can_import: bool,
}

pub fn locator_announcement(input: &LocatorAnnouncementInput<'_>) -> Result<LocatorEvidence> {
    validate_locator_identity(input.peer_ref, input.subject_ref)?;
    validate_peer(input.signer)?;
    validate_availability(input.availability)?;
    validate_freshness(input.freshness)?;
    validate_refs(input.evidence_refs, "locator announcement evidence ref")?;
    let value = record("content-locator-announcement-v1", vec![
        string(LOCATOR_ANNOUNCEMENT_SCHEMA),
        record("peer", vec![string(input.peer_ref)]),
        record("signer", vec![string(input.signer)]),
        record("subject", vec![string(input.subject_ref)]),
        record("availability", vec![string(input.availability)]),
        record("freshness", vec![string(input.freshness)]),
        record("evidence", vec![string_sequence(input.evidence_refs)]),
        locator_caveats(),
    ]);
    locator_evidence("pass", value, Vec::new())
}

pub fn admit_locator_import(input: &LocatorAdmissionInput<'_>) -> Result<LocatorEvidence> {
    validate_refs(input.locator_refs, "locator admission locator ref")?;
    if let Some(reference) = input.fetched_ref {
        require_ref(reference, "locator admission fetched ref")?;
    }
    validate_refs(input.verification_refs, "locator admission verification ref")?;
    validate_refs(input.admission_refs, "locator admission local admission ref")?;
    validate_refs(input.authority_refs, "locator admission authority ref")?;
    validate_refs(input.policy_refs, "locator admission policy ref")?;
    validate_refs(input.resource_refs, "locator admission resource ref")?;
    let mut diagnostics = Vec::with_capacity(LOCATOR_DIAGNOSTIC_CAPACITY);
    if input.fetched_ref.is_none() {
        diagnostics.push("locator evidence is hint-only until receiver fetched bytes are present".to_string());
    }
    if input.verification_refs.is_empty() {
        diagnostics.push("locator evidence is hint-only until hash verification passes".to_string());
    }
    if input.admission_refs.is_empty() {
        diagnostics.push("locator evidence is hint-only until local admission passes".to_string());
    }
    if input.authority_refs.is_empty() || input.policy_refs.is_empty() || input.resource_refs.is_empty() {
        diagnostics.push("locator evidence is not authority, policy admission, or resource rights".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("content-locator-admission-receipt-v1", vec![
        string(LOCATOR_ADMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("locators", vec![string_sequence(input.locator_refs)]),
        record("fetched", vec![optional_str(input.fetched_ref)]),
        record("verification", vec![string_sequence(input.verification_refs)]),
        record("local-admission", vec![string_sequence(input.admission_refs)]),
        record("authority", vec![string_sequence(input.authority_refs)]),
        record("policy", vec![string_sequence(input.policy_refs)]),
        record("resource", vec![string_sequence(input.resource_refs)]),
        record("diagnostics", vec![string_sequence(&diagnostics)]),
        locator_caveats(),
    ]);
    locator_evidence(decision, value, diagnostics)
}

pub fn pkarr_locator_result(input: &PkarrLocatorInput<'_>) -> Result<LocatorEvidence> {
    require_ref(input.key_ref, "pkarr key ref")?;
    validate_peer(input.signer)?;
    require_ref(input.resolved_subject_ref, "pkarr resolved subject ref")?;
    validate_freshness(input.freshness)?;
    require_ref(input.signature_ref, "pkarr signature ref")?;
    let mut diagnostics = Vec::with_capacity(LOCATOR_DIAGNOSTIC_CAPACITY);
    if input.freshness == LOCATOR_STALE {
        diagnostics.push("pkarr locator pointer is stale and remains diagnostic only".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("pkarr-locator-result-v1", vec![
        string(PKARR_LOCATOR_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("key", vec![string(input.key_ref)]),
        record("signer", vec![string(input.signer)]),
        record("resolved-subject", vec![string(input.resolved_subject_ref)]),
        record("freshness", vec![string(input.freshness)]),
        record("signature", vec![string(input.signature_ref)]),
        record("diagnostics", vec![string_sequence(&diagnostics)]),
        locator_caveats(),
    ]);
    locator_evidence(decision, value, diagnostics)
}

fn locator_evidence(decision: &str, value: IoValue, diagnostics: Vec<String>) -> Result<LocatorEvidence> {
    Ok(LocatorEvidence {
        decision: decision.to_string(),
        evidence_ref: canonical_hash(&value)?,
        value,
        diagnostics,
        can_import: false,
    })
}

fn locator_caveats() -> IoValue {
    record("caveats", vec![sequence(vec![string(LOCATOR_NON_AUTHORITY_CAVEAT)])])
}

fn string_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_str(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), string)
}

fn validate_locator_identity(peer_ref: &str, subject_ref: &str) -> Result<()> {
    validate_peer(peer_ref)?;
    require_ref(subject_ref, "locator subject ref")
}

fn validate_availability(value: &str) -> Result<()> {
    match value {
        LOCATOR_COMPLETE | LOCATOR_PARTIAL | LOCATOR_UNKNOWN => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported locator availability {value}"))),
    }
}

fn validate_freshness(value: &str) -> Result<()> {
    match value {
        LOCATOR_FRESH | LOCATOR_STALE => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported locator freshness {value}"))),
    }
}

