//! Diagnostic measurement of the reviewed Wasm optimization profiles.
//!
//! This example exists so issue #41's per-profile numbers can be re-derived from
//! reviewed source. It grants no capability, registers no runtime backend, and is
//! not a benchmark receipt or a release claim.
//!
//! Run: `cargo run --release --example wasm-optimization-profiles -- WORKLOAD [SAMPLES]`
//!
//! Workloads:
//! - `loop`: one instance per sample runs a bounded integer loop, so per-call cost dominates
//! - `short-lived`: one instance per sample runs a trivial function, `SAMPLES` times, so instance
//!   churn dominates
//!
//! The four measured configurations mirror `molten.wasm.optimization.*.v1`:
//! - baseline: no pooling allocator, copy-on-write heap images off, no pre-instantiation
//! - pooling: pooling allocator sized from the reviewed bounds
//! - cow: copy-on-write heap images on, no pooling allocator
//! - instance-pre: one pre-instantiation reused for every measured call
#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    tigerstyle::ambient_clock,
    reason = "the diagnostic measurement must record elapsed host time per phase"
)]

use std::time::Duration;
use std::time::Instant;

use wasmtime::Config;
use wasmtime::Engine;
use wasmtime::InstanceAllocationStrategy;
use wasmtime::InstancePre;
use wasmtime::Linker;
use wasmtime::Module;
use wasmtime::PoolingAllocationConfig;
use wasmtime::Store;

/// Per-call cost dominates: a bounded integer loop plus one 64 KiB memory page touched per call.
const LOOP: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "run") (param $n i64) (result i64)
    (local $i i64)
    (local $acc i64)
    (i32.store8 (i32.const 0) (i32.wrap_i64 (local.get $n)))
    (local.set $acc (i64.const 1))
    (block $done
      (loop $loop
        (br_if $done (i64.ge_u (local.get $i) (local.get $n)))
        (local.set $acc
          (i64.rem_u
            (i64.mul (local.get $acc) (i64.add (local.get $i) (i64.const 3)))
            (i64.const 1000000007)))
        (local.set $i (i64.add (local.get $i) (i64.const 1)))
        (br $loop)))
    (i64.add (local.get $acc) (i64.extend_i32_u (i32.load8_u (i32.const 0)))))
)
"#;

/// Instance churn dominates: one memory page and one trivial exported function.
const SHORT_LIVED: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "run") (param $n i64) (result i64)
    (i32.store8 (i32.const 0) (i32.wrap_i64 (local.get $n)))
    (i64.extend_i32_u (i32.load8_u (i32.const 0))))
)
"#;

const FUEL: u64 = 10_000_000;
const MAX_MEMORIES: u32 = 16;
const MAX_TABLES: u32 = 16;
const MAX_MEMORY_BYTES: usize = 1 << 20;
const LOOP_SAMPLES: u64 = 100;
const SHORT_LIVED_SAMPLES: u64 = 500;
const LOOP_BUDGET: i64 = 50_000;
const SHORT_LIVED_BUDGET: i64 = 1;

#[derive(Clone, Copy)]
struct Profile {
    name: &'static str,
    pooling: bool,
    copy_on_write: bool,
    instance_pre: bool,
}

const PROFILES: [Profile; 4] = [
    Profile {
        name: "baseline",
        pooling: false,
        copy_on_write: false,
        instance_pre: false,
    },
    Profile {
        name: "pooling",
        pooling: true,
        copy_on_write: false,
        instance_pre: false,
    },
    Profile {
        name: "cow",
        pooling: false,
        copy_on_write: true,
        instance_pre: false,
    },
    Profile {
        name: "instance-pre",
        pooling: false,
        copy_on_write: false,
        instance_pre: true,
    },
];

// r[impl wasm_profile_measurement.tool]
fn engine(profile: Profile) -> Result<Engine, Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.consume_fuel(true).cranelift_nan_canonicalization(true);
    config.memory_init_cow(profile.copy_on_write);
    if profile.pooling {
        let mut pooling = PoolingAllocationConfig::default();
        pooling.total_memories(MAX_MEMORIES).total_tables(MAX_TABLES);
        pooling.max_memory_size(MAX_MEMORY_BYTES);
        config.allocation_strategy(InstanceAllocationStrategy::from(pooling));
    }
    Ok(Engine::new(&config)?)
}

// r[impl wasm_profile_measurement.aggregation]
fn json_round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Median, p90 and max of the recorded microseconds; pure in its input order.
// r[impl wasm_profile_measurement.aggregation]
fn micros(samples: &mut Vec<Duration>) -> (f64, f64, f64) {
    samples.sort_unstable();
    let at = |fraction: f64| samples[((samples.len() - 1) as f64 * fraction).round() as usize].as_secs_f64() * 1e6;
    let (median, p90, max) = (at(0.5), at(0.9), at(1.0));
    (json_round(median), json_round(p90), json_round(max))
}

// r[impl wasm_profile_measurement.aggregation]
fn phase(mut samples: Vec<Duration>) -> serde_json::Value {
    let (median, p90, max) = micros(&mut samples);
    serde_json::json!({"median": median, "p90": p90, "max": max})
}

// r[impl wasm_profile_measurement.reporting]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workload = std::env::args().nth(1).unwrap_or_else(|| "loop".to_string());
    let short_lived = workload == "short-lived";
    let default_samples = if short_lived { SHORT_LIVED_SAMPLES } else { LOOP_SAMPLES };
    let samples: u64 = std::env::args().nth(2).map(|value| value.parse()).transpose()?.unwrap_or(default_samples);
    let wat = if short_lived { SHORT_LIVED } else { LOOP };
    let budget = if short_lived { SHORT_LIVED_BUDGET } else { LOOP_BUDGET };
    let mut profiles = Vec::with_capacity(PROFILES.len());
    for profile in PROFILES.iter() {
        let engine = engine(*profile)?;
        let module = Module::new(&engine, wat)?;
        let linker: Linker<()> = Linker::new(&engine);
        let pre: Option<InstancePre<()>> = if profile.instance_pre {
            Some(linker.instantiate_pre(&module)?)
        } else {
            None
        };
        // One warm-up sample per profile, so the recorded samples measure steady state.
        let mut compile = Vec::with_capacity(samples as usize);
        let mut call = Vec::with_capacity(samples as usize);
        for sample in 0..=samples {
            let started = Instant::now();
            let fresh = Module::new(&engine, wat)?;
            let compile_elapsed = started.elapsed();
            let started = Instant::now();
            for _ in 0..if short_lived { samples } else { 1 } {
                let mut store = Store::new(&engine, ());
                store.set_fuel(FUEL)?;
                let instance = match &pre {
                    Some(pre) => pre.instantiate(&mut store)?,
                    None => linker.instantiate(&mut store, &fresh)?,
                };
                let run = instance.get_typed_func::<i64, i64>(&mut store, "run")?;
                let result = run.call(&mut store, budget)?;
                std::hint::black_box(result);
            }
            let call_elapsed = started.elapsed();
            if sample > 0 {
                compile.push(compile_elapsed);
                call.push(call_elapsed);
            }
        }
        profiles.push(serde_json::json!({
            "profile": profile.name,
            "pooling": profile.pooling,
            "copy_on_write": profile.copy_on_write,
            "instance_pre": profile.instance_pre,
            "compile_us": phase(compile),
            "instantiate_and_execute_us": phase(call),
        }));
    }
    let report = serde_json::json!({
        "workload": if short_lived { "short-lived-instances-v1" } else { "bounded-integer-loop-v1" },
        "samples": samples,
        "instances_per_sample": if short_lived { samples } else { 1 },
        "profiles": profiles,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify wasm_profile_measurement.aggregation]
    #[test]
    fn percentiles_depend_only_on_the_recorded_samples() {
        let mut order = vec![
            Duration::from_micros(5),
            Duration::from_micros(1),
            Duration::from_micros(3),
            Duration::from_micros(2),
            Duration::from_micros(4),
        ];
        let mut reversed = order.clone();
        reversed.reverse();
        assert_eq!(micros(&mut order), micros(&mut reversed));
        assert_eq!(micros(&mut order), (3.0, 5.0, 5.0));
    }

    // r[verify wasm_profile_measurement.aggregation]
    #[test]
    fn single_sample_is_its_own_percentile() {
        let mut one = vec![Duration::from_micros(7)];
        assert_eq!(micros(&mut one), (7.0, 7.0, 7.0));
    }

    // r[verify wasm_profile_measurement.reporting]
    #[test]
    fn report_names_the_workload_and_sample_counts_without_a_verdict() {
        let value = phase(vec![Duration::from_micros(4), Duration::from_micros(8)]);
        let object = value.as_object().expect("phase object");
        let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["max", "median", "p90"]);
    }
}
