//! BUGGIFY-style fault injection (inspired by FoundationDB).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use serde::Serialize;

/// Type of fault that can be injected via BUGGIFY.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum BuggifyFault {
    /// Inject random network delays.
    NetworkDelay,
    /// Drop random network packets.
    NetworkDrop,
    /// Crash random nodes.
    NodeCrash,
    /// Inject slow disk operations.
    SlowDisk,
    /// Corrupt random messages.
    MessageCorruption,
    /// Force election timeouts.
    ElectionTimeout,
    /// Partition the network randomly.
    NetworkPartition,
    /// Trigger snapshot operations.
    SnapshotTrigger,
}

/// BUGGIFY configuration for systematic fault injection.
///
/// Inspired by FoundationDB's BUGGIFY macro, this provides
/// deterministic fault injection based on the test seed.
#[derive(Clone)]
pub struct BuggifyConfig {
    /// Whether BUGGIFY is enabled globally.
    pub(crate) enabled: Arc<AtomicBool>,
    /// Base probability (0.0 to 1.0) for each fault type.
    pub(crate) probabilities: Arc<Mutex<HashMap<BuggifyFault, f64>>>,
    /// Count of triggers per fault type.
    trigger_counts: Arc<Mutex<HashMap<BuggifyFault, u64>>>,
    /// Seed used to initialize this BUGGIFY instance.
    /// Note: Probability checking now uses madsim's RNG for uniform distribution,
    /// but the seed is retained for debugging and metrics purposes.
    #[allow(dead_code)]
    seed: u64,
}

impl BuggifyConfig {
    /// Create a new BUGGIFY configuration with default settings.
    pub fn new(seed: u64) -> Self {
        let mut default_probs = HashMap::new();
        // Default probabilities for each fault type
        default_probs.insert(BuggifyFault::NetworkDelay, 0.05); // 5%
        default_probs.insert(BuggifyFault::NetworkDrop, 0.02); // 2%
        default_probs.insert(BuggifyFault::NodeCrash, 0.01); // 1%
        default_probs.insert(BuggifyFault::SlowDisk, 0.05); // 5%
        default_probs.insert(BuggifyFault::MessageCorruption, 0.01); // 1%
        default_probs.insert(BuggifyFault::ElectionTimeout, 0.02); // 2%
        default_probs.insert(BuggifyFault::NetworkPartition, 0.005); // 0.5%
        default_probs.insert(BuggifyFault::SnapshotTrigger, 0.02); // 2%

        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            probabilities: Arc::new(Mutex::new(default_probs)),
            trigger_counts: Arc::new(Mutex::new(HashMap::new())),
            seed,
        }
    }

    /// Enable BUGGIFY with optional custom probabilities.
    pub fn enable(&self, custom_probs: Option<HashMap<BuggifyFault, f64>>) {
        if let Some(probs) = custom_probs {
            *self.probabilities.lock().unwrap() = probs;
        }
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable BUGGIFY.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if a specific fault should be triggered.
    ///
    /// Uses madsim's deterministic RNG for uniform probability distribution.
    /// The simulation framework seeds the RNG, ensuring reproducibility.
    pub fn should_trigger(&self, fault: BuggifyFault) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        let mut counts = self.trigger_counts.lock().unwrap();
        let count = counts.entry(fault).or_insert(0);
        *count += 1;

        let probs = self.probabilities.lock().unwrap();
        let probability = probs.get(&fault).copied().unwrap_or(0.0);

        // Use madsim's deterministic RNG for uniform distribution.
        // This provides proper probability checking unlike DefaultHasher,
        // which has poor distribution for probability thresholding.
        let random_value: f64 = (madsim::rand::random::<u64>() as f64) / (u64::MAX as f64);
        random_value < probability
    }

    /// Get total trigger counts for metrics.
    pub fn total_triggers(&self) -> u64 {
        self.trigger_counts.lock().unwrap().values().sum()
    }
}
