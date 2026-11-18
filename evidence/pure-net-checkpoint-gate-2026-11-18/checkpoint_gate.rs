//! Scratch probe for the Aspen #34 checkpoint gate (not committed).
//! Runs the admitted sequential-net calculation on generated `dup +` chains and
//! reports the whole restart-from-input calculation cost per phase.
use std::time::Instant;

use molten_batch_core::Id;
use molten_pure_net::*;

fn host() -> HostBindings {
    HostBindings {
        executable: Id([1; 32]),
        dependency_closure: Id([2; 32]),
    }
}

fn median(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() {
    let samples = 31;
    let grants = ExecutionGrants {
        fuel: 256_000_000,
        memory_bytes: 64 * 1024 * 1024,
        max_output_bytes: 262_144,
        max_attempts: 1,
    };
    let manifest = encode_manifest(&manifest(host(), Backend::SequentialNet, Limits::default()).unwrap()).unwrap();
    let inputs = encode_inputs(&[1]).unwrap();
    for pairs in std::env::args().skip(1).filter_map(|value| value.parse::<usize>().ok()) {
        let source = "dup + ".repeat(pairs);
        let source = source.trim_end();
        let program = match compile(source, 1) {
            Ok(program) => program,
            Err(error) => {
                println!("pairs={pairs} source_bytes={} compile_error={error:?}", source.len());
                continue;
            }
        };
        let mut totals = Vec::new();
        let mut phases: std::collections::BTreeMap<String, Vec<u128>> = Default::default();
        let mut interactions = 0;
        for _ in 0..samples {
            let mut marks: Vec<(String, Instant, u64)> = Vec::new();
            let start = Instant::now();
            let result = execute(
                ExecutionInput {
                    manifest_bytes: &manifest,
                    program_bytes: &program,
                    input_bytes: &inputs,
                    host: &host(),
                    grants: &grants,
                },
                &mut |observation: &Observation| {
                    marks.push((format!("{:?}", observation.phase), Instant::now(), observation.interactions));
                    Control::Continue
                },
            );
            let end = Instant::now();
            if let Err(error) = result {
                println!("pairs={pairs} execute_error={error:?}");
                return;
            }
            totals.push(end.duration_since(start).as_nanos());
            interactions = marks.iter().map(|mark| mark.2).max().unwrap_or(0);
            let mut starts: Vec<(String, Instant)> = Vec::new();
            for (phase, at, _) in &marks {
                if starts.last().map(|last| &last.0) != Some(phase) {
                    starts.push((phase.clone(), *at));
                }
            }
            for (index, (phase, at)) in starts.iter().enumerate() {
                let next = starts.get(index + 1).map(|next| next.1).unwrap_or(end);
                phases.entry(phase.clone()).or_default().push(next.duration_since(*at).as_nanos());
            }
        }
        let total = median(&mut totals);
        let (fastest, slowest) = (totals[0], totals[totals.len() - 1]);
        let phase_medians: Vec<String> = phases
            .iter_mut()
            .map(|(phase, values)| format!("{phase}={}", median(values)))
            .collect();
        println!(
            "pairs={pairs} source_bytes={} program_bytes={} interactions={interactions} samples={samples} total_ns_median={total} total_ns_min={fastest} total_ns_max={slowest} phases_ns_median=[{}]",
            source.len(),
            program.len(),
            phase_medians.join(" ")
        );
    }
}
