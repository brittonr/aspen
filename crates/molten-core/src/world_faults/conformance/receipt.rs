use std::collections::BTreeSet;

use super::super::*;
use super::PHYSICAL_FAILURE_PROFILE_CASE;

pub fn validate_world_fault_receipt(
    receipt: &WorldFaultConformanceReceipt,
    profile: &WorldFaultProfile,
) -> Vec<WorldFaultIssue> {
    let mut issues = Vec::with_capacity(profile.limits.max_unsupported_rows);
    if receipt.schema != WORLD_FAULT_RECEIPT_SCHEMA {
        issues.push(WorldFaultIssue::SchemaMismatch("world-fault-receipt"));
    }
    if receipt.source_revision != profile.source_revision || receipt.inventory_ref != profile.inventory_ref {
        issues.push(WorldFaultIssue::MalformedReference {
            field: "receipt-profile-binding",
            value: receipt.profile_ref.clone(),
        });
    }
    if identify_world_fault_profile(profile).ok().as_ref() != Some(&receipt.profile_ref) {
        issues.push(WorldFaultIssue::MalformedReference {
            field: "receipt-profile-ref",
            value: receipt.profile_ref.clone(),
        });
    }
    if receipt.mutation_authorized_by_evidence || receipt.cleanup_authorized_by_evidence {
        issues.push(WorldFaultIssue::EvidenceAuthorityOverclaim);
    }
    for non_claim in REQUIRED_WORLD_FAULT_NON_CLAIMS {
        if !receipt.non_claims.contains(&non_claim) {
            issues.push(WorldFaultIssue::ReceiptNonClaimMissing(non_claim));
        }
    }
    let unsupported_cases = receipt.unsupported_rows.iter().map(|row| row.case_id.as_str()).collect::<BTreeSet<_>>();
    for case in profile.cases.iter().filter(|case| case.mutation == WorldMutationKind::Witness) {
        if !unsupported_cases.contains(case.case_id.as_str()) {
            issues.push(WorldFaultIssue::UnsupportedRowDropped(case.case_id.clone()));
        }
    }
    if !unsupported_cases.contains(PHYSICAL_FAILURE_PROFILE_CASE) {
        issues.push(WorldFaultIssue::UnsupportedRowDropped(PHYSICAL_FAILURE_PROFILE_CASE.to_string()));
    }
    issues.sort();
    issues.dedup();
    issues
}
