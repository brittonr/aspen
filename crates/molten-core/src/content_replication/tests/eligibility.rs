use super::*;

const CONTENT_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn replicated_policy() -> ReplicaPolicy {
    ReplicaPolicy {
        desired_replicas: 2,
        minimum_verified_replicas: 2,
        minimum_fault_domains: 2,
    }
}

fn observations(guarantee: StorageGuaranteeKind) -> EligibilityObservations {
    EligibilityObservations {
        content_ref: CONTENT_REF.to_string(),
        operation_id: "operation-1".to_string(),
        receiver_set: vec!["peer-b".to_string(), "peer-a".to_string()],
        retention_generation: 3,
        placement_epoch: 4,
        storage_guarantee: guarantee,
    }
}

// r[verify molten.observations.eligibility]
// r[verify molten.observations.tests]
#[test]
fn a_replicated_guarantee_satisfies_a_replicated_requirement() {
    let receipt = check_publication_eligibility(
        &replicated_policy(),
        &observations(StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
            replicas: 2,
            fault_domains: 2,
        }),
    )
    .expect("eligible publication");
    assert_eq!(receipt.content_ref, CONTENT_REF);
    assert_eq!(receipt.operation_id, "operation-1");
    assert_eq!(receipt.receiver_set, vec!["peer-a".to_string(), "peer-b".to_string()]);
    assert_eq!(receipt.retention_generation, 3);
    assert_eq!(receipt.placement_epoch, 4);
    assert_eq!(receipt.storage_guarantee, StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
        replicas: 2,
        fault_domains: 2,
    });
    assert!(receipt.receipt_ref.starts_with("blake3:"));
}

// r[verify molten.observations.eligibility]
#[test]
fn a_replica_count_alone_never_satisfies_the_gate() {
    let error = check_publication_eligibility(&replicated_policy(), &observations(StorageGuaranteeKind::Local))
        .expect_err("a local guarantee does not satisfy a replicated requirement");
    assert_eq!(error, EligibilityIssue::StorageGuaranteeInsufficient);

    let error = check_publication_eligibility(
        &replicated_policy(),
        &observations(StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
            replicas: 2,
            fault_domains: 1,
        }),
    )
    .expect_err("one fault domain does not satisfy a two-domain requirement");
    assert_eq!(error, EligibilityIssue::FaultDomainSpreadInsufficient);

    let error = check_publication_eligibility(
        &replicated_policy(),
        &observations(StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
            replicas: 1,
            fault_domains: 2,
        }),
    )
    .expect_err("one replica does not satisfy a two-replica requirement");
    assert_eq!(error, EligibilityIssue::ReplicaAvailabilityInsufficient);
}

// r[verify molten.observations.eligibility]
#[test]
fn a_local_profile_accepts_a_local_guarantee() {
    let policy = ReplicaPolicy {
        desired_replicas: 1,
        minimum_verified_replicas: 1,
        minimum_fault_domains: 1,
    };
    let receipt = check_publication_eligibility(&policy, &observations(StorageGuaranteeKind::Local))
        .expect("a local profile accepts a local guarantee");
    assert_eq!(receipt.storage_guarantee, StorageGuaranteeKind::Local);
}

// r[verify molten.observations.eligibility]
#[test]
fn an_incomplete_observation_is_refused() {
    let policy = replicated_policy();
    let mut invalid = observations(StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
        replicas: 2,
        fault_domains: 2,
    });
    invalid.content_ref = "not-a-content-ref".to_string();
    assert_eq!(
        check_publication_eligibility(&policy, &invalid).expect_err("an invalid content ref is refused"),
        EligibilityIssue::InvalidContentRef
    );

    let mut missing_operation = observations(StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
        replicas: 2,
        fault_domains: 2,
    });
    missing_operation.operation_id = String::new();
    assert_eq!(
        check_publication_eligibility(&policy, &missing_operation)
            .expect_err("a missing operation identity is refused"),
        EligibilityIssue::MissingOperationId
    );

    let mut empty_receivers = observations(StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
        replicas: 2,
        fault_domains: 2,
    });
    empty_receivers.receiver_set.clear();
    assert_eq!(
        check_publication_eligibility(&policy, &empty_receivers).expect_err("an empty receiver set is refused"),
        EligibilityIssue::EmptyReceiverSet
    );
}

// r[verify molten.observations.eligibility]
#[test]
fn the_receipt_identity_binds_its_fields() {
    let policy = replicated_policy();
    let receipt = check_publication_eligibility(
        &policy,
        &observations(StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
            replicas: 2,
            fault_domains: 2,
        }),
    )
    .expect("eligible publication");
    let mut changed = receipt.clone();
    changed.placement_epoch += 1;
    assert_ne!(
        eligibility_receipt_identity(&changed),
        receipt.receipt_ref,
        "the receipt identity binds the placement epoch"
    );
    let mut changed = receipt.clone();
    changed.receiver_set = vec!["peer-c".to_string()];
    assert_ne!(
        eligibility_receipt_identity(&changed),
        receipt.receipt_ref,
        "the receipt identity binds the receiver set"
    );
}
