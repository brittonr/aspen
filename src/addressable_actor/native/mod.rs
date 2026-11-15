//! Public-node composition of the accepted actor shell and its existing effect owners.
mod binding;
mod effects;
pub(crate) mod observation;
mod service;
mod survival;

pub use observation::CounterResult;

/// Exact application selection. References select existing owners; they grant no authority.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub actor_key: molten_core::addressable_actor::ActorKey,
    pub profile: molten_core::addressable_actor::AddressableActorProfile,
    pub node_id: String,
    pub assignment_id: String,
    pub role_id: String,
    pub delivery_member: String,
    pub queue_id: String,
}

/// Supplied currentness fence remains explicit at the public boundary.
#[derive(Clone, Debug)]
pub struct Fence {
    pub actor_key_ref: String,
    pub placement_ref: String,
    pub generation: u64,
    pub incarnation: u64,
    pub lifecycle_sequence: u64,
    pub expected: super::ExpectedState,
}

/// Exact existing owners selected when opening a native consumer.
pub struct Open<'a, 'storage, C: crate::fabric_time::TimerClockAdapter> {
    pub storage: &'storage crate::node_state::Scope,
    pub engine_epoch: u64,
    pub configuration: &'a Configuration,
    pub owner: &'a mut crate::node_service_host::installed::Owner,
    pub placement: &'a mut crate::fabric_membership::persistence::Redb,
    pub delivery: &'a mut super::delivery::Bridge<C>,
    pub signing: &'a mut dyn SigningPort,
}

/// One fenced operation and its exact retained delivery evidence.
pub struct Apply<'a> {
    pub fence: &'a Fence,
    pub operation_id: &'a str,
    pub operation: molten_core::addressable_actor::ActorOperation,
    pub logical_tick: u64,
    pub work: Option<&'a crate::node_service_hosting::DeliveryInput>,
    pub acknowledgement: Option<&'a molten_core::coordination_delivery::DeliveryRequest>,
}

/// Retains the actual public native host, assignment backend and coordination-delivery owner.
/// No actor mailbox, native journal, assignment copy or semantic-result store is introduced.
// r[impl molten.addressable_actor.public_native_consumer]
pub struct Consumer<'a, C: crate::fabric_time::TimerClockAdapter> {
    configuration: &'a Configuration,
    owner: &'a mut crate::node_service_host::installed::Owner,
    placement: &'a mut crate::fabric_membership::persistence::Redb,
    delivery: &'a mut super::delivery::Bridge<C>,
    signing: &'a mut dyn SigningPort,
    store: super::Local,
    engine_epoch: u64,
}

/// Credential transport only. Current retains all signature, policy, revocation and replay checks.
pub trait SigningPort {
    fn prepare(&mut self, request: &crate::node_service_hosting::OperationRequest) -> crate::error::Result<()>;
}

pub fn prepare(
    signing: &mut dyn SigningPort,
    owner: &crate::node_service_host::installed::Owner,
    input: crate::node_service_hosting::native::actor::Input<'_>,
    logical_tick: u64,
) -> crate::error::Result<()> {
    signing.prepare(&owner.actor_preview(input, logical_tick)?)
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn port_error(error: impl std::fmt::Debug, unknown: bool) -> super::PortError {
    super::PortError::new("public-native-actor", format!("{error:?}"), unknown)
}
