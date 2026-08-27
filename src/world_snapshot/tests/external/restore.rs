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
