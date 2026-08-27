#![cfg(feature = "executable-extents")]

// r[verify molten.world_extents.dependency]
// r[verify molten.world_extents.identity_domains]
// r[verify molten.world_extents.profile]
// r[verify molten.world_extents.admission]
// r[verify molten.world_extents.wx]
// r[verify molten.world_extents.materialization]
// r[verify molten.world_extents.activation]
// r[verify molten.world_extents.receipts]
// r[verify molten.world_extents.verification]

use std::collections::BTreeMap;

use molten::executable_extent::*;

const BUNDLE: &str = include_str!("fixtures/executable-extent/executable-extent-bundle.valid.json");
const PRODUCER_RECEIPT: &str = include_str!("fixtures/executable-extent/executable-extent-producer-receipt.valid.json");
const CONSUMER_RECEIPT: &str =
    include_str!("fixtures/executable-extent/molten-executable-extent-consumer-receipt.valid.json");
const BUNDLE_LEAF: &str = "bundle-2f44f5eeb1d93cafc65dfdac36fb0d2020fed4466d074d331602422a8d411d81.json";
const PRODUCER_RECEIPT_LEAF: &str =
    "producer-receipt-82b1ccefdbe9080649c154e02410f53006cdf7ce541a3b93e653c1846aea526a.json";
const EXTENT_LEAF: &str = "extent-4598e001cd6e4c4fe4aa57bb055c11f1cbe10b3e0def42de0da8ec4036500f6c.bin";
const PAGE_BYTES: usize = 4_096;
const PAGE_BYTES_U64: u64 = 4_096;
const SOURCE_BYTE: u8 = 0x41;
const OTHER_SOURCE_BYTE: u8 = 0x42;
const EXPECTED_LAYOUT_CORPUS_IDENTITY: [u8; blake3::OUT_LEN] = [
    118, 90, 123, 114, 76, 59, 161, 33, 95, 102, 174, 232, 101, 122, 114, 128, 56, 182, 96, 142, 247, 230, 6, 3, 102,
    224, 73, 10, 98, 138, 202, 174,
];
const EXPECTED_TRANSITION_CORPUS_IDENTITY: [u8; blake3::OUT_LEN] = [
    187, 246, 254, 165, 139, 67, 204, 182, 216, 228, 125, 76, 234, 17, 59, 28, 10, 148, 255, 128, 88, 206, 20, 239, 16,
    77, 143, 200, 87, 196, 131, 98,
];
const CONSUMER_IMPLEMENTATION_ID: &str = "molten-private-rad-consumer@v1";

struct MemorySource {
    members: BTreeMap<String, Vec<u8>>,
}

impl MemorySource {
    fn complete() -> Self {
        let members = BTreeMap::from([
            (BUNDLE_LEAF.to_string(), BUNDLE.as_bytes().to_vec()),
            (PRODUCER_RECEIPT_LEAF.to_string(), PRODUCER_RECEIPT.as_bytes().to_vec()),
            (EXTENT_LEAF.to_string(), vec![SOURCE_BYTE; PAGE_BYTES]),
        ]);
        Self { members }
    }
}

impl BundleSource for MemorySource {
    fn read_leaf(&self, leaf: &str, maximum_bytes: usize) -> Result<Vec<u8>, SourceError> {
        let bytes = self.members.get(leaf).ok_or_else(|| {
            SourceError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "fixture member missing"))
        })?;
        if bytes.len() > maximum_bytes {
            return Err(SourceError::BoundExceeded);
        }
        Ok(bytes.clone())
    }
}

#[derive(Clone, Copy)]
struct FixedAdmission {
    facts: molten_core::executable_extent::ActivationFacts,
    available: bool,
}

impl CurrentAdmissionPort for FixedAdmission {
    fn observe(
        &self,
        _profile: &molten_core::executable_extent::ExtentCodeRootProfile,
    ) -> Result<molten_core::executable_extent::ActivationFacts, AdmissionPortError> {
        if self.available {
            Ok(self.facts)
        } else {
            Err(AdmissionPortError::ObservationUnavailable)
        }
    }
}

fn admitted() -> FixedAdmission {
    FixedAdmission {
        facts: molten_core::executable_extent::ActivationFacts {
            artifact_current: true,
            runtime_current: true,
            resources_available: true,
            policy_current: true,
            execution_authorized: true,
        },
        available: true,
    }
}

fn request() -> ConsumerRequest {
    ConsumerRequest {
        bundle_manifest_leaf: BUNDLE_LEAF.to_string(),
        producer_receipt_leaf: PRODUCER_RECEIPT_LEAF.to_string(),
        semantic_code: molten_core::executable_extent::SemanticCodeIdentity::from_bytes(
            *blake3::hash(b"molten-test-semantic-code-v1").as_bytes(),
        ),
        consumer: molten_core::executable_extent::ConsumerProfile {
            architecture: "x86_64".to_string(),
            abi: "linux-gnu".to_string(),
            endianness: executable_extent_core::Endianness::Little,
            page_size_bytes: PAGE_BYTES_U64,
            runtime_cohort: molten_core::executable_extent::RuntimeCohortIdentity::from_bytes(
                *blake3::hash(b"molten-test-runtime-cohort-v1").as_bytes(),
            ),
            policy: molten_core::executable_extent::PolicyIdentity::from_bytes(
                *blake3::hash(b"molten-test-executable-extent-policy-v1").as_bytes(),
            ),
        },
        extent_profile_required: true,
    }
}

#[test]
fn shared_hostile_corpora_keep_frozen_identities_in_molten_cohort() {
    let layout = executable_extent_conformance::run(
        executable_extent_conformance::AdapterRole::Consumer,
        CONSUMER_IMPLEMENTATION_ID,
        executable_extent_conformance::standard_vectors(),
    )
    .expect("layout corpus");
    assert_eq!(layout.corpus_identity, EXPECTED_LAYOUT_CORPUS_IDENTITY);
    let transitions = executable_extent_conformance::run_transitions(
        executable_extent_conformance::AdapterRole::Consumer,
        CONSUMER_IMPLEMENTATION_ID,
        executable_extent_conformance::standard_transition_vectors(),
    )
    .expect("transition corpus");
    assert_eq!(transitions.corpus_identity, EXPECTED_TRANSITION_CORPUS_IDENTITY);
}

#[test]
fn capability_root_admits_maps_remeasures_and_unmaps_exact_mantle_fixture() {
    let root = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary capability root");
    root.write(BUNDLE_LEAF, BUNDLE.as_bytes()).expect("write bundle");
    root.write(PRODUCER_RECEIPT_LEAF, PRODUCER_RECEIPT.as_bytes()).expect("write producer receipt");
    root.write(EXTENT_LEAF, vec![SOURCE_BYTE; PAGE_BYTES]).expect("write extent");
    let source = CapabilityBundleSource::new(&root);
    let outcome = consume_bundle(&source, &admitted(), &request()).expect("admit exact bundle");
    let ConsumeOutcome::Mapped(mapped) = outcome else {
        panic!("current authority must admit a live mapping");
    };
    let receipt = mapped.complete().expect("explicit teardown");
    assert_eq!(receipt.disposition, "mapped-and-unmapped");
    assert_eq!(receipt.bundle_identity_blake3, "2f44f5eeb1d93cafc65dfdac36fb0d2020fed4466d074d331602422a8d411d81");
    assert_eq!(
        receipt.producer_receipt_identity_blake3,
        "82b1ccefdbe9080649c154e02410f53006cdf7ce541a3b93e653c1846aea526a"
    );
    assert_eq!(receipt.layout_identity_blake3, "b1f86e8102e359b22e9f0cb4f7efaa65ad84f6953fc68259b725bf1051400dd9");
    assert_eq!(receipt.mappings.len(), 1);
    assert_eq!(receipt.mappings[0].mapped_state, "executable-read-only");
    assert_eq!(receipt.mappings[0].final_state, "unmapped");
    assert_eq!(receipt.executable_extent_revision, EXECUTABLE_EXTENT_REVISION);
    assert_eq!(receipt.mantle_producer_revision, MANTLE_PRODUCER_REVISION);
    assert_eq!(receipt.receipt_identity_blake3, "e02c5e04505f3bd05b351593d471ef64a86596b4c304a71f97d49a770dd4482f");
    let encoded = serde_json::to_vec(&receipt).expect("encode consumer receipt");
    let decoded: ConsumerReceipt = serde_json::from_slice(&encoded).expect("decode consumer receipt");
    assert_eq!(decoded, receipt);
    let fixture: ConsumerReceipt = serde_json::from_str(CONSUMER_RECEIPT).expect("consumer fixture");
    assert_eq!(fixture, receipt);
}

#[test]
fn valid_extent_remains_inert_without_current_execution_authority() {
    let mut authority = admitted();
    authority.facts.execution_authorized = false;
    let outcome = consume_bundle(&MemorySource::complete(), &authority, &request())
        .expect("structurally valid extent remains a bounded outcome");
    let ConsumeOutcome::Inert(receipt) = outcome else {
        panic!("denied extent must not be mapped");
    };
    assert_eq!(receipt.disposition, "inert");
    assert_eq!(receipt.denial.as_deref(), Some("execution-unauthorized"));
    assert!(receipt.mappings.is_empty());
}

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

#[test]
fn fails_closed_for_unavailable_authority_and_path_traversal() {
    let unavailable = FixedAdmission {
        facts: admitted().facts,
        available: false,
    };
    assert!(matches!(
        consume_bundle(&MemorySource::complete(), &unavailable, &request()),
        Err(ConsumeError::AdmissionPort(AdmissionPortError::ObservationUnavailable))
    ));

    let root = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary capability root");
    let source = CapabilityBundleSource::new(&root);
    let mut traversal = request();
    traversal.bundle_manifest_leaf = "../bundle.json".to_string();
    assert!(matches!(
        consume_bundle(&source, &admitted(), &traversal),
        Err(ConsumeError::Source(SourceError::InvalidLeaf))
    ));
}
