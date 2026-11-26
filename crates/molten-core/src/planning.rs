//! Adapter effect-port vocabulary.
//!
//! `EffectKind` names the adapter effect ports that the deterministic replay boundary classifies
//! (`world_replay::boundary::sources` in the root crate). Admission decisions live with the
//! production shells that perform each effect; this module carries no planner.

/// One adapter effect port whose replay source the world-replay boundary must classify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    StoreWrite,
    StoreDelete,
    TransportSend,
    ExecuteWorker,
    ClockRead,
    ReceiptWrite,
    RegistryRead,
}
