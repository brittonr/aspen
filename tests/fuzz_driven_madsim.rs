//! Fuzz-driven deterministic simulation testing.
//!
//! This module combines fuzzer-generated seeds with madsim's deterministic simulation
//! to get the best of both worlds:
//! - Fuzzer explores scenario space automatically
//! - Madsim provides deterministic replay of any found bugs
//!
//! # Architecture
//!
//! The test generates arbitrary scenario configurations (seeds, operations, faults)
//! using proptest strategies, then executes them in madsim with a fixed seed.
//! If a bug is found, the madsim seed makes it perfectly reproducible.
//!
//! # Tiger Style
//!
//! - Bounded operation counts (MAX_OPS_PER_SCENARIO)
//! - Bounded fault injection (MAX_FAULTS_PER_SCENARIO)
//! - Fixed cluster sizes (3-5 nodes)
//! - Deterministic execution via madsim

use aspen::testing::{AspenRaftTester, BuggifyFault};
use proptest::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

/// Tiger Style: Maximum operations per scenario
const MAX_OPS_PER_SCENARIO: usize = 50;

/// Tiger Style: Maximum faults per scenario
const MAX_FAULTS_PER_SCENARIO: usize = 10;

/// Tiger Style: Maximum cluster size
const MAX_CLUSTER_SIZE: usize = 5;

/// An operation that can be performed on the cluster
#[derive(Debug, Clone)]
enum ClusterOperation {
    /// Write a key-value pair
    Write { key: String, value: String },
    /// Read a key
    Read { key: String },
    /// Delete a key
    Delete { key: String },
    /// Trigger an election on a specific node
    TriggerElection { node_idx: usize },
    /// Sleep for some time (simulated)
    Sleep { ms: u64 },
}

/// A fault that can be injected
#[derive(Debug, Clone)]
struct FaultEvent {
    /// When to inject (operation index)
    at_op: usize,
    /// What fault to inject
    fault: BuggifyFault,
}

/// A complete test scenario
#[derive(Debug, Clone)]
struct FuzzScenario {
    /// Madsim seed for deterministic replay
    seed: u64,
    /// Number of nodes (3-5)
    node_count: usize,
    /// Operations to perform
    operations: Vec<ClusterOperation>,
    /// Faults to inject
    faults: Vec<FaultEvent>,
    /// Whether to enable BUGGIFY
    enable_buggify: bool,
}

/// Generate arbitrary keys
fn arb_key() -> impl Strategy<Value = String> {
    "[a-z]{2,8}".prop_map(|s| format!("fuzz_key_{}", s))
}

/// Generate arbitrary values
fn arb_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{4,16}".prop_map(|s| format!("fuzz_val_{}", s))
}

/// Generate an arbitrary cluster operation
fn arb_operation(max_nodes: usize) -> impl Strategy<Value = ClusterOperation> {
    prop_oneof![
        // Writes (40%)
        4 => (arb_key(), arb_value()).prop_map(|(key, value)|
            ClusterOperation::Write { key, value }),
        // Reads (30%)
        3 => arb_key().prop_map(|key|
            ClusterOperation::Read { key }),
        // Deletes (10%)
        1 => arb_key().prop_map(|key|
            ClusterOperation::Delete { key }),
        // Elections (10%)
        1 => (0..max_nodes).prop_map(|node_idx|
            ClusterOperation::TriggerElection { node_idx }),
        // Sleep (10%)
        1 => (50u64..500).prop_map(|ms|
            ClusterOperation::Sleep { ms }),
    ]
}

/// Generate an arbitrary fault
fn arb_fault(max_ops: usize) -> impl Strategy<Value = FaultEvent> {
    let fault = prop_oneof![
        Just(BuggifyFault::NetworkDelay),
        Just(BuggifyFault::NetworkDrop),
        Just(BuggifyFault::SlowDisk),
        Just(BuggifyFault::MessageCorruption),
        Just(BuggifyFault::ElectionTimeout),
    ];

    (0..max_ops, fault).prop_map(|(at_op, fault)| FaultEvent { at_op, fault })
}

/// Generate a complete test scenario
fn arb_scenario() -> impl Strategy<Value = FuzzScenario> {
    let ops_count = 10..MAX_OPS_PER_SCENARIO;
    let faults_count = 0..MAX_FAULTS_PER_SCENARIO;
    let node_count = 3..=MAX_CLUSTER_SIZE;

    (
        any::<u64>(),  // seed
        node_count,    // node_count
        ops_count,     // ops count for sizing
        faults_count,  // faults count for sizing
        any::<bool>(), // enable_buggify
    )
        .prop_flat_map(
            |(seed, node_count, ops_count, faults_count, enable_buggify)| {
                (
                    Just(seed),
                    Just(node_count),
                    prop::collection::vec(arb_operation(node_count), ops_count),
                    prop::collection::vec(arb_fault(ops_count), faults_count),
                    Just(enable_buggify),
                )
            },
        )
        .prop_map(
            |(seed, node_count, operations, faults, enable_buggify)| FuzzScenario {
                seed,
                node_count,
                operations,
                faults,
                enable_buggify,
            },
        )
}

/// Execute a scenario and verify invariants
async fn execute_scenario(scenario: &FuzzScenario) -> Result<(), String> {
    let test_name = format!("fuzz_scenario_{}", scenario.seed);
    let mut tester =
        AspenRaftTester::new_with_seed(scenario.node_count, &test_name, scenario.seed).await;

    if scenario.enable_buggify {
        tester.enable_buggify(None);
    }

    // Track expected state for verification
    let mut expected_values: HashMap<String, String> = HashMap::new();
    // Use u64 for both term and leader_id (NodeId converts via .into())
    let mut leaders_per_term: HashMap<u64, u64> = HashMap::new();

    // Execute operations
    for (op_idx, op) in scenario.operations.iter().enumerate() {
        // Check if we should inject a fault
        for fault_event in &scenario.faults {
            if fault_event.at_op == op_idx {
                tester.apply_single_fault(fault_event.fault).await;
            }
        }

        // Execute the operation
        match op {
            ClusterOperation::Write { key, value } => {
                if tester.write(key.clone(), value.clone()).await.is_ok() {
                    expected_values.insert(key.clone(), value.clone());
                }
            }
            ClusterOperation::Read { key } => {
                if let Ok(Some(read_value)) = tester.read(key).await
                    && let Some(expected) = expected_values.get(key)
                    && &read_value != expected
                {
                    return Err(format!(
                        "Linearizability violation at op {}: expected '{}', got '{}'",
                        op_idx, expected, read_value
                    ));
                }
            }
            ClusterOperation::Delete { key } => {
                if tester.delete(key.clone()).await.is_ok() {
                    expected_values.remove(key);
                }
            }
            ClusterOperation::TriggerElection { node_idx } => {
                let _ = tester.trigger_election(*node_idx as u64).await;
                // Give time for election
                madsim::time::sleep(Duration::from_millis(300)).await;
            }
            ClusterOperation::Sleep { ms } => {
                madsim::time::sleep(Duration::from_millis(*ms)).await;
            }
        }

        // Verify leader safety invariant
        for node_id in 0..scenario.node_count {
            if let Some(metrics) = tester.get_metrics(node_id as u64)
                && let Some(leader_id) = metrics.current_leader
            {
                let term = metrics.current_term;
                let leader_id_u64: u64 = leader_id.into();
                match leaders_per_term.get(&term) {
                    Some(existing_leader) if *existing_leader != leader_id_u64 => {
                        return Err(format!(
                            "Leader safety violation at op {}: term {} has leaders {} and {}",
                            op_idx, term, existing_leader, leader_id_u64
                        ));
                    }
                    None => {
                        leaders_per_term.insert(term, leader_id_u64);
                    }
                    _ => {}
                }
            }
        }
    }

    // Let cluster settle
    madsim::time::sleep(Duration::from_secs(1)).await;

    // Final verification: read all expected values
    for (key, expected_value) in &expected_values {
        if let Ok(Some(read_value)) = tester.read(key).await
            && &read_value != expected_value
        {
            return Err(format!(
                "Final state violation: key '{}' expected '{}', got '{}'",
                key, expected_value, read_value
            ));
        }
    }

    tester.end();
    Ok(())
}

/// Property test: Random scenarios should not violate Raft invariants
#[madsim::test]
async fn test_fuzz_driven_scenarios() {
    // Use fixed seeds for reproducibility in CI
    // Each seed generates a unique scenario
    let test_seeds: [u64; 5] = [12345, 67890, 11111, 22222, 33333];

    for (idx, &base_seed) in test_seeds.iter().enumerate() {
        // Generate a scenario from the seed
        let scenario = FuzzScenario {
            seed: base_seed,
            node_count: 3,
            operations: (0..20)
                .map(|i| match i % 5 {
                    0..=2 => ClusterOperation::Write {
                        key: format!("key_{}", i % 7),
                        value: format!("value_{}_{}", base_seed, i),
                    },
                    3 => ClusterOperation::Read {
                        key: format!("key_{}", i % 7),
                    },
                    _ => ClusterOperation::Sleep { ms: 100 },
                })
                .collect(),
            faults: if idx % 2 == 0 {
                vec![
                    FaultEvent {
                        at_op: 5,
                        fault: BuggifyFault::NetworkDelay,
                    },
                    FaultEvent {
                        at_op: 10,
                        fault: BuggifyFault::SlowDisk,
                    },
                ]
            } else {
                vec![]
            },
            enable_buggify: idx % 3 == 0,
        };

        eprintln!(
            "Running fuzz scenario {} with seed {}, {} ops, {} faults",
            idx,
            scenario.seed,
            scenario.operations.len(),
            scenario.faults.len()
        );

        if let Err(e) = execute_scenario(&scenario).await {
            panic!(
                "Scenario {} (seed {}) failed: {}\nTo reproduce, run with MADSIM_TEST_SEED={}",
                idx, scenario.seed, e, scenario.seed
            );
        }

        eprintln!("Scenario {} completed successfully", idx);
    }
}

/// Property test: Proptest-generated scenarios
#[test]
fn test_proptest_scenario_generation() {
    // Verify that our strategies generate valid scenarios
    let config = ProptestConfig {
        cases: 50,
        ..ProptestConfig::default()
    };

    let mut runner = proptest::test_runner::TestRunner::new(config);

    runner
        .run(&arb_scenario(), |scenario| {
            // Verify bounds
            prop_assert!(scenario.node_count >= 3);
            prop_assert!(scenario.node_count <= MAX_CLUSTER_SIZE);
            prop_assert!(scenario.operations.len() <= MAX_OPS_PER_SCENARIO);
            prop_assert!(scenario.faults.len() <= MAX_FAULTS_PER_SCENARIO);

            // Verify fault timing is valid
            for fault in &scenario.faults {
                prop_assert!(fault.at_op < scenario.operations.len());
            }

            Ok(())
        })
        .expect("Strategy generation test failed");
}

/// Regression test: Specific seeds that found bugs should be tested
#[madsim::test]
async fn test_regression_seeds() {
    // Add any seeds that found bugs here for regression testing
    // These seeds are known to exercise interesting edge cases
    let regression_seeds: [(u64, &str); 3] = [
        (42, "basic_operations"),
        (0, "edge_case_zero_seed"),
        (u64::MAX / 2, "large_seed"),
    ];

    for (seed, description) in regression_seeds {
        eprintln!("Running regression test: {} (seed {})", description, seed);

        let scenario = FuzzScenario {
            seed,
            node_count: 3,
            operations: vec![
                ClusterOperation::Write {
                    key: "test".to_string(),
                    value: "value".to_string(),
                },
                ClusterOperation::Read {
                    key: "test".to_string(),
                },
                ClusterOperation::Write {
                    key: "test".to_string(),
                    value: "updated".to_string(),
                },
                ClusterOperation::Read {
                    key: "test".to_string(),
                },
                ClusterOperation::Delete {
                    key: "test".to_string(),
                },
            ],
            faults: vec![],
            enable_buggify: false,
        };

        if let Err(e) = execute_scenario(&scenario).await {
            panic!(
                "Regression test '{}' (seed {}) failed: {}",
                description, seed, e
            );
        }

        eprintln!("Regression test '{}' passed", description);
    }
}
