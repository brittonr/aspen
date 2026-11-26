//! Producer-record and consumer-compatibility rejections for the executable-extent consumer.

use super::*;

#[test]
fn rejects_extent_identity_target_page_and_missing_member() {
    let mut drift = MemorySource::complete();
    drift.members.insert(EXTENT_LEAF.to_string(), vec![OTHER_SOURCE_BYTE; PAGE_BYTES]);
    assert!(matches!(consume_bundle(&drift, &admitted(), &request()), Err(ConsumeError::ExtentRemeasurement)));

    let mut target = request();
    target.consumer.architecture = "aarch64".to_string();
    assert!(matches!(
        consume_bundle(&MemorySource::complete(), &admitted(), &target),
        Err(ConsumeError::Admission(molten_core::executable_extent::AdmissionError::Compatibility(
            executable_extent_core::CompatibilityError::ArchitectureMismatch
        )))
    ));

    let mut abi = request();
    abi.consumer.abi = "linux-musl".to_string();
    assert!(matches!(
        consume_bundle(&MemorySource::complete(), &admitted(), &abi),
        Err(ConsumeError::Admission(molten_core::executable_extent::AdmissionError::Compatibility(
            executable_extent_core::CompatibilityError::AbiMismatch
        )))
    ));

    let mut page = request();
    page.consumer.page_size_bytes = PAGE_BYTES_U64.saturating_mul(2);
    assert!(matches!(
        consume_bundle(&MemorySource::complete(), &admitted(), &page),
        Err(ConsumeError::Admission(molten_core::executable_extent::AdmissionError::Compatibility(
            executable_extent_core::CompatibilityError::PageSizeMismatch
        )))
    ));

    let mut missing = MemorySource::complete();
    missing.members.remove(EXTENT_LEAF);
    assert!(matches!(
        consume_bundle(&missing, &admitted(), &request()),
        Err(ConsumeError::Source(SourceError::Io(_)))
    ));
}

#[test]
fn rejects_stale_tampered_and_extended_producer_records() {
    let mut stale = MemorySource::complete();
    let mut receipt: serde_json::Value = serde_json::from_str(PRODUCER_RECEIPT).expect("receipt fixture");
    receipt["manifest_published"] = serde_json::Value::Bool(false);
    stale
        .members
        .insert(PRODUCER_RECEIPT_LEAF.to_string(), serde_json::to_vec(&receipt).expect("stale fixture"));
    assert!(matches!(
        consume_bundle(&stale, &admitted(), &request()),
        Err(ConsumeError::Producer(molten::executable_extent::producer::Error::Publication))
    ));

    let mut tampered = MemorySource::complete();
    let mut bundle: serde_json::Value = serde_json::from_str(BUNDLE).expect("bundle fixture");
    bundle["bundle_identity_blake3"] = serde_json::Value::String("0".repeat(blake3::OUT_LEN * 2));
    tampered
        .members
        .insert(BUNDLE_LEAF.to_string(), serde_json::to_vec(&bundle).expect("tampered fixture"));
    assert!(matches!(
        consume_bundle(&tampered, &admitted(), &request()),
        Err(ConsumeError::Producer(molten::executable_extent::producer::Error::Identity))
    ));

    let mut unsupported = MemorySource::complete();
    let mut bundle: serde_json::Value = serde_json::from_str(BUNDLE).expect("bundle fixture");
    bundle["format"] = serde_json::Value::String("unknown-format".to_string());
    unsupported
        .members
        .insert(BUNDLE_LEAF.to_string(), serde_json::to_vec(&bundle).expect("unsupported fixture"));
    assert!(matches!(
        consume_bundle(&unsupported, &admitted(), &request()),
        Err(ConsumeError::Producer(molten::executable_extent::producer::Error::Profile))
    ));

    let mut extended = MemorySource::complete();
    let mut bundle: serde_json::Value = serde_json::from_str(BUNDLE).expect("bundle fixture");
    bundle["ambient_authority"] = serde_json::Value::Bool(true);
    extended
        .members
        .insert(BUNDLE_LEAF.to_string(), serde_json::to_vec(&bundle).expect("extended fixture"));
    assert!(matches!(
        consume_bundle(&extended, &admitted(), &request()),
        Err(ConsumeError::Producer(molten::executable_extent::producer::Error::Json))
    ));
}
