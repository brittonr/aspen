//! Standalone test binary for job orchestration primitives.
//!
//! Exercises job subsystems against a `DeterministicKeyValueStore`:
//! dependency tracking, scheduling, dead letter queue, saga execution,
//! workflow engine, affinity routing, and deterministic replay.
//!
//! Each subcommand runs a specific test scenario and exits 0 on success,
//! 1 on failure. Designed to be called from NixOS VM integration tests.
//!
//! Usage:
//!   aspen-job-primitives-test dependency
//!   aspen-job-primitives-test scheduler
//!   aspen-job-primitives-test dlq
//!   aspen-job-primitives-test saga
//!   aspen-job-primitives-test workflow
//!   aspen-job-primitives-test affinity
//!   aspen-job-primitives-test replay
//!   aspen-job-primitives-test all

use std::sync::Arc;

use aspen_testing::DeterministicKeyValueStore;
use clap::Parser;

mod tests_affinity;
mod tests_dependency;
mod tests_dlq;
mod tests_replay;
mod tests_saga;
mod tests_scheduler;
mod tests_workflow;

#[derive(Parser)]
#[command(name = "aspen-job-primitives-test")]
struct Args {
    /// Test to run: dependency, scheduler, dlq, saga, workflow, affinity, replay, all
    test: String,
}

type TestResult = Result<(), String>;

fn make_store() -> Arc<DeterministicKeyValueStore> {
    DeterministicKeyValueStore::new()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let args = Args::parse();

    let result = match args.test.as_str() {
        "dependency" => tests_dependency::run().await,
        "scheduler" => tests_scheduler::run().await,
        "dlq" => tests_dlq::run().await,
        "saga" => tests_saga::run().await,
        "workflow" => tests_workflow::run().await,
        "affinity" => tests_affinity::run().await,
        "replay" => tests_replay::run().await,
        "all" => run_all().await,
        other => {
            eprintln!("Unknown test: {other}");
            eprintln!("Available: dependency, scheduler, dlq, saga, workflow, affinity, replay, all");
            std::process::exit(2);
        }
    };

    match result {
        Ok(()) => {
            println!("PASS");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1);
        }
    }
}

async fn run_all() -> TestResult {
    let tests: &[(&str, fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = TestResult> + Send>>)] = &[
        ("dependency", || Box::pin(tests_dependency::run())),
        ("scheduler", || Box::pin(tests_scheduler::run())),
        ("dlq", || Box::pin(tests_dlq::run())),
        ("saga", || Box::pin(tests_saga::run())),
        ("workflow", || Box::pin(tests_workflow::run())),
        ("affinity", || Box::pin(tests_affinity::run())),
        ("replay", || Box::pin(tests_replay::run())),
    ];

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut failures = Vec::new();

    for (name, test_fn) in tests {
        print!("  {name} ... ");
        match test_fn().await {
            Ok(()) => {
                println!("PASS");
                passed += 1;
            }
            Err(e) => {
                println!("FAIL: {e}");
                failed += 1;
                failures.push((*name, e));
            }
        }
    }

    println!("\n{passed} passed, {failed} failed");

    if failed > 0 {
        let names: Vec<&str> = failures.iter().map(|(n, _)| *n).collect();
        Err(format!("Failed tests: {}", names.join(", ")))
    } else {
        Ok(())
    }
}
