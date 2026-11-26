use super::*;

const LOWERCASE_REF: &str = "blake3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";
const UPPERCASE_REF: &str = "blake3:ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD";
const MIXED_CASE_REF: &str = "blake3:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcD";
const PREFIX_ONLY_REF: &str = "blake3:metadata";

fn local_publication(content_ref: &str) -> PublicationObservations {
    PublicationObservations {
        content_ref: content_ref.to_string(),
        operation_id: "operation-1".to_string(),
        receiver_set: vec!["peer-a".to_string()],
        retention_generation: 1,
        placement_epoch: 1,
        storage_guarantee: StorageGuaranteeKind::Local,
    }
}

fn local_policy() -> ReplicaPolicy {
    ReplicaPolicy {
        desired_replicas: 1,
        minimum_verified_replicas: 1,
        minimum_fault_domains: 1,
    }
}

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn publication_admits_only_lowercase_canonical_digests() {
    let receipt = check_publication(&local_policy(), &local_publication(LOWERCASE_REF)).expect("lowercase admitted");
    assert_eq!(receipt.content_ref, LOWERCASE_REF);
    for spelling in [UPPERCASE_REF, MIXED_CASE_REF, PREFIX_ONLY_REF] {
        assert_eq!(
            check_publication(&local_policy(), &local_publication(spelling)).expect_err("spelling denied"),
            PublicationIssue::InvalidContentRef,
            "{spelling}"
        );
    }
}
