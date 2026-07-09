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
const EVIDENCE_ONLY_NON_CLAIM: &str = "evidence-only";
const OVERBROAD_AUTHORITY_CLAIM: &str = "grants authority";

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

fn matching_role_members<'a>(
    members: &'a [StackEvidenceMember<'a>],
    role: StackEvidenceRole,
) -> Vec<&'a StackEvidenceMember<'a>> {
    members.iter().filter(|member| member.role == role).collect()
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
    const REQUIRED_SCHEMA_PREFIX: &str = "molten.stack-evidence.member.v1";
    const UNSUPPORTED_SCHEMA: &str = "molten.stack-evidence.member.v2";

    fn member(role: StackEvidenceRole) -> StackEvidenceMember<'static> {
        StackEvidenceMember {
            role,
            schema: REQUIRED_SCHEMA_PREFIX,
            artifact_ref: VALID_REF,
            verification_role: "manual-review-input",
            non_claims: &[EVIDENCE_ONLY],
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
        members[0].artifact_ref = STALE_REF;
        members[1].schema = UNSUPPORTED_SCHEMA;
        let shortened = &members[..REQUIRED_ROLE_COUNT - 1];

        let issues = validate_stack_evidence_envelope(&StackEvidenceEnvelope { members: shortened })
            .expect_err("invalid envelope fails closed");

        assert!(issues.contains(&StackEvidenceIssue::MalformedArtifactRef(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&StackEvidenceIssue::UnsupportedSchema(StackEvidenceRole::Ucan)));
        assert!(issues.contains(&StackEvidenceIssue::MissingRole(StackEvidenceRole::Mantle)));
    }

    #[test]
    fn stack_evidence_envelope_rejects_overbroad_authority_claims() {
        let mut members = complete_members();
        members[0].non_claims = &[OVERBROAD];

        let issues = validate_stack_evidence_envelope(&StackEvidenceEnvelope { members: &members })
            .expect_err("overbroad claim fails closed");

        assert!(issues.contains(&StackEvidenceIssue::MissingEvidenceOnlyNonClaim(StackEvidenceRole::Basalt)));
        assert!(issues.contains(&StackEvidenceIssue::OverbroadClaim(StackEvidenceRole::Basalt)));
    }
}
