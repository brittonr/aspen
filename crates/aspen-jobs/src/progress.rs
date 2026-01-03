//! CRDT-based job progress tracking for distributed updates.
//!
//! This module provides eventually-consistent progress tracking using CRDTs,
//! allowing multiple workers to update job progress concurrently without conflicts.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::debug;

use crate::error::Result;
use crate::job::JobId;

/// CRDT-based progress tracker for jobs.
pub struct CrdtProgressTracker {
    /// Progress state for each job.
    states: Arc<RwLock<HashMap<JobId, ProgressCrdt>>>,
    /// Node ID for vector clock.
    node_id: String,
    /// Synchronization interval.
    sync_interval_ms: u64,
}

impl CrdtProgressTracker {
    /// Create a new CRDT progress tracker.
    pub fn new(node_id: String) -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            node_id,
            sync_interval_ms: 100,
        }
    }

    /// Update job progress.
    pub async fn update_progress(&self, job_id: &JobId, update: ProgressUpdate) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states.entry(job_id.clone()).or_insert_with(|| ProgressCrdt::new(job_id.clone()));

        state.apply_update(update, &self.node_id);

        debug!(
            job_id = %job_id,
            node_id = %self.node_id,
            "updated job progress"
        );

        Ok(())
    }

    /// Get current progress for a job.
    pub async fn get_progress(&self, job_id: &JobId) -> Option<JobProgress> {
        let states = self.states.read().await;
        states.get(job_id).map(|crdt| crdt.current_progress())
    }

    /// Merge remote CRDT state.
    pub async fn merge_remote(&self, remote_state: ProgressCrdt) -> Result<()> {
        let mut states = self.states.write().await;

        let local = states
            .entry(remote_state.job_id.clone())
            .or_insert_with(|| ProgressCrdt::new(remote_state.job_id.clone()));

        local.merge(remote_state);

        Ok(())
    }

    /// Get all CRDT states for synchronization.
    pub async fn get_all_states(&self) -> Vec<ProgressCrdt> {
        let states = self.states.read().await;
        states.values().cloned().collect()
    }

    /// Batch update multiple jobs.
    pub async fn batch_update(&self, updates: Vec<(JobId, ProgressUpdate)>) -> Result<()> {
        let mut states = self.states.write().await;

        for (job_id, update) in updates {
            let state = states.entry(job_id.clone()).or_insert_with(|| ProgressCrdt::new(job_id));

            state.apply_update(update, &self.node_id);
        }

        Ok(())
    }

    /// Subscribe to progress changes.
    pub async fn subscribe(&self, job_id: &JobId) -> ProgressSubscription {
        ProgressSubscription {
            job_id: job_id.clone(),
            tracker: self.states.clone(),
        }
    }

    /// Garbage collect old progress entries.
    pub async fn gc(&self, max_age_ms: u64) -> usize {
        let mut states = self.states.write().await;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

        let before = states.len();
        states.retain(|_, crdt| now - crdt.last_update_ms < max_age_ms);
        let after = states.len();

        before - after
    }
}

/// CRDT state for a single job's progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressCrdt {
    /// Job ID.
    pub job_id: JobId,
    /// Vector clock for ordering updates.
    pub vector_clock: VectorClock,
    /// Progress percentage (0-100).
    pub percentage: MaxCounter,
    /// Current step in workflow.
    pub current_step: LwwRegister<String>,
    /// Steps completed.
    pub completed_steps: GrowOnlySet<String>,
    /// Error count.
    pub error_count: GCounter,
    /// Warning count.
    pub warning_count: GCounter,
    /// Metrics from workers.
    pub metrics: ObservedRemoveMap<String, f64>,
    /// Last update timestamp.
    pub last_update_ms: u64,
}

impl ProgressCrdt {
    /// Create a new progress CRDT.
    pub fn new(job_id: JobId) -> Self {
        Self {
            job_id,
            vector_clock: VectorClock::new(),
            percentage: MaxCounter::new(),
            current_step: LwwRegister::new("pending".to_string()),
            completed_steps: GrowOnlySet::new(),
            error_count: GCounter::new(),
            warning_count: GCounter::new(),
            metrics: ObservedRemoveMap::new(),
            last_update_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        }
    }

    /// Apply an update to the CRDT.
    pub fn apply_update(&mut self, update: ProgressUpdate, node_id: &str) {
        self.vector_clock.increment(node_id);

        match update {
            ProgressUpdate::SetPercentage(pct) => {
                self.percentage.set(pct.min(100));
            }
            ProgressUpdate::SetStep(step) => {
                self.current_step.set(step.clone(), self.last_update_ms);
                self.completed_steps.add(step);
            }
            ProgressUpdate::CompleteStep(step) => {
                self.completed_steps.add(step);
            }
            ProgressUpdate::IncrementErrors(count) => {
                self.error_count.increment(node_id, count);
            }
            ProgressUpdate::IncrementWarnings(count) => {
                self.warning_count.increment(node_id, count);
            }
            ProgressUpdate::SetMetric { key, value } => {
                self.metrics.set(key, value, node_id);
            }
            ProgressUpdate::Batch(updates) => {
                for u in updates {
                    self.apply_update(u, node_id);
                }
            }
        }

        self.last_update_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    }

    /// Merge with another CRDT state.
    pub fn merge(&mut self, other: ProgressCrdt) {
        self.vector_clock.merge(&other.vector_clock);
        self.percentage.merge(&other.percentage);
        self.current_step.merge(&other.current_step);
        self.completed_steps.merge(&other.completed_steps);
        self.error_count.merge(&other.error_count);
        self.warning_count.merge(&other.warning_count);
        self.metrics.merge(&other.metrics);
        self.last_update_ms = self.last_update_ms.max(other.last_update_ms);
    }

    /// Get current progress as a snapshot.
    pub fn current_progress(&self) -> JobProgress {
        JobProgress {
            job_id: self.job_id.clone(),
            percentage: self.percentage.value(),
            current_step: self.current_step.value(),
            completed_steps: self.completed_steps.values(),
            error_count: self.error_count.value(),
            warning_count: self.warning_count.value(),
            metrics: self.metrics.entries(),
            last_update_ms: self.last_update_ms,
        }
    }
}

/// Progress update operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressUpdate {
    /// Set progress percentage.
    SetPercentage(u8),
    /// Set current step.
    SetStep(String),
    /// Mark step as complete.
    CompleteStep(String),
    /// Increment error count.
    IncrementErrors(u64),
    /// Increment warning count.
    IncrementWarnings(u64),
    /// Set a metric value.
    SetMetric { key: String, value: f64 },
    /// Batch of updates.
    Batch(Vec<ProgressUpdate>),
}

/// Job progress snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    /// Job ID.
    pub job_id: JobId,
    /// Progress percentage (0-100).
    pub percentage: u8,
    /// Current step.
    pub current_step: String,
    /// Completed steps.
    pub completed_steps: Vec<String>,
    /// Error count.
    pub error_count: u64,
    /// Warning count.
    pub warning_count: u64,
    /// Metrics.
    pub metrics: HashMap<String, f64>,
    /// Last update timestamp.
    pub last_update_ms: u64,
}

/// Subscription to progress updates.
pub struct ProgressSubscription {
    job_id: JobId,
    tracker: Arc<RwLock<HashMap<JobId, ProgressCrdt>>>,
}

impl ProgressSubscription {
    /// Poll for updates.
    pub async fn poll(&self) -> Option<JobProgress> {
        let states = self.tracker.read().await;
        states.get(&self.job_id).map(|crdt| crdt.current_progress())
    }
}

/// Vector clock for ordering events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorClock {
    clocks: BTreeMap<String, u64>,
}

impl VectorClock {
    fn new() -> Self {
        Self {
            clocks: BTreeMap::new(),
        }
    }

    fn increment(&mut self, node_id: &str) {
        *self.clocks.entry(node_id.to_string()).or_insert(0) += 1;
    }

    fn merge(&mut self, other: &VectorClock) {
        for (node, &clock) in &other.clocks {
            let entry = self.clocks.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(clock);
        }
    }
}

/// Max counter CRDT (keeps highest value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxCounter {
    value: u8,
}

impl MaxCounter {
    fn new() -> Self {
        Self { value: 0 }
    }

    fn set(&mut self, value: u8) {
        self.value = self.value.max(value);
    }

    fn merge(&mut self, other: &MaxCounter) {
        self.value = self.value.max(other.value);
    }

    fn value(&self) -> u8 {
        self.value
    }
}

/// Last-write-wins register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwRegister<T> {
    value: T,
    timestamp: u64,
}

impl<T: Clone> LwwRegister<T> {
    fn new(value: T) -> Self {
        Self { value, timestamp: 0 }
    }

    fn set(&mut self, value: T, timestamp: u64) {
        if timestamp > self.timestamp {
            self.value = value;
            self.timestamp = timestamp;
        }
    }

    fn merge(&mut self, other: &LwwRegister<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        }
    }

    fn value(&self) -> T {
        self.value.clone()
    }
}

/// Grow-only set CRDT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowOnlySet<T> {
    elements: Vec<T>,
}

impl<T: Clone + Eq> GrowOnlySet<T> {
    fn new() -> Self {
        Self { elements: Vec::new() }
    }

    fn add(&mut self, element: T) {
        if !self.elements.contains(&element) {
            self.elements.push(element);
        }
    }

    fn merge(&mut self, other: &GrowOnlySet<T>) {
        for element in &other.elements {
            self.add(element.clone());
        }
    }

    fn values(&self) -> Vec<T> {
        self.elements.clone()
    }
}

/// G-Counter (grow-only counter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCounter {
    counts: HashMap<String, u64>,
}

impl GCounter {
    fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    fn increment(&mut self, node_id: &str, delta: u64) {
        *self.counts.entry(node_id.to_string()).or_insert(0) += delta;
    }

    fn merge(&mut self, other: &GCounter) {
        for (node, &count) in &other.counts {
            let entry = self.counts.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(count);
        }
    }

    fn value(&self) -> u64 {
        self.counts.values().sum()
    }
}

/// Observed-remove map CRDT.
#[derive(Debug, Clone)]
pub struct ObservedRemoveMap<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    entries: HashMap<K, (V, String, u64)>, // value, node_id, version
    version_counter: u64,
}

impl<K, V> Serialize for ObservedRemoveMap<K, V>
where
    K: Clone + Eq + std::hash::Hash + Serialize,
    V: Clone + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ObservedRemoveMap", 2)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("version_counter", &self.version_counter)?;
        state.end()
    }
}

impl<'de, K, V> Deserialize<'de> for ObservedRemoveMap<K, V>
where
    K: Clone + Eq + std::hash::Hash + Deserialize<'de>,
    V: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper<K, V>
        where
            K: Eq + std::hash::Hash,
        {
            entries: HashMap<K, (V, String, u64)>,
            version_counter: u64,
        }

        let helper = Helper::<K, V>::deserialize(deserializer)?;
        Ok(ObservedRemoveMap {
            entries: helper.entries,
            version_counter: helper.version_counter,
        })
    }
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> ObservedRemoveMap<K, V> {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            version_counter: 0,
        }
    }

    fn set(&mut self, key: K, value: V, node_id: &str) {
        self.version_counter += 1;
        self.entries.insert(key, (value, node_id.to_string(), self.version_counter));
    }

    fn merge(&mut self, other: &ObservedRemoveMap<K, V>) {
        for (key, (value, node_id, version)) in &other.entries {
            match self.entries.get(key) {
                Some((_, _, local_version)) if local_version >= version => {
                    // Keep local version
                }
                _ => {
                    self.entries.insert(key.clone(), (value.clone(), node_id.clone(), *version));
                    self.version_counter = self.version_counter.max(*version);
                }
            }
        }
    }

    fn entries(&self) -> HashMap<K, V> {
        self.entries.iter().map(|(k, (v, _, _))| (k.clone(), v.clone())).collect()
    }
}

/// Progress synchronization manager.
pub struct ProgressSyncManager {
    tracker: Arc<CrdtProgressTracker>,
    peers: Arc<RwLock<Vec<String>>>,
}

impl ProgressSyncManager {
    /// Create a new sync manager.
    pub fn new(tracker: Arc<CrdtProgressTracker>) -> Self {
        Self {
            tracker,
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a peer for synchronization.
    pub async fn add_peer(&self, peer_id: String) {
        let mut peers = self.peers.write().await;
        if !peers.contains(&peer_id) {
            peers.push(peer_id);
        }
    }

    /// Remove a peer from synchronization.
    pub async fn remove_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        peers.retain(|p| p != peer_id);
    }

    /// Sync with all peers.
    pub async fn sync_all(&self) -> Result<()> {
        let _states = self.tracker.get_all_states().await;
        let peers = self.peers.read().await.clone();

        for peer in peers {
            debug!("syncing progress with peer: {}", peer);
            // In production, this would send states to peer via Iroh
        }

        Ok(())
    }

    /// Handle incoming sync from peer.
    pub async fn handle_sync(&self, remote_states: Vec<ProgressCrdt>) -> Result<()> {
        for state in remote_states {
            self.tracker.merge_remote(state).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_updates() {
        let tracker = CrdtProgressTracker::new("node1".to_string());
        let job_id = JobId::new();

        // Update percentage
        tracker.update_progress(&job_id, ProgressUpdate::SetPercentage(50)).await.unwrap();

        // Update step
        tracker.update_progress(&job_id, ProgressUpdate::SetStep("processing".to_string())).await.unwrap();

        // Get progress
        let progress = tracker.get_progress(&job_id).await.unwrap();
        assert_eq!(progress.percentage, 50);
        assert_eq!(progress.current_step, "processing");
    }

    #[tokio::test]
    async fn test_crdt_merge() {
        let tracker1 = CrdtProgressTracker::new("node1".to_string());
        let tracker2 = CrdtProgressTracker::new("node2".to_string());
        let job_id = JobId::new();

        // Node 1 updates
        tracker1.update_progress(&job_id, ProgressUpdate::SetPercentage(30)).await.unwrap();
        tracker1.update_progress(&job_id, ProgressUpdate::IncrementErrors(1)).await.unwrap();

        // Node 2 updates
        tracker2.update_progress(&job_id, ProgressUpdate::SetPercentage(60)).await.unwrap();
        tracker2.update_progress(&job_id, ProgressUpdate::IncrementWarnings(2)).await.unwrap();

        // Get states and merge
        let state1 = tracker1.get_all_states().await.into_iter().next().unwrap();
        let state2 = tracker2.get_all_states().await.into_iter().next().unwrap();

        tracker1.merge_remote(state2).await.unwrap();
        tracker2.merge_remote(state1).await.unwrap();

        // Both should have same view
        let progress1 = tracker1.get_progress(&job_id).await.unwrap();
        let progress2 = tracker2.get_progress(&job_id).await.unwrap();

        assert_eq!(progress1.percentage, 60); // Max of 30 and 60
        assert_eq!(progress2.percentage, 60);
        assert_eq!(progress1.error_count, 1);
        assert_eq!(progress2.error_count, 1);
        assert_eq!(progress1.warning_count, 2);
        assert_eq!(progress2.warning_count, 2);
    }

    #[tokio::test]
    async fn test_batch_updates() {
        let tracker = CrdtProgressTracker::new("node1".to_string());
        let job_id = JobId::new();

        let updates = vec![
            (job_id.clone(), ProgressUpdate::SetPercentage(25)),
            (job_id.clone(), ProgressUpdate::SetStep("step1".to_string())),
            (job_id.clone(), ProgressUpdate::CompleteStep("step1".to_string())),
        ];

        tracker.batch_update(updates).await.unwrap();

        let progress = tracker.get_progress(&job_id).await.unwrap();
        assert_eq!(progress.percentage, 25);
        assert_eq!(progress.current_step, "step1");
        assert!(progress.completed_steps.contains(&"step1".to_string()));
    }

    #[tokio::test]
    async fn test_garbage_collection() {
        let tracker = CrdtProgressTracker::new("node1".to_string());

        // Add some jobs
        for i in 0..5 {
            let job_id = JobId::new();
            tracker.update_progress(&job_id, ProgressUpdate::SetPercentage(i * 10)).await.unwrap();
        }

        // GC with very short max age
        let collected = tracker.gc(1).await; // 1ms max age

        // Should collect most/all entries
        assert!(collected >= 4);
    }
}
