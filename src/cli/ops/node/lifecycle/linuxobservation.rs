//! Node startup observation. The only ambient inputs here are the host clocks and boot ID.
use molten::node_runtime::ValidatedNodeSourceGate;
use molten::node_state::DirectoryRoot;

#[cfg(target_os = "linux")]
pub(super) fn at_start(root: &DirectoryRoot, gate: &ValidatedNodeSourceGate, path: &str) -> molten::error::Result<()> {
    use std::io::Read;
    use std::io::Write;

    use molten::fabric_observability::nodeadmission as observer;

    const MAX_SELECTION_BYTES: u64 = 16 * 1024;
    const BOOT_ID_BYTES: usize = 36;

    let path = molten::node_state::RelativePath::parse(path).map_err(|_| denied())?;
    let bytes = root.read(&path, MAX_SELECTION_BYTES).map_err(|_| denied())?;
    let selection = observer::decode_node_observation_selection(&bytes).map_err(|_| denied())?;
    let mut boot = [0; BOOT_ID_BYTES + 1];
    std::fs::File::open("/proc/sys/kernel/random/boot_id")
        .and_then(|mut file| file.read_exact(&mut boot))
        .map_err(|_| denied())?;
    let boot_id = std::str::from_utf8(&boot[..BOOT_ID_BYTES]).map_err(|_| denied())?;
    if boot[BOOT_ID_BYTES] != b'\n' || boot_id != selection.boot_id {
        return Err(denied());
    }
    let unix_seconds =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| denied())?.as_secs();
    let lease = observer::admit_node_observation(root, &selection, gate, unix_seconds)?;
    let monotonic_ns = monotonic_ns()?;
    let observed = lease.poll_window(observer::ObserverWindowTick {
        boot_id,
        unix_seconds,
        monotonic_ns,
        deadline_ns: monotonic_ns,
        cancelled: false,
    })?;
    let value = molten::preserves_rail::record("node-linux-observation-start-v1", vec![
        observed.status.value,
        observed.health.value,
    ]);
    writeln!(std::io::stdout().lock(), "{}", molten::preserves_rail::to_text(&value)?)
        .map_err(molten::error::Failure::from)?;
    if observed.deny_required_readiness {
        return Err(molten::error::Failure::invalid_harness("required Linux observation unavailable at startup"));
    }
    Ok(())
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
pub(super) fn at_start(_: &DirectoryRoot, _: &ValidatedNodeSourceGate, _: &str) -> molten::error::Result<()> {
    Err(molten::error::Failure::invalid_harness("Linux observation unsupported on this platform"))
}
