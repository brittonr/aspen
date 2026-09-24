use molten_core::content_replication::*;

use super::*;

pub trait AuthorityPort {
    fn observe(&mut self, manifest: &Manifest) -> crate::error::Result<AuthorityObservation>;
}

pub trait IdentityPort {
    fn observe(&mut self, manifest: &Manifest) -> crate::error::Result<IdentityObservation>;
}

pub trait MembershipPort {
    fn observe(&mut self, manifest: &Manifest) -> crate::error::Result<MembershipObservation>;
}

pub trait PlacementPort {
    fn observe(&mut self, manifest: &Manifest) -> crate::error::Result<PlacementObservation>;
}

pub trait TimePort {
    fn observe(&mut self, manifest: &Manifest) -> crate::error::Result<TimeObservation>;
}

pub trait ResourcePort {
    fn reserve(&mut self, plan: &Plan) -> crate::error::Result<ResourceObservation>;
}

pub trait ContentPort {
    fn inventory(&mut self, manifest: &Manifest) -> crate::error::Result<Inventory>;

    fn verify(&mut self, action: &Action, envelope: &TransferEnvelope)
    -> crate::error::Result<VerificationObservation>;

    fn cleanup(&mut self, action: &Action, admission: &CleanupObservation) -> crate::error::Result<String>;
}

pub trait TransportPort {
    fn fetch(&mut self, action: &Action) -> crate::error::Result<TransferOutcome>;
}

pub trait DurablePort {
    fn load_history(&mut self, manifest: &Manifest) -> crate::error::Result<Vec<PriorOperation>>;

    fn store_operation(&mut self, operation: &PriorOperation) -> crate::error::Result<String>;

    fn store_status(&mut self, status: &CanonicalReplicationRecord) -> crate::error::Result<String>;
}

// r[impl molten.content_replication.retention_confidentiality]
pub trait RetentionPort {
    fn acquire_pin(&mut self, action: &Action) -> crate::error::Result<PinObservation>;

    fn authorize_cleanup(&mut self, action: &Action) -> crate::error::Result<CleanupObservation>;
}

pub trait ObservationPort {
    fn publish_plan(&mut self, plan: &CanonicalReplicationRecord) -> crate::error::Result<()>;

    fn publish_operation(&mut self, operation: &CanonicalReplicationRecord) -> crate::error::Result<()>;

    fn publish_status(&mut self, status: &CanonicalReplicationRecord) -> crate::error::Result<()>;
}

pub trait ReceiptPort {
    fn publish_receipt(&mut self, receipt: &CanonicalReplicationRecord) -> crate::error::Result<()>;
}
