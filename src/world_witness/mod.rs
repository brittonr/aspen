pub mod authority;
mod lifecycle;
pub mod provider;
mod records;
pub mod recovery;
pub(crate) mod store;

#[cfg(test)]
mod tests;

pub use lifecycle::adapters::LivePorts;
pub use lifecycle::adapters::live_clock;
pub use lifecycle::current::*;
pub use lifecycle::durable::*;
pub use lifecycle::local::advance_generation;
pub use lifecycle::shell::*;
pub use records::*;
pub use recovery::resume::*;
pub use store::*;

/// The fresh callback authenticates the actual claim and observes Basalt/UCAN
/// branch authority. A provider response is never supplied as branch authority.
pub trait FreshAdmission {
    fn witness_policy_ref(&mut self) -> crate::error::Result<String>;
    fn recheck(
        &mut self,
        claim: &crate::world_head::CanonicalWorldHeadClaim,
        observed: Option<&molten_core::world_head::WorldHeadState>,
    ) -> crate::error::Result<crate::world_head::WorldHeadFreshAdmission>;
}

pub trait WitnessRecheck {
    fn recheck(
        &mut self,
        claim: &crate::world_head::CanonicalWorldHeadClaim,
        observed: Option<&molten_core::world_head::WorldHeadState>,
        retained: molten_core::world_witness::RetainedWitnessState,
    ) -> crate::error::Result<(crate::world_head::WorldHeadFreshAdmission, molten_core::world_witness::CurrentnessInput)>;
}

/// Shell-selected wall clock. Witness cores observe Unix time only through this port.
pub trait WallClock {
    fn unix_seconds(&mut self) -> crate::error::Result<u64>;
}

pub struct Transition<'a> {
    pub claim: &'a crate::world_head::CanonicalWorldHeadClaim,
    pub plan: &'a molten_core::world_head::WorldHeadTransitionPlan,
    pub authentication_ref: &'a str,
    pub authority_ref: &'a str,
    pub currentness: molten_core::world_witness::CurrentnessInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Committed {
        receipt_ref: String,
    },
    AlreadyComplete,
    Superseded,
    Uncertain {
        operation_ref: String,
        phase: WitnessUncertainty,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessUncertainty {
    Staging,
    Reserve,
    LocalCommit,
    ExternalCommit,
    Orphan,
}
