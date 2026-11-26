use molten_core::world_snapshot::SnapshotClass;
use molten_core::world_snapshot::SnapshotRestoreStep;

use super::super::super::*;
use super::support::Admission;
use super::support::ChaosObserver;
use super::support::Handles;
use super::support::Materializer;
use super::support::OpaqueRuntime;
use super::support::Receipts;
use super::support::binding;
use super::support::fixture;

// r[verify molten.world_snapshot.opaque]
// r[verify molten.world_snapshot.restore]
#[test]
fn exact_chaoscontrol_descriptor_restores_before_activation_and_receipt() {
    let (_chaos, descriptor) = fixture();
    assert_eq!(descriptor.class, SnapshotClass::Opaque);
    let destination = descriptor.cohort.clone();
    let mut materialization = Materializer;
    let mut chaoscontrol = ChaosObserver;
    let mut admission = Admission::current();
    let mut handles = Handles;
    let mut runtime = OpaqueRuntime;
    let mut receipts = Receipts::default();

    let outcome = restore_opaque_snapshot(&descriptor, &destination, OpaqueSnapshotPorts {
        materialization: &mut materialization,
        chaoscontrol: &mut chaoscontrol,
        admission: &mut admission,
        handles: &mut handles,
        runtime: &mut runtime,
        receipts: &mut receipts,
    })
    .expect("exact opaque restore");

    assert_eq!(receipts.published, 1);
    assert_eq!(
        outcome.observations.last().map(|observation| observation.step),
        Some(SnapshotRestoreStep::ActivateRuntime)
    );
    assert!(outcome.plan.artifact_ref.starts_with("blake3:"));
    assert!(outcome.receipt.artifact_ref.starts_with("blake3:"));
}

// r[verify molten.world_snapshot.verification]
#[test]
fn descriptor_identity_drift_is_rejected_before_molten_mapping() {
    let (mut envelope, _descriptor) = fixture();
    envelope.descriptor.architecture = "aarch64".to_string();
    let result = map_chaoscontrol_snapshot(&envelope, binding());
    assert!(result.is_err());
}

// r[verify molten.world_snapshot.verification]
#[test]
fn opaque_restore_denies_a_crossed_chaoscontrol_observation() {
    let (_chaos, descriptor) = fixture();
    let destination = descriptor.cohort.clone();
    let mut materialization = Materializer;
    let mut chaoscontrol = CrossedChaosObserver;
    let mut admission = Admission::current();
    let mut handles = Handles;
    let mut runtime = OpaqueRuntime;
    let mut receipts = Receipts::default();

    let result = restore_opaque_snapshot(&descriptor, &destination, OpaqueSnapshotPorts {
        materialization: &mut materialization,
        chaoscontrol: &mut chaoscontrol,
        admission: &mut admission,
        handles: &mut handles,
        runtime: &mut runtime,
        receipts: &mut receipts,
    });
    assert!(result.is_err());
    assert_eq!(receipts.published, 0);
}

struct CrossedChaosObserver;

impl ChaosControlSnapshotDescriptorPort for CrossedChaosObserver {
    fn observe_descriptor(
        &mut self,
        descriptor: &molten_core::world_snapshot::SnapshotDescriptor,
    ) -> crate::error::Result<ChaosControlDescriptorObservation> {
        Ok(ChaosControlDescriptorObservation {
            descriptor_ref: super::support::digest('b'),
            cohort_ref: descriptor.cohort.cohort_ref.as_str().to_string(),
            available: true,
            identity_verified: true,
        })
    }
}

fn framed_identity(domain: &str, label: &str, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    for part in [label.as_bytes(), bytes] {
        hasher.update(&u64::try_from(part.len()).expect("bounded identity input").to_le_bytes());
        hasher.update(part);
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn chaoscontrol_fact_and_component_identities_hash_the_explicit_preserves_projection() {
    let (envelope, descriptor) = fixture();
    let fact = |kind| {
        descriptor
            .cohort
            .facts
            .iter()
            .find(|fact| fact.kind == kind)
            .map(|fact| fact.identity.clone())
            .expect("fact")
    };
    let canonical = |value: &preserves::IOValue| crate::preserves_rail::canonical_bytes(value).expect("canonical");
    let topology = &envelope.descriptor.topology;

    let architecture = canonical(&crate::preserves_rail::string(&envelope.descriptor.architecture));
    assert_eq!(
        fact(molten_core::world_snapshot::CohortFactKind::Architecture),
        framed_identity("onixresearch.molten.world-snapshot.chaoscontrol-fact.v2", "architecture", &architecture)
    );
    let vcpu_topology = canonical(
        &crate::preserves_rail::parse_text(&format!(
            "<vcpu-topology {} {}>",
            topology.vcpu_count, topology.memory_bytes
        ))
        .expect("vcpu topology projection"),
    );
    assert_eq!(
        fact(molten_core::world_snapshot::CohortFactKind::VcpuTopology),
        framed_identity("onixresearch.molten.world-snapshot.chaoscontrol-fact.v2", "vcpu-topology", &vcpu_topology)
    );
    let json = serde_json::to_vec(&envelope.descriptor.architecture).expect("json");
    assert_ne!(
        fact(molten_core::world_snapshot::CohortFactKind::Architecture),
        framed_identity("onixresearch.molten.world-snapshot.chaoscontrol-fact.v1", "architecture", &json),
        "JSON bytes are no longer identity material"
    );

    let devices = topology
        .devices
        .iter()
        .map(|device| {
            let backend = match device.backend {
                chaoscontrol_snapshot_descriptor::DeviceBackend::Network => "network",
                chaoscontrol_snapshot_descriptor::DeviceBackend::Block => "block",
                chaoscontrol_snapshot_descriptor::DeviceBackend::Entropy => "entropy",
            };
            format!(
                "<device-cohort {} {} {} {} \"{backend}\">",
                device.identity.base_address, device.identity.irq, device.identity.device_id, device.queue_count
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let devices = canonical(&crate::preserves_rail::parse_text(&format!("[{devices}]")).expect("device projection"));
    let device_state = descriptor
        .components
        .iter()
        .find(|component| component.kind == molten_core::world_snapshot::SnapshotComponentKind::DeviceState)
        .expect("device state component");
    assert!(!topology.devices.is_empty(), "fixture must exercise the device projection");
    assert_eq!(
        device_state.identity,
        framed_identity("onixresearch.molten.world-snapshot.chaoscontrol-component.v2", "device-state", &devices)
    );
}
