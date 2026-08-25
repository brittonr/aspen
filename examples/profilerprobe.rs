#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    tigerstyle::ambient_clock,
    reason = "the development-only profiler probe requires elapsed host time for a bounded capture"
)]

const PROBE_RUNTIME: std::time::Duration = std::time::Duration::from_secs(3);
const PROBE_PAUSE: std::time::Duration = std::time::Duration::from_millis(1);

#[cfg_attr(
    any(feature = "profiler", feature = "profiler-disabled"),
    flux_profiler::timed("molten_profiler_probe_frame")
)]
fn profiled_probe_frame(value: u64) -> u64 {
    std::hint::black_box(value.wrapping_mul(value))
}

fn main() {
    molten::profiling::enable_development_profiler();
    let start = std::time::Instant::now();
    let mut value = 1_u64;
    while start.elapsed() < PROBE_RUNTIME {
        value = profiled_probe_frame(value);
        std::thread::sleep(PROBE_PAUSE);
    }
    std::hint::black_box(value);
}
