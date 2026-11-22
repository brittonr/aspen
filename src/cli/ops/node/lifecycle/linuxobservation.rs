//! Node startup observation. The only ambient inputs here are the host clocks and boot ID.

pub(super) struct StartupFence {
    #[cfg(target_os = "linux")]
    readiness: molten::fabric_observability::nodeadmission::ObserverReadiness,
    #[cfg(target_os = "linux")]
    clock: molten::fabric_time::LiveClockAdapter,
}

impl StartupFence {
    #[cfg(target_os = "linux")]
    pub(super) fn validate(&mut self) -> molten::error::Result<()> {
        self.readiness
            .validate(molten::world_witness::WallClock::unix_seconds(&mut self.clock)?, monotonic_ns()?)
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn validate(&mut self) -> molten::error::Result<()> {
        Err(unsupported())
    }
}

#[cfg(target_os = "linux")]
pub(super) fn at_start(
    root: &molten::node_state::DirectoryRoot,
    gate: &molten::node_runtime::ValidatedNodeSourceGate,
    path: &str,
) -> molten::error::Result<Option<StartupFence>> {
    use std::io::Read;
    use std::io::Write;

    const MAX_SELECTION_BYTES: u64 = 16 * 1024;
    const BOOT_ID_BYTES: usize = 36;

    let path = molten::node_state::RelativePath::parse(path).map_err(|_| denied())?;
    let bytes = root.read(&path, MAX_SELECTION_BYTES).map_err(|_| denied())?;
    let selection =
        molten::fabric_observability::nodeadmission::decode_node_observation_selection(&bytes).map_err(|_| denied())?;
    let mut boot = [0; BOOT_ID_BYTES + 1];
    std::fs::File::open("/proc/sys/kernel/random/boot_id")
        .and_then(|mut file| file.read_exact(&mut boot))
        .map_err(|_| denied())?;
    let boot_id = std::str::from_utf8(&boot[..BOOT_ID_BYTES]).map_err(|_| denied())?;
    if boot[BOOT_ID_BYTES] != b'\n' || boot_id != selection.boot_id {
        return Err(denied());
    }
    let mut clock = molten::world_witness::live_clock()?;
    let unix_seconds = molten::world_witness::WallClock::unix_seconds(&mut clock)?;
    let lease =
        molten::fabric_observability::nodeadmission::admit_node_observation(root, &selection, gate, unix_seconds)?;
    let monotonic_ns = monotonic_ns()?;
    let readback = lease.poll_window(molten::fabric_observability::nodeadmission::ObserverWindowTick {
        boot_id,
        unix_seconds,
        monotonic_ns,
        deadline_ns: monotonic_ns,
        cancelled: false,
    })?;
    let observed = readback.observation;
    let value = molten::preserves_rail::record("node-linux-observation-start-v1", vec![
        observed.status.value,
        observed.health.value,
    ]);
    writeln!(std::io::stdout().lock(), "{}", molten::preserves_rail::to_text(&value)?)
        .map_err(molten::error::Failure::from)?;
    if observed.deny_required_readiness {
        return Err(molten::error::Failure::invalid_harness("required Linux observation unavailable at startup"));
    }
    Ok(readback.readiness.map(|readiness| StartupFence { readiness, clock }))
}

#[cfg(target_os = "linux")]
fn monotonic_ns() -> molten::error::Result<u64> {
    let clock = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    let seconds = u64::try_from(clock.tv_sec).map_err(|_| denied())?;
    let nanos = u32::try_from(clock.tv_nsec).map_err(|_| denied())?;
    u64::try_from(std::time::Duration::new(seconds, nanos).as_nanos()).map_err(|_| denied())
}

#[cfg(target_os = "linux")]
fn denied() -> molten::error::Failure {
    molten::error::Failure::invalid_harness("Linux observation startup selection denied")
}

#[cfg(not(target_os = "linux"))]
pub(super) fn at_start(
    _: &molten::node_state::DirectoryRoot,
    _: &molten::node_runtime::ValidatedNodeSourceGate,
    _: &str,
) -> molten::error::Result<Option<StartupFence>> {
    Err(unsupported())
}

#[cfg(not(target_os = "linux"))]
fn unsupported() -> molten::error::Failure {
    molten::error::Failure::invalid_harness("Linux observation unsupported on this platform")
}
