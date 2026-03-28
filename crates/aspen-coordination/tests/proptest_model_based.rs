//! Model-based property tests for coordination primitives.
//!
//! Each test generates a random sequence of operations, applies them to both
//! the real async implementation (backed by DeterministicKeyValueStore) and a
//! simple reference model. Every operation's result must match between the two.
//!
//! This catches bugs in the async shell layer that Verus-verified pure functions
//! can't reach: retry logic, CAS loops, serialization, key encoding, etc.

use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_coordination::SignedAtomicCounter;
use aspen_testing::DeterministicKeyValueStore;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Counter model
// ---------------------------------------------------------------------------

/// Reference model for AtomicCounter: just a u64 with saturating arithmetic.
#[derive(Debug, Clone, Default)]
struct CounterModel {
    value: u64,
}

impl CounterModel {
    fn increment(&mut self) -> u64 {
        self.value = self.value.saturating_add(1);
        self.value
    }
    fn decrement(&mut self) -> u64 {
        self.value = self.value.saturating_sub(1);
        self.value
    }
    fn add(&mut self, n: u64) -> u64 {
        self.value = self.value.saturating_add(n);
        self.value
    }
    fn subtract(&mut self, n: u64) -> u64 {
        self.value = self.value.saturating_sub(n);
        self.value
    }
    fn set(&mut self, v: u64) {
        self.value = v;
    }
    fn compare_and_set(&mut self, expected: u64, new: u64) -> bool {
        if self.value == expected {
            self.value = new;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
enum CounterOp {
    Increment,
    Decrement,
    Add(u64),
    Subtract(u64),
    Set(u64),
    CompareAndSet { expected: u64, new_value: u64 },
    Get,
}

fn counter_op_strategy() -> impl Strategy<Value = CounterOp> {
    prop_oneof![
        3 => Just(CounterOp::Increment),
        3 => Just(CounterOp::Decrement),
        2 => (0u64..1000).prop_map(CounterOp::Add),
        2 => (0u64..1000).prop_map(CounterOp::Subtract),
        1 => (0u64..10000).prop_map(CounterOp::Set),
        2 => (0u64..10000, 0u64..10000)
            .prop_map(|(e, n)| CounterOp::CompareAndSet {
                expected: e,
                new_value: n,
            }),
        2 => Just(CounterOp::Get),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Counter: model tracks real implementation across random op sequences.
    #[test]
    fn test_counter_model_equivalence(
        ops in prop::collection::vec(counter_op_strategy(), 1..50)
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let store = DeterministicKeyValueStore::new();
            let counter = AtomicCounter::new(
                store.clone(),
                "test_counter",
                CounterConfig::default(),
            );
            let mut model = CounterModel::default();

            for op in &ops {
                match op {
                    CounterOp::Increment => {
                        let real = counter.increment().await.unwrap();
                        let expected = model.increment();
                        assert_eq!(real, expected, "increment mismatch");
                    }
                    CounterOp::Decrement => {
                        let real = counter.decrement().await.unwrap();
                        let expected = model.decrement();
                        assert_eq!(real, expected, "decrement mismatch");
                    }
                    CounterOp::Add(n) => {
                        let real = counter.add(*n).await.unwrap();
                        let expected = model.add(*n);
                        assert_eq!(real, expected, "add({n}) mismatch");
                    }
                    CounterOp::Subtract(n) => {
                        let real = counter.subtract(*n).await.unwrap();
                        let expected = model.subtract(*n);
                        assert_eq!(real, expected, "subtract({n}) mismatch");
                    }
                    CounterOp::Set(v) => {
                        counter.set(*v).await.unwrap();
                        model.set(*v);
                    }
                    CounterOp::CompareAndSet { expected, new_value } => {
                        let real = counter
                            .compare_and_set(*expected, *new_value)
                            .await
                            .unwrap();
                        let model_result = model.compare_and_set(*expected, *new_value);
                        assert_eq!(real, model_result, "CAS({expected}, {new_value}) mismatch");
                    }
                    CounterOp::Get => {
                        let real = counter.get().await.unwrap();
                        assert_eq!(real, model.value, "get mismatch");
                    }
                }
            }

            // Final consistency check
            let final_value = counter.get().await.unwrap();
            assert_eq!(final_value, model.value, "final value mismatch");
        });
    }
}

// ---------------------------------------------------------------------------
// Signed counter model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct SignedCounterModel {
    value: i64,
}

impl SignedCounterModel {
    fn add(&mut self, n: i64) -> i64 {
        self.value = self.value.saturating_add(n);
        self.value
    }
}

#[derive(Debug, Clone)]
enum SignedCounterOp {
    Add(i64),
    Get,
}

fn signed_counter_op_strategy() -> impl Strategy<Value = SignedCounterOp> {
    prop_oneof![
        4 => (-500i64..500).prop_map(SignedCounterOp::Add),
        1 => Just(SignedCounterOp::Get),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// SignedCounter: model tracks real across random add operations.
    #[test]
    fn test_signed_counter_model_equivalence(
        ops in prop::collection::vec(signed_counter_op_strategy(), 1..50)
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let store = DeterministicKeyValueStore::new();
            let counter = SignedAtomicCounter::new(
                store.clone(),
                "test_signed_counter",
                CounterConfig::default(),
            );
            let mut model = SignedCounterModel::default();

            for op in &ops {
                match op {
                    SignedCounterOp::Add(n) => {
                        let real = counter.add(*n).await.unwrap();
                        let expected = model.add(*n);
                        assert_eq!(real, expected, "signed add({n}) mismatch");
                    }
                    SignedCounterOp::Get => {
                        let real = counter.get().await.unwrap();
                        assert_eq!(real, model.value, "signed get mismatch");
                    }
                }
            }

            let final_value = counter.get().await.unwrap();
            assert_eq!(final_value, model.value, "signed final value mismatch");
        });
    }
}

// ---------------------------------------------------------------------------
// Sequence model
// ---------------------------------------------------------------------------

/// Reference model for Sequence: monotonically increasing counter.
#[derive(Debug, Clone)]
struct SequenceModel {
    next_value: u64,
}

impl SequenceModel {
    fn new(start: u64) -> Self {
        Self { next_value: start }
    }
    fn next(&mut self) -> u64 {
        let v = self.next_value;
        self.next_value = v.saturating_add(1);
        v
    }
    fn reserve(&mut self, count: u64) -> u64 {
        let start = self.next_value;
        self.next_value = start.saturating_add(count);
        start
    }
    fn current(&self) -> u64 {
        self.next_value
    }
}

#[derive(Debug, Clone)]
enum SequenceOp {
    Next,
    Reserve(u64),
    Current,
}

fn sequence_op_strategy() -> impl Strategy<Value = SequenceOp> {
    prop_oneof![
        5 => Just(SequenceOp::Next),
        2 => (1u64..100).prop_map(SequenceOp::Reserve),
        2 => Just(SequenceOp::Current),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Sequence: every value is unique and monotonically increasing.
    #[test]
    fn test_sequence_model_equivalence(
        ops in prop::collection::vec(sequence_op_strategy(), 1..80)
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let store = DeterministicKeyValueStore::new();
            // batch_size_ids=1 disables batching so the model can track
            // exact state without simulating batch reservation.
            let config = SequenceConfig {
                batch_size_ids: 1,
                start_value: 1,
            };
            let seq = SequenceGenerator::new(store.clone(), "test_seq", config);
            let mut model = SequenceModel::new(1);
            let mut all_ids: Vec<u64> = Vec::new();

            for op in &ops {
                match op {
                    SequenceOp::Next => {
                        let real = seq.next().await.unwrap();
                        let expected = model.next();
                        assert_eq!(real, expected, "next() mismatch");
                        // Track for uniqueness check
                        assert!(
                            !all_ids.contains(&real),
                            "Duplicate ID {real} from next()"
                        );
                        all_ids.push(real);
                    }
                    SequenceOp::Reserve(count) => {
                        let real_start = seq.reserve(*count).await.unwrap();
                        let expected_start = model.reserve(*count);
                        assert_eq!(
                            real_start, expected_start,
                            "reserve({count}) start mismatch"
                        );
                        // All IDs in [start, start+count) must be unique
                        for id in real_start..real_start + count {
                            assert!(
                                !all_ids.contains(&id),
                                "Duplicate ID {id} from reserve({count})"
                            );
                            all_ids.push(id);
                        }
                    }
                    SequenceOp::Current => {
                        let real = seq.current().await.unwrap();
                        let expected = model.current();
                        assert_eq!(real, expected, "current() mismatch");
                    }
                }
            }

            // Monotonicity: all IDs in insertion order are strictly increasing
            // (next() produces sequential, reserve() produces ranges)
            for window in all_ids.windows(2) {
                assert!(
                    window[0] < window[1],
                    "Non-monotonic IDs: {} >= {}",
                    window[0],
                    window[1]
                );
            }
        });
    }

    /// Sequence: multiple independent sequences don't interfere.
    #[test]
    fn test_independent_sequences(
        ops_a in prop::collection::vec(sequence_op_strategy(), 1..30),
        ops_b in prop::collection::vec(sequence_op_strategy(), 1..30),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let store = DeterministicKeyValueStore::new();
            let config = SequenceConfig {
                batch_size_ids: 1,
                start_value: 1,
            };
            let seq_a = SequenceGenerator::new(store.clone(), "seq_a", config.clone());
            let seq_b = SequenceGenerator::new(store.clone(), "seq_b", config);
            let mut model_a = SequenceModel::new(1);
            let mut model_b = SequenceModel::new(1);

            // Interleave operations on both sequences
            let max_len = ops_a.len().max(ops_b.len());
            for i in 0..max_len {
                if let Some(op) = ops_a.get(i) {
                    match op {
                        SequenceOp::Next => {
                            let real = seq_a.next().await.unwrap();
                            let expected = model_a.next();
                            assert_eq!(real, expected, "seq_a next() mismatch at step {i}");
                        }
                        SequenceOp::Reserve(count) => {
                            let real = seq_a.reserve(*count).await.unwrap();
                            let expected = model_a.reserve(*count);
                            assert_eq!(real, expected, "seq_a reserve mismatch at step {i}");
                        }
                        SequenceOp::Current => {
                            let real = seq_a.current().await.unwrap();
                            assert_eq!(real, model_a.current(), "seq_a current mismatch");
                        }
                    }
                }
                if let Some(op) = ops_b.get(i) {
                    match op {
                        SequenceOp::Next => {
                            let real = seq_b.next().await.unwrap();
                            let expected = model_b.next();
                            assert_eq!(real, expected, "seq_b next() mismatch at step {i}");
                        }
                        SequenceOp::Reserve(count) => {
                            let real = seq_b.reserve(*count).await.unwrap();
                            let expected = model_b.reserve(*count);
                            assert_eq!(real, expected, "seq_b reserve mismatch at step {i}");
                        }
                        SequenceOp::Current => {
                            let real = seq_b.current().await.unwrap();
                            assert_eq!(real, model_b.current(), "seq_b current mismatch");
                        }
                    }
                }
            }
        });
    }
}
