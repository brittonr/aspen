/// The result of one restart-fencing probe against the deterministic fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleDeliveryProbe {
    pub stale_incarnation: u64,
    pub active_incarnation: u64,
    pub denial: Option<String>,
}

/// Deliver a runtime-local callback event that carries the pre-restart
/// incarnation to the replacement instance and report whether the dispatcher
/// denied it.
pub fn probe_stale_delivery_after_restart(
    profile: super::ExecutionProfile,
) -> crate::error::Result<StaleDeliveryProbe> {
    let [admitted, ..] = fixture_manifests(profile)?;
    let mut host = super::Dispatcher::new(admitted, EchoExecutor::new(profile)?)?;
    host.activate(START_TICK)?;
    host.checkpoint(CHECKPOINT_TICK)?;
    let stale_incarnation = host.state().incarnation;
    host.observe_host_loss()?;
    host.restart(RECOVERY_TICK)?;
    let active_incarnation = host.state().incarnation;
    let stale_event = super::CallbackEvent {
        callback: super::CallbackKind::Request,
        generation: host.state().generation,
        incarnation: stale_incarnation,
        event_ref: HASH_C.to_string(),
        payload_ref: Some(HASH_B.to_string()),
        accounted_bytes: REQUEST_BYTES,
        logical_tick: POST_RECOVERY_TICK,
        deadline_tick: Some(POST_RECOVERY_TICK),
        cancellation_requested: false,
    };
    let denial = host.dispatch(stale_event).err().map(|error| error.to_string());
    Ok(StaleDeliveryProbe {
        stale_incarnation,
        active_incarnation,
        denial,
    })
}
