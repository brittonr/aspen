use molten_core::content_replication::*;

use super::*;
use crate::error::Result;

pub trait AuthorityPort {
    fn observe(&mut self, manifest: &Manifest) -> Result<AuthorityObservation>;
}

pub trait IdentityPort {
    fn observe(&mut self, manifest: &Manifest) -> Result<IdentityObservation>;
}

pub trait MembershipPort {
    fn observe(&mut self, manifest: &Manifest) -> Result<MembershipObservation>;
}

pub trait PlacementPort {
    fn observe(&mut self, manifest: &Manifest) -> Result<PlacementObservation>;
}

pub trait TimePort {
    fn observe(&mut self, manifest: &Manifest) -> Result<TimeObservation>;
}

pub trait ResourcePort {
    fn reserve(&mut self, plan: &Plan) -> Result<ResourceObservation>;
}

pub trait ContentPort {
    fn inventory(&mut self, manifest: &Manifest) -> Result<Inventory>;

    fn verify(&mut self, action: &Action, envelope: &TransferEnvelope) -> Result<VerificationObservation>;

    fn cleanup(&mut self, action: &Action, admission: &CleanupObservation) -> Result<String>;
}

pub trait TransportPort {
    fn fetch(&mut self, action: &Action) -> Result<TransferOutcome>;
}

pub trait DurablePort {
    fn load_history(&mut self, manifest: &Manifest) -> Result<Vec<PriorOperation>>;

    fn store_operation(&mut self, operation: &PriorOperation) -> Result<String>;

    fn store_status(&mut self, status: &CanonicalReplicationRecord) -> Result<String>;
}

pub trait RetentionPort {
    fn acquire_pin(&mut self, action: &Action) -> Result<PinObservation>;

    fn authorize_cleanup(&mut self, action: &Action) -> Result<CleanupObservation>;
}

pub trait ObservationPort {
    fn publish_plan(&mut self, plan: &CanonicalReplicationRecord) -> Result<()>;

    fn publish_operation(&mut self, operation: &CanonicalReplicationRecord) -> Result<()>;

    fn publish_status(&mut self, status: &CanonicalReplicationRecord) -> Result<()>;
}

pub trait ReceiptPort {
    fn publish_receipt(&mut self, receipt: &CanonicalReplicationRecord) -> Result<()>;
}
