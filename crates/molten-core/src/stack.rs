#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StackEvidenceRole {
    Basalt,
    Ucan,
    Trellis,
    Octet,
    Valence,
    Cairn,
    Mantle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackEvidenceMember<'a> {
    pub role: StackEvidenceRole,
    pub schema: &'a str,
    pub artifact_ref: &'a str,
    pub verification_role: &'a str,
    pub non_claims: &'a [&'a str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackEvidenceEnvelope<'a> {
    pub members: &'a [StackEvidenceMember<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackEvidenceSummary {
    pub member_count: usize,
    pub required_roles_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackEvidenceIssue {
    MissingRole(StackEvidenceRole),
    DuplicateRole(StackEvidenceRole),
    MalformedArtifactRef(StackEvidenceRole),
    UnsupportedSchema(StackEvidenceRole),
    MissingVerificationRole(StackEvidenceRole),
    MissingEvidenceOnlyNonClaim(StackEvidenceRole),
    OverbroadClaim(StackEvidenceRole),
}

// r[impl molten.evidence.valence_stack_adapter.contract]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValenceStackAdapterRow<'a> {
    pub molten_role: StackEvidenceRole,
    pub molten_schema: &'a str,
    pub valence_role: &'a str,
    pub valence_schema: &'a str,
    pub verification_role: &'a str,
    pub required_non_claim: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValenceStackAdapterReport {
    pub member_count: usize,
    pub mapped_rows: Vec<ValenceStackAdapterReportRow>,
    pub supported_claim: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValenceStackAdapterReportRow {
    pub molten_role: StackEvidenceRole,
    pub molten_schema: String,
    pub valence_role: String,
    pub valence_schema: String,
    pub artifact_ref: String,
    pub verification_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValenceStackAdapterIssue {
    StackEvidence(StackEvidenceIssue),
    MissingValenceRow(StackEvidenceRole),
    DuplicateValenceRow(StackEvidenceRole),
    MoltenSchemaMismatch(StackEvidenceRole),
    ValenceRoleMismatch(StackEvidenceRole),
    ValenceSchemaMismatch(StackEvidenceRole),
    VerificationRoleMismatch(StackEvidenceRole),
    MissingValenceNonClaim(StackEvidenceRole),
    ValenceAuthorityOverclaim(StackEvidenceRole),
}

const REQUIRED_ROLES: [StackEvidenceRole; REQUIRED_ROLE_COUNT] = [
    StackEvidenceRole::Basalt,
    StackEvidenceRole::Ucan,
    StackEvidenceRole::Trellis,
    StackEvidenceRole::Octet,
    StackEvidenceRole::Valence,
    StackEvidenceRole::Cairn,
    StackEvidenceRole::Mantle,
];
const REQUIRED_ROLE_COUNT: usize = 7;
const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_CHAR_COUNT: usize = 64;
const BLAKE3_REF_CHAR_COUNT: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_CHAR_COUNT;
const STACK_SCHEMA_PREFIX: &str = "molten.stack-evidence.";
const STACK_SCHEMA_SUFFIX: &str = ".v1";
const STACK_MEMBER_SCHEMA: &str = "molten.stack-evidence.member.v1";
const EVIDENCE_ONLY_NON_CLAIM: &str = "evidence-only";
const OVERBROAD_AUTHORITY_CLAIM: &str = "grants authority";
// r[impl molten.evidence.valence_stack_adapter.docs]
const VALENCE_ADAPTER_SUPPORTED_CLAIM: &str =
    "role/schema/ref compatibility and evidence-only non-claim conformance only";
const VALENCE_ADAPTER_NON_CLAIM: &str = "evidence-only: Molten/Valence stack adapter proves role/schema/ref compatibility only; does not grant runtime authority, release authority, transport trust, storage trust, UCAN authority, or permission to bypass subsystem gates";

const VALENCE_ROLE_BASALT: &str = "valence.stack.role.basalt-policy";
const VALENCE_ROLE_UCAN: &str = "valence.stack.role.ucan-capability";
const VALENCE_ROLE_TRELLIS: &str = "valence.stack.role.trellis-proof";
const VALENCE_ROLE_OCTET: &str = "valence.stack.role.octet-source-gate";
const VALENCE_ROLE_VALENCE: &str = "valence.stack.role.valence-identity";
const VALENCE_ROLE_CAIRN: &str = "valence.stack.role.cairn-lifecycle";
const VALENCE_ROLE_MANTLE: &str = "valence.stack.role.mantle-release";

const VALENCE_SCHEMA_BASALT: &str = "valence.stack-provenance.basalt-policy.v1";
const VALENCE_SCHEMA_UCAN: &str = "valence.stack-provenance.ucan-capability.v1";
const VALENCE_SCHEMA_TRELLIS: &str = "valence.stack-provenance.trellis-proof.v1";
const VALENCE_SCHEMA_OCTET: &str = "valence.stack-provenance.octet-source-gate.v1";
const VALENCE_SCHEMA_VALENCE: &str = "valence.stack-provenance.valence-identity.v1";
const VALENCE_SCHEMA_CAIRN: &str = "valence.stack-provenance.cairn-lifecycle.v1";
const VALENCE_SCHEMA_MANTLE: &str = "valence.stack-provenance.mantle-release.v1";

const VERIFICATION_ROLE_BASALT: &str = "policy-preflight-evidence";
const VERIFICATION_ROLE_UCAN: &str = "capability-admission-evidence";
const VERIFICATION_ROLE_TRELLIS: &str = "proof-reference-evidence";
const VERIFICATION_ROLE_OCTET: &str = "source-gate-evidence";
const VERIFICATION_ROLE_VALENCE: &str = "identity-linkage-evidence";
const VERIFICATION_ROLE_CAIRN: &str = "lifecycle-gate-evidence";
const VERIFICATION_ROLE_MANTLE: &str = "release-bundle-evidence";

const DEFAULT_VALENCE_STACK_ADAPTER_ROWS: [ValenceStackAdapterRow<'static>; REQUIRED_ROLE_COUNT] = [
    ValenceStackAdapterRow {
        molten_role: StackEvidenceRole::Basalt,
        molten_schema: STACK_MEMBER_SCHEMA,
        valence_role: VALENCE_ROLE_BASALT,
        valence_schema: VALENCE_SCHEMA_BASALT,
        verification_role: VERIFICATION_ROLE_BASALT,
        required_non_claim: VALENCE_ADAPTER_NON_CLAIM,
    },
    ValenceStackAdapterRow {
        molten_role: StackEvidenceRole::Ucan,
        molten_schema: STACK_MEMBER_SCHEMA,
        valence_role: VALENCE_ROLE_UCAN,
        valence_schema: VALENCE_SCHEMA_UCAN,
        verification_role: VERIFICATION_ROLE_UCAN,
        required_non_claim: VALENCE_ADAPTER_NON_CLAIM,
    },
    ValenceStackAdapterRow {
        molten_role: StackEvidenceRole::Trellis,
        molten_schema: STACK_MEMBER_SCHEMA,
        valence_role: VALENCE_ROLE_TRELLIS,
        valence_schema: VALENCE_SCHEMA_TRELLIS,
        verification_role: VERIFICATION_ROLE_TRELLIS,
        required_non_claim: VALENCE_ADAPTER_NON_CLAIM,
    },
    ValenceStackAdapterRow {
        molten_role: StackEvidenceRole::Octet,
        molten_schema: STACK_MEMBER_SCHEMA,
        valence_role: VALENCE_ROLE_OCTET,
        valence_schema: VALENCE_SCHEMA_OCTET,
        verification_role: VERIFICATION_ROLE_OCTET,
        required_non_claim: VALENCE_ADAPTER_NON_CLAIM,
    },
    ValenceStackAdapterRow {
        molten_role: StackEvidenceRole::Valence,
        molten_schema: STACK_MEMBER_SCHEMA,
        valence_role: VALENCE_ROLE_VALENCE,
        valence_schema: VALENCE_SCHEMA_VALENCE,
        verification_role: VERIFICATION_ROLE_VALENCE,
        required_non_claim: VALENCE_ADAPTER_NON_CLAIM,
    },
    ValenceStackAdapterRow {
        molten_role: StackEvidenceRole::Cairn,
        molten_schema: STACK_MEMBER_SCHEMA,
        valence_role: VALENCE_ROLE_CAIRN,
        valence_schema: VALENCE_SCHEMA_CAIRN,
        verification_role: VERIFICATION_ROLE_CAIRN,
        required_non_claim: VALENCE_ADAPTER_NON_CLAIM,
    },
    ValenceStackAdapterRow {
        molten_role: StackEvidenceRole::Mantle,
        molten_schema: STACK_MEMBER_SCHEMA,
        valence_role: VALENCE_ROLE_MANTLE,
        valence_schema: VALENCE_SCHEMA_MANTLE,
        verification_role: VERIFICATION_ROLE_MANTLE,
        required_non_claim: VALENCE_ADAPTER_NON_CLAIM,
    },
];

pub fn validate_stack_evidence_envelope(
    envelope: &StackEvidenceEnvelope<'_>,
) -> Result<StackEvidenceSummary, Vec<StackEvidenceIssue>> {
    let mut issues = Vec::new();
    for role in REQUIRED_ROLES {
        let matching_members = matching_role_members(envelope.members, role);
        if matching_members.is_empty() {
            issues.push(StackEvidenceIssue::MissingRole(role));
            continue;
        }
        if matching_members.len() > 1 {
            issues.push(StackEvidenceIssue::DuplicateRole(role));
        }
        for member in matching_members {
            validate_member(member, &mut issues);
        }
    }
    if issues.is_empty() {
        Ok(StackEvidenceSummary {
            member_count: envelope.members.len(),
            required_roles_present: true,
        })
    } else {
        Err(issues)
    }
}

pub fn default_valence_stack_adapter_rows() -> &'static [ValenceStackAdapterRow<'static>] {
    &DEFAULT_VALENCE_STACK_ADAPTER_ROWS
}

// r[impl molten.evidence.valence_stack_adapter.validation]
pub fn validate_valence_stack_adapter(
    envelope: &StackEvidenceEnvelope<'_>,
    rows: &[ValenceStackAdapterRow<'_>],
) -> Result<ValenceStackAdapterReport, Vec<ValenceStackAdapterIssue>> {
    let mut issues = Vec::new();
    if let Err(stack_issues) = validate_stack_evidence_envelope(envelope) {
        issues.extend(stack_issues.into_iter().map(ValenceStackAdapterIssue::StackEvidence));
    }

    for role in REQUIRED_ROLES {
        let matching_rows = matching_valence_rows(rows, role);
        if matching_rows.is_empty() {
            issues.push(ValenceStackAdapterIssue::MissingValenceRow(role));
            continue;
        }
        if matching_rows.len() > 1 {
            issues.push(ValenceStackAdapterIssue::DuplicateValenceRow(role));
        }
        for row in matching_rows {
            validate_valence_row(row, &mut issues);
            if let Some(member) = matching_role_members(envelope.members, role).first() {
                validate_member_against_valence_row(member, row, &mut issues);
            }
        }
    }

    if !issues.is_empty() {
        return Err(issues);
    }

    Ok(ValenceStackAdapterReport {
        member_count: envelope.members.len(),
        mapped_rows: mapped_valence_rows(envelope, rows),
        supported_claim: VALENCE_ADAPTER_SUPPORTED_CLAIM,
    })
}

fn matching_role_members<'a>(
    members: &'a [StackEvidenceMember<'a>],
    role: StackEvidenceRole,
) -> Vec<&'a StackEvidenceMember<'a>> {
    members.iter().filter(|member| member.role == role).collect()
}

fn matching_valence_rows<'a>(
    rows: &'a [ValenceStackAdapterRow<'a>],
    role: StackEvidenceRole,
) -> Vec<&'a ValenceStackAdapterRow<'a>> {
    rows.iter().filter(|row| row.molten_role == role).collect()
}

fn validate_member(member: &StackEvidenceMember<'_>, issues: &mut Vec<StackEvidenceIssue>) {
    if !valid_blake3_ref(member.artifact_ref) {
        issues.push(StackEvidenceIssue::MalformedArtifactRef(member.role));
    }
    if !supported_stack_schema(member.schema) {
        issues.push(StackEvidenceIssue::UnsupportedSchema(member.role));
    }
    if member.verification_role.is_empty() {
        issues.push(StackEvidenceIssue::MissingVerificationRole(member.role));
    }
    if !member.non_claims.iter().any(|claim| claim.contains(EVIDENCE_ONLY_NON_CLAIM)) {
        issues.push(StackEvidenceIssue::MissingEvidenceOnlyNonClaim(member.role));
    }
    if member.non_claims.iter().any(|claim| claim.contains(OVERBROAD_AUTHORITY_CLAIM)) {
        issues.push(StackEvidenceIssue::OverbroadClaim(member.role));
    }
}

fn validate_valence_row(row: &ValenceStackAdapterRow<'_>, issues: &mut Vec<ValenceStackAdapterIssue>) {
    if row.molten_schema != STACK_MEMBER_SCHEMA {
        issues.push(ValenceStackAdapterIssue::MoltenSchemaMismatch(row.molten_role));
    }
    if row.valence_role != expected_valence_role(row.molten_role) {
        issues.push(ValenceStackAdapterIssue::ValenceRoleMismatch(row.molten_role));
    }
    if row.valence_schema != expected_valence_schema(row.molten_role) {
        issues.push(ValenceStackAdapterIssue::ValenceSchemaMismatch(row.molten_role));
    }
    if row.verification_role != expected_verification_role(row.molten_role) {
        issues.push(ValenceStackAdapterIssue::VerificationRoleMismatch(row.molten_role));
    }
    if !row.required_non_claim.contains(EVIDENCE_ONLY_NON_CLAIM) {
        issues.push(ValenceStackAdapterIssue::MissingValenceNonClaim(row.molten_role));
    }
    if row.required_non_claim.contains(OVERBROAD_AUTHORITY_CLAIM) {
        issues.push(ValenceStackAdapterIssue::ValenceAuthorityOverclaim(row.molten_role));
    }
}

fn validate_member_against_valence_row(
    member: &StackEvidenceMember<'_>,
    row: &ValenceStackAdapterRow<'_>,
    issues: &mut Vec<ValenceStackAdapterIssue>,
) {
    if member.schema != row.molten_schema {
        issues.push(ValenceStackAdapterIssue::MoltenSchemaMismatch(member.role));
    }
    if member.verification_role != row.verification_role {
        issues.push(ValenceStackAdapterIssue::VerificationRoleMismatch(member.role));
    }
    if !member.non_claims.contains(&row.required_non_claim) {
        issues.push(ValenceStackAdapterIssue::MissingValenceNonClaim(member.role));
    }
}

fn mapped_valence_rows(
    envelope: &StackEvidenceEnvelope<'_>,
    rows: &[ValenceStackAdapterRow<'_>],
) -> Vec<ValenceStackAdapterReportRow> {
    let mut mapped_rows = Vec::with_capacity(REQUIRED_ROLE_COUNT);
    for role in REQUIRED_ROLES {
        let member = matching_role_members(envelope.members, role)
            .into_iter()
            .next()
            .expect("validated envelope has one member for every required role");
        let row = matching_valence_rows(rows, role)
            .into_iter()
            .next()
            .expect("validated adapter has one row for every required role");
        mapped_rows.push(ValenceStackAdapterReportRow {
            molten_role: role,
            molten_schema: row.molten_schema.to_string(),
            valence_role: row.valence_role.to_string(),
            valence_schema: row.valence_schema.to_string(),
            artifact_ref: member.artifact_ref.to_string(),
            verification_role: member.verification_role.to_string(),
        });
    }
    mapped_rows
}

fn expected_valence_role(role: StackEvidenceRole) -> &'static str {
    match role {
        StackEvidenceRole::Basalt => VALENCE_ROLE_BASALT,
        StackEvidenceRole::Ucan => VALENCE_ROLE_UCAN,
        StackEvidenceRole::Trellis => VALENCE_ROLE_TRELLIS,
        StackEvidenceRole::Octet => VALENCE_ROLE_OCTET,
        StackEvidenceRole::Valence => VALENCE_ROLE_VALENCE,
        StackEvidenceRole::Cairn => VALENCE_ROLE_CAIRN,
        StackEvidenceRole::Mantle => VALENCE_ROLE_MANTLE,
    }
}

fn expected_valence_schema(role: StackEvidenceRole) -> &'static str {
    match role {
        StackEvidenceRole::Basalt => VALENCE_SCHEMA_BASALT,
        StackEvidenceRole::Ucan => VALENCE_SCHEMA_UCAN,
        StackEvidenceRole::Trellis => VALENCE_SCHEMA_TRELLIS,
        StackEvidenceRole::Octet => VALENCE_SCHEMA_OCTET,
        StackEvidenceRole::Valence => VALENCE_SCHEMA_VALENCE,
        StackEvidenceRole::Cairn => VALENCE_SCHEMA_CAIRN,
        StackEvidenceRole::Mantle => VALENCE_SCHEMA_MANTLE,
    }
}

fn expected_verification_role(role: StackEvidenceRole) -> &'static str {
    match role {
        StackEvidenceRole::Basalt => VERIFICATION_ROLE_BASALT,
        StackEvidenceRole::Ucan => VERIFICATION_ROLE_UCAN,
        StackEvidenceRole::Trellis => VERIFICATION_ROLE_TRELLIS,
        StackEvidenceRole::Octet => VERIFICATION_ROLE_OCTET,
        StackEvidenceRole::Valence => VERIFICATION_ROLE_VALENCE,
        StackEvidenceRole::Cairn => VERIFICATION_ROLE_CAIRN,
        StackEvidenceRole::Mantle => VERIFICATION_ROLE_MANTLE,
    }
}

fn valid_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return false;
    };
    value.len() == BLAKE3_REF_CHAR_COUNT && hex.chars().all(|character| character.is_ascii_hexdigit())
}

fn supported_stack_schema(schema: &str) -> bool {
    schema.starts_with(STACK_SCHEMA_PREFIX) && schema.ends_with(STACK_SCHEMA_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const STALE_REF: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const EVIDENCE_ONLY: &str =
        "evidence-only: no runtime authority, provenance trust, verifier soundness, or deployment trust";
    const OVERBROAD: &str = "grants authority to execute runtime effects";
    const UNSUPPORTED_SCHEMA: &str = "molten.stack-evidence.member.v2";
    const UNKNOWN_VALENCE_ROLE: &str = "valence.stack.role.unknown";
    const UNKNOWN_VALENCE_SCHEMA: &str = "valence.stack-provenance.unknown.v1";
    const UNKNOWN_VERIFICATION_ROLE: &str = "unknown-verification-role";
    const NON_EVIDENCE_ONLY_CLAIM: &str = "compatibility without explicit evidence boundary";
    const BASALT_MEMBER_INDEX: usize = 0;
    const UCAN_MEMBER_INDEX: usize = 1;
    const LAST_REQUIRED_ROLE_INDEX: usize = REQUIRED_ROLE_COUNT - 1;

    fn member(role: StackEvidenceRole) -> StackEvidenceMember<'static> {
        StackEvidenceMember {
            role,
            schema: STACK_MEMBER_SCHEMA,
            artifact_ref: VALID_REF,
            verification_role: "manual-review-input",
            non_claims: &[EVIDENCE_ONLY],
        }
    }

    fn adapter_member(role: StackEvidenceRole) -> StackEvidenceMember<'static> {
        StackEvidenceMember {
            verification_role: expected_verification_role(role),
            non_claims: &[VALENCE_ADAPTER_NON_CLAIM],
            ..member(role)
        }
    }

    fn complete_members() -> [StackEvidenceMember<'static>; REQUIRED_ROLE_COUNT] {
        [
            member(StackEvidenceRole::Basalt),
            member(StackEvidenceRole::Ucan),
            member(StackEvidenceRole::Trellis),
            member(StackEvidenceRole::Octet),
            member(StackEvidenceRole::Valence),
            member(StackEvidenceRole::Cairn),
            member(StackEvidenceRole::Mantle),
        ]
    }

    fn complete_adapter_members() -> [StackEvidenceMember<'static>; REQUIRED_ROLE_COUNT] {
        [
            adapter_member(StackEvidenceRole::Basalt),
            adapter_member(StackEvidenceRole::Ucan),
            adapter_member(StackEvidenceRole::Trellis),
            adapter_member(StackEvidenceRole::Octet),
            adapter_member(StackEvidenceRole::Valence),
            adapter_member(StackEvidenceRole::Cairn),
            adapter_member(StackEvidenceRole::Mantle),
        ]
    }

    #[test]
    fn stack_evidence_envelope_accepts_complete_evidence_only_roles() {
        let members = complete_members();
        let summary = validate_stack_evidence_envelope(&StackEvidenceEnvelope { members: &members })
            .expect("complete envelope validates");

        assert_eq!(summary.member_count, REQUIRED_ROLE_COUNT);
        assert!(summary.required_roles_present);
    }

    #[test]
    fn stack_evidence_envelope_rejects_missing_role_stale_ref_and_unsupported_schema() {
        let mut members = complete_members();
        members[BASALT_MEMBER_INDEX].artifact_ref = STALE_REF;
        members[UCAN_MEMBER_INDEX].schema = UNSUPPORTED_SCHEMA;
        let shortened = &members[..LAST_REQUIRED_ROLE_INDEX];

        let issues = validate_stack_evidence_envelope(&StackEvidenceEnvelope { members: shortened })
            .expect_err("invalid envelope fails closed");

        assert!(issues.contains(&StackEvidenceIssue::MalformedArtifactRef(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&StackEvidenceIssue::UnsupportedSchema(StackEvidenceRole::Ucan)));
        assert!(issues.contains(&StackEvidenceIssue::MissingRole(StackEvidenceRole::Mantle)));
    }

    #[test]
    fn stack_evidence_envelope_rejects_overbroad_authority_claims() {
        let mut members = complete_members();
        members[BASALT_MEMBER_INDEX].non_claims = &[OVERBROAD];

        let issues = validate_stack_evidence_envelope(&StackEvidenceEnvelope { members: &members })
            .expect_err("overbroad claim fails closed");

        assert!(issues.contains(&StackEvidenceIssue::MissingEvidenceOnlyNonClaim(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&StackEvidenceIssue::OverbroadClaim(StackEvidenceRole::Basalt)));
    }

    #[test]
    fn valence_stack_adapter_maps_complete_evidence_only_envelope() {
        // r[verify molten.evidence.valence_stack_adapter.fixtures.positive]
        let members = complete_adapter_members();
        let report = validate_valence_stack_adapter(
            &StackEvidenceEnvelope { members: &members },
            default_valence_stack_adapter_rows(),
        )
        .expect("complete adapter fixture validates");

        assert_eq!(report.member_count, REQUIRED_ROLE_COUNT);
        assert_eq!(report.mapped_rows.len(), REQUIRED_ROLE_COUNT);
        assert_eq!(report.supported_claim, VALENCE_ADAPTER_SUPPORTED_CLAIM);
        assert!(report.mapped_rows.iter().any(|row| row.molten_role == StackEvidenceRole::Valence
            && row.valence_role == VALENCE_ROLE_VALENCE
            && row.valence_schema == VALENCE_SCHEMA_VALENCE));
    }

    #[test]
    fn valence_stack_adapter_rejects_member_and_mapping_failures() {
        // r[verify molten.evidence.valence_stack_adapter.fixtures.negative]
        let mut members = complete_adapter_members();
        members[BASALT_MEMBER_INDEX].artifact_ref = STALE_REF;
        members[UCAN_MEMBER_INDEX].verification_role = UNKNOWN_VERIFICATION_ROLE;
        let shortened = &members[..LAST_REQUIRED_ROLE_INDEX];
        let mut rows = default_valence_stack_adapter_rows().to_vec();
        rows[BASALT_MEMBER_INDEX].valence_role = UNKNOWN_VALENCE_ROLE;
        rows[UCAN_MEMBER_INDEX].valence_schema = UNKNOWN_VALENCE_SCHEMA;

        let issues = validate_valence_stack_adapter(&StackEvidenceEnvelope { members: shortened }, &rows)
            .expect_err("invalid adapter fixture fails closed");

        assert!(issues.contains(&ValenceStackAdapterIssue::StackEvidence(StackEvidenceIssue::MalformedArtifactRef(
            StackEvidenceRole::Basalt
        ))));
        assert!(issues.contains(&ValenceStackAdapterIssue::StackEvidence(StackEvidenceIssue::MissingRole(
            StackEvidenceRole::Mantle
        ))));
        assert!(issues.contains(&ValenceStackAdapterIssue::ValenceRoleMismatch(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&ValenceStackAdapterIssue::ValenceSchemaMismatch(StackEvidenceRole::Ucan)));
        assert!(issues.contains(&ValenceStackAdapterIssue::VerificationRoleMismatch(StackEvidenceRole::Ucan)));
    }

    #[test]
    fn valence_stack_adapter_rejects_duplicate_rows_missing_non_claims_and_overclaims() {
        // r[verify molten.evidence.valence_stack_adapter.docs]
        // r[verify molten.evidence.valence_stack_adapter.fixtures.negative]
        let mut members = complete_adapter_members();
        members[BASALT_MEMBER_INDEX].non_claims = &[EVIDENCE_ONLY];
        let mut rows = default_valence_stack_adapter_rows().to_vec();
        rows.push(rows[BASALT_MEMBER_INDEX]);
        rows[UCAN_MEMBER_INDEX].required_non_claim = NON_EVIDENCE_ONLY_CLAIM;
        rows[BASALT_MEMBER_INDEX].required_non_claim = OVERBROAD;

        let issues = validate_valence_stack_adapter(&StackEvidenceEnvelope { members: &members }, &rows)
            .expect_err("non-claims and duplicate rows fail closed");

        assert!(issues.contains(&ValenceStackAdapterIssue::DuplicateValenceRow(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&ValenceStackAdapterIssue::MissingValenceNonClaim(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&ValenceStackAdapterIssue::ValenceAuthorityOverclaim(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&ValenceStackAdapterIssue::MissingValenceNonClaim(StackEvidenceRole::Ucan)));
    }
}
