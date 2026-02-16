//! Topological processing of Git object DAGs.
//!
//! Git objects form a directed acyclic graph (DAG) where:
//! - Commits point to trees and parent commits
//! - Trees point to blobs and subtrees
//! - Tags point to any object type
//!
//! For import/export, objects must be processed in dependency order:
//! 1. Blobs (no dependencies)
//! 2. Trees (depend on blobs and subtrees)
//! 3. Commits (depend on trees and parent commits)
//! 4. Tags (depend on target objects)
//!
//! This module implements Kahn's algorithm for topological sorting.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use super::constants::MAX_DAG_TRAVERSAL_DEPTH;
use super::constants::MAX_PENDING_OBJECTS;
use super::error::BridgeError;
use super::error::BridgeResult;
use super::mapping::GitObjectType;
use super::sha1::Sha1Hash;

/// An object pending conversion with its dependencies.
#[derive(Debug, Clone)]
pub struct PendingObject {
    /// SHA-1 hash of this object.
    pub sha1: Sha1Hash,
    /// Type of this object.
    pub object_type: GitObjectType,
    /// Raw git object bytes (including header).
    pub data: Vec<u8>,
    /// SHA-1 hashes of objects this object depends on.
    pub dependencies: Vec<Sha1Hash>,
}

impl PendingObject {
    /// Create a new pending object.
    pub fn new(sha1: Sha1Hash, object_type: GitObjectType, data: Vec<u8>, dependencies: Vec<Sha1Hash>) -> Self {
        Self {
            sha1,
            object_type,
            data,
            dependencies,
        }
    }

    /// Create a blob object (no dependencies).
    pub fn blob(sha1: Sha1Hash, data: Vec<u8>) -> Self {
        Self::new(sha1, GitObjectType::Blob, data, Vec::new())
    }
}

/// Result of topological sorting.
#[derive(Debug)]
pub struct TopologicalOrder {
    /// Objects in dependency order (dependencies come before dependents).
    pub objects: Vec<PendingObject>,
    /// Objects that were already processed (had existing mappings).
    pub skipped: HashSet<Sha1Hash>,
}

/// Result of wave-based topological sorting.
///
/// Objects are grouped into waves where all objects in a wave can be
/// processed concurrently because they have no dependencies on each other.
/// Waves must be processed sequentially (wave 0 before wave 1, etc.)
/// because later waves depend on objects in earlier waves.
#[derive(Debug)]
pub struct TopologicalWaves {
    /// Objects grouped by dependency level (wave).
    /// Wave 0 contains objects with no in-set dependencies.
    /// Wave N contains objects whose dependencies are all in waves < N.
    pub waves: Vec<Vec<PendingObject>>,
    /// Objects that were already processed (had existing mappings).
    pub skipped: HashSet<Sha1Hash>,
}

/// Topologically sort a collection of objects.
///
/// Uses Kahn's algorithm to produce an ordering where dependencies
/// come before the objects that depend on them.
pub fn topological_sort(objects: Vec<PendingObject>) -> BridgeResult<TopologicalOrder> {
    // Tiger Style: validate input bounds
    debug_assert!(
        objects.len() <= MAX_PENDING_OBJECTS,
        "TOPO_SORT: objects.len() {} exceeds MAX_PENDING_OBJECTS {}",
        objects.len(),
        MAX_PENDING_OBJECTS
    );

    if objects.len() > MAX_PENDING_OBJECTS {
        return Err(BridgeError::ImportBatchExceeded {
            count: objects.len(),
            max: MAX_PENDING_OBJECTS,
        });
    }

    // First pass: collect all hashes so we know which dependencies are internal
    let all_hashes: HashSet<Sha1Hash> = objects.iter().map(|obj| obj.sha1).collect();
    debug_assert_eq!(all_hashes.len(), objects.len(), "TOPO_SORT: duplicate SHA-1 hashes detected in input");

    // Build adjacency list and in-degree count
    let mut in_degree: HashMap<Sha1Hash, usize> = HashMap::new();
    let mut dependents: HashMap<Sha1Hash, Vec<Sha1Hash>> = HashMap::new();
    let mut object_map: HashMap<Sha1Hash, PendingObject> = HashMap::new();

    for obj in objects {
        in_degree.entry(obj.sha1).or_insert(0);

        for dep in &obj.dependencies {
            if all_hashes.contains(dep) {
                // Only count dependencies that are in our set
                *in_degree.entry(obj.sha1).or_insert(0) += 1;
                dependents.entry(*dep).or_default().push(obj.sha1);
            }
            // External dependencies (already processed) don't contribute to in-degree
        }

        object_map.insert(obj.sha1, obj);
    }

    // Initialize queue with objects that have no in-set dependencies
    let mut queue: VecDeque<Sha1Hash> = in_degree.iter().filter(|&(_, deg)| *deg == 0).map(|(sha1, _)| *sha1).collect();

    let mut result = Vec::with_capacity(object_map.len());
    let mut processed = HashSet::new();

    while let Some(sha1) = queue.pop_front() {
        if processed.contains(&sha1) {
            continue;
        }
        processed.insert(sha1);

        if let Some(obj) = object_map.remove(&sha1) {
            result.push(obj);
        }

        // Decrease in-degree for dependents
        if let Some(deps) = dependents.get(&sha1) {
            for dependent in deps {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 && !processed.contains(dependent) {
                        queue.push_back(*dependent);
                    }
                }
            }
        }
    }

    // Check for cycles - if any objects remain in object_map, we have a cycle
    if !object_map.is_empty() {
        return Err(BridgeError::CycleDetected);
    }

    // Tiger Style: postcondition - all objects processed
    debug_assert!(
        processed.len() == result.len(),
        "TOPO_SORT: processed {} != result {} - internal inconsistency",
        processed.len(),
        result.len()
    );

    Ok(TopologicalOrder {
        objects: result,
        skipped: HashSet::new(),
    })
}

/// Topologically sort objects into parallel waves.
///
/// Uses Kahn's algorithm to produce waves where all objects in a wave
/// can be processed concurrently because they have no dependencies on
/// each other. Waves must be processed sequentially.
///
/// This enables parallel import: instead of importing objects one at a time,
/// we can import all objects in wave 0 concurrently, then wave 1, etc.
pub fn topological_sort_waves(objects: Vec<PendingObject>) -> BridgeResult<TopologicalWaves> {
    // Tiger Style: validate input bounds
    debug_assert!(
        objects.len() <= MAX_PENDING_OBJECTS,
        "TOPO_SORT_WAVES: objects.len() {} exceeds MAX_PENDING_OBJECTS {}",
        objects.len(),
        MAX_PENDING_OBJECTS
    );

    if objects.len() > MAX_PENDING_OBJECTS {
        return Err(BridgeError::ImportBatchExceeded {
            count: objects.len(),
            max: MAX_PENDING_OBJECTS,
        });
    }

    if objects.is_empty() {
        return Ok(TopologicalWaves {
            waves: Vec::new(),
            skipped: HashSet::new(),
        });
    }

    // First pass: collect all hashes so we know which dependencies are internal
    let all_hashes: HashSet<Sha1Hash> = objects.iter().map(|obj| obj.sha1).collect();

    // Build adjacency list and in-degree count
    let mut in_degree: HashMap<Sha1Hash, usize> = HashMap::new();
    let mut dependents: HashMap<Sha1Hash, Vec<Sha1Hash>> = HashMap::new();
    let mut object_map: HashMap<Sha1Hash, PendingObject> = HashMap::new();

    for obj in objects {
        in_degree.entry(obj.sha1).or_insert(0);

        for dep in &obj.dependencies {
            if all_hashes.contains(dep) {
                // Only count dependencies that are in our set
                *in_degree.entry(obj.sha1).or_insert(0) += 1;
                dependents.entry(*dep).or_default().push(obj.sha1);
            }
            // External dependencies (already processed) don't contribute to in-degree
        }

        object_map.insert(obj.sha1, obj);
    }

    // Initialize first wave with objects that have no in-set dependencies
    let mut current_wave: Vec<Sha1Hash> =
        in_degree.iter().filter(|&(_, deg)| *deg == 0).map(|(sha1, _)| *sha1).collect();

    let mut waves: Vec<Vec<PendingObject>> = Vec::new();
    let mut processed = HashSet::new();

    while !current_wave.is_empty() {
        let mut wave_objects = Vec::with_capacity(current_wave.len());
        let mut next_wave = Vec::new();

        for sha1 in current_wave {
            if processed.contains(&sha1) {
                continue;
            }
            processed.insert(sha1);

            if let Some(obj) = object_map.remove(&sha1) {
                wave_objects.push(obj);
            }

            // Decrease in-degree for dependents and collect those ready for next wave
            if let Some(deps) = dependents.get(&sha1) {
                for dependent in deps {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 && !processed.contains(dependent) {
                            next_wave.push(*dependent);
                        }
                    }
                }
            }
        }

        if !wave_objects.is_empty() {
            waves.push(wave_objects);
        }

        current_wave = next_wave;
    }

    // Check for cycles - if any objects remain in object_map, we have a cycle
    if !object_map.is_empty() {
        return Err(BridgeError::CycleDetected);
    }

    // Tiger Style: postcondition - waves are non-empty and ordered
    debug_assert!(!waves.is_empty() || processed.is_empty(), "TOPO_SORT_WAVES: non-empty processed set but no waves");
    debug_assert!(waves.iter().all(|w| !w.is_empty()), "TOPO_SORT_WAVES: empty wave detected");

    Ok(TopologicalWaves {
        waves,
        skipped: HashSet::new(),
    })
}

/// Extract dependencies from git tree content.
///
/// Returns SHA-1 hashes of all entries in the tree, excluding gitlinks (submodules).
/// Gitlinks (mode 160000) reference commits in external repositories and are not
/// dependencies that exist in this repository's object store.
pub fn extract_tree_dependencies(tree_content: &[u8]) -> BridgeResult<Vec<Sha1Hash>> {
    // Tiger Style: precondition - tree content can be empty but not oversized
    debug_assert!(
        tree_content.len() <= 100 * 1024 * 1024,
        "EXTRACT_TREE_DEPS: tree content {} bytes exceeds 100MB limit",
        tree_content.len()
    );

    let mut deps = Vec::new();
    let mut pos = 0;

    while pos < tree_content.len() {
        // Read mode (until space)
        let space_pos =
            tree_content[pos..].iter().position(|&b| b == b' ').ok_or_else(|| BridgeError::MalformedTreeEntry {
                message: "missing space after mode".to_string(),
            })?;
        let mode = &tree_content[pos..pos + space_pos];
        pos += space_pos + 1;

        // Skip name (until NUL)
        let nul_pos =
            tree_content[pos..].iter().position(|&b| b == 0).ok_or_else(|| BridgeError::MalformedTreeEntry {
                message: "missing NUL after name".to_string(),
            })?;
        pos += nul_pos + 1;

        // Read SHA-1 (20 bytes)
        if pos + 20 > tree_content.len() {
            return Err(BridgeError::MalformedTreeEntry {
                message: "truncated SHA-1 hash".to_string(),
            });
        }
        let sha1 = Sha1Hash::from_slice(&tree_content[pos..pos + 20])?;
        pos += 20;

        // Skip gitlinks (mode 160000) - they reference commits in external repositories
        // Mode is stored as ASCII octal, so "160000" = [0x31, 0x36, 0x30, 0x30, 0x30, 0x30]
        if mode == b"160000" {
            continue;
        }

        deps.push(sha1);
    }

    Ok(deps)
}

/// Extract dependencies from git commit content.
///
/// Returns SHA-1 hashes of tree and parent commits.
pub fn extract_commit_dependencies(commit_content: &str) -> BridgeResult<Vec<Sha1Hash>> {
    // Tiger Style: precondition - commit content should not be empty
    debug_assert!(!commit_content.is_empty(), "EXTRACT_COMMIT_DEPS: empty commit content");

    let mut deps = Vec::new();

    for line in commit_content.lines() {
        if let Some(tree_hex) = line.strip_prefix("tree ") {
            deps.push(Sha1Hash::from_hex(tree_hex)?);
        } else if let Some(parent_hex) = line.strip_prefix("parent ") {
            deps.push(Sha1Hash::from_hex(parent_hex)?);
        } else if line.starts_with("author ") {
            // Author line marks end of references
            break;
        }
    }

    Ok(deps)
}

/// Extract dependencies from git tag content.
///
/// Returns SHA-1 hash of target object.
pub fn extract_tag_dependencies(tag_content: &str) -> BridgeResult<Vec<Sha1Hash>> {
    // Tiger Style: precondition - tag content should not be empty
    debug_assert!(!tag_content.is_empty(), "EXTRACT_TAG_DEPS: empty tag content");

    for line in tag_content.lines() {
        if let Some(object_hex) = line.strip_prefix("object ") {
            return Ok(vec![Sha1Hash::from_hex(object_hex)?]);
        }
    }

    Err(BridgeError::MalformedObject {
        message: "tag missing object line".to_string(),
    })
}

/// Collector for building object sets for topological sorting.
pub struct ObjectCollector {
    /// Objects collected so far.
    objects: Vec<PendingObject>,
    /// Hashes we've seen (to avoid duplicates).
    seen: HashSet<Sha1Hash>,
    /// Hashes that already have mappings (external dependencies).
    existing: HashSet<Sha1Hash>,
    /// Current traversal depth (for cycle detection).
    depth: usize,
}

impl ObjectCollector {
    /// Create a new object collector.
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            seen: HashSet::new(),
            existing: HashSet::new(),
            depth: 0,
        }
    }

    /// Check if we've seen this hash.
    pub fn has_seen(&self, sha1: &Sha1Hash) -> bool {
        self.seen.contains(sha1) || self.existing.contains(sha1)
    }

    /// Mark a hash as already having a mapping.
    pub fn mark_existing(&mut self, sha1: Sha1Hash) {
        self.existing.insert(sha1);
    }

    /// Add an object to the collection.
    pub fn add(&mut self, obj: PendingObject) -> BridgeResult<()> {
        if self.objects.len() >= MAX_PENDING_OBJECTS {
            return Err(BridgeError::ImportBatchExceeded {
                count: self.objects.len() + 1,
                max: MAX_PENDING_OBJECTS,
            });
        }

        self.seen.insert(obj.sha1);
        self.objects.push(obj);
        Ok(())
    }

    /// Increment depth for recursive traversal.
    pub fn enter(&mut self) -> BridgeResult<()> {
        self.depth += 1;
        if self.depth > MAX_DAG_TRAVERSAL_DEPTH {
            return Err(BridgeError::DepthExceeded {
                depth: self.depth,
                max: MAX_DAG_TRAVERSAL_DEPTH,
            });
        }
        Ok(())
    }

    /// Decrement depth after recursive traversal.
    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Get collected objects count.
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Finish collection and return sorted objects.
    pub fn finish(self) -> BridgeResult<TopologicalOrder> {
        let mut order = topological_sort(self.objects)?;
        order.skipped = self.existing;
        Ok(order)
    }

    /// Finish collection and return objects grouped into parallel waves.
    ///
    /// Objects in the same wave can be processed concurrently.
    /// Waves must be processed sequentially.
    pub fn finish_waves(self) -> BridgeResult<TopologicalWaves> {
        let mut waves = topological_sort_waves(self.objects)?;
        waves.skipped = self.existing;
        Ok(waves)
    }
}

impl Default for ObjectCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_simple() {
        let blob = PendingObject::blob(Sha1Hash::from_bytes([1; 20]), vec![]);

        let tree = PendingObject::new(
            Sha1Hash::from_bytes([2; 20]),
            GitObjectType::Tree,
            vec![],
            vec![Sha1Hash::from_bytes([1; 20])], // depends on blob
        );

        let commit = PendingObject::new(
            Sha1Hash::from_bytes([3; 20]),
            GitObjectType::Commit,
            vec![],
            vec![Sha1Hash::from_bytes([2; 20])], // depends on tree
        );

        let objects = vec![commit, tree, blob]; // out of order

        let result = topological_sort(objects).unwrap();

        // Blob should come first, then tree, then commit
        assert_eq!(result.objects.len(), 3);
        assert_eq!(result.objects[0].object_type, GitObjectType::Blob);
        assert_eq!(result.objects[1].object_type, GitObjectType::Tree);
        assert_eq!(result.objects[2].object_type, GitObjectType::Commit);
    }

    #[test]
    fn test_topological_sort_diamond() {
        // Diamond dependency: commit -> tree1, tree2 -> blob
        let blob = PendingObject::blob(Sha1Hash::from_bytes([1; 20]), vec![]);

        let tree1 =
            PendingObject::new(Sha1Hash::from_bytes([2; 20]), GitObjectType::Tree, vec![], vec![Sha1Hash::from_bytes(
                [1; 20],
            )]);

        let tree2 =
            PendingObject::new(Sha1Hash::from_bytes([3; 20]), GitObjectType::Tree, vec![], vec![Sha1Hash::from_bytes(
                [1; 20],
            )]);

        let commit = PendingObject::new(Sha1Hash::from_bytes([4; 20]), GitObjectType::Commit, vec![], vec![
            Sha1Hash::from_bytes([2; 20]),
            Sha1Hash::from_bytes([3; 20]),
        ]);

        let objects = vec![commit, tree2, blob, tree1];

        let result = topological_sort(objects).unwrap();

        // Blob should come first
        assert_eq!(result.objects[0].object_type, GitObjectType::Blob);

        // Commit should come last
        assert_eq!(result.objects.last().unwrap().object_type, GitObjectType::Commit);
    }

    #[test]
    fn test_extract_commit_dependencies() {
        // SHA-1 hashes must be exactly 40 hex characters
        let tree_hash = "a".repeat(40);
        let parent1_hash = "b".repeat(40);
        let parent2_hash = "c".repeat(40);

        let commit_content = format!(
            "tree {}\n\
             parent {}\n\
             parent {}\n\
             author Test <test@example.com> 1234567890 +0000\n\
             committer Test <test@example.com> 1234567890 +0000\n\
             \n\
             Commit message",
            tree_hash, parent1_hash, parent2_hash
        );

        let deps = extract_commit_dependencies(&commit_content).unwrap();
        assert_eq!(deps.len(), 3); // 1 tree + 2 parents
    }

    #[test]
    fn test_object_collector_depth_limit() {
        let mut collector = ObjectCollector::new();

        // Should succeed up to max depth
        for _ in 0..MAX_DAG_TRAVERSAL_DEPTH {
            collector.enter().unwrap();
        }

        // Should fail on exceeding max depth
        assert!(collector.enter().is_err());
    }

    #[test]
    fn test_topological_sort_waves_simple_chain() {
        // Chain: blob -> tree -> commit
        let blob = PendingObject::blob(Sha1Hash::from_bytes([1; 20]), vec![]);

        let tree =
            PendingObject::new(Sha1Hash::from_bytes([2; 20]), GitObjectType::Tree, vec![], vec![Sha1Hash::from_bytes(
                [1; 20],
            )]);

        let commit = PendingObject::new(Sha1Hash::from_bytes([3; 20]), GitObjectType::Commit, vec![], vec![
            Sha1Hash::from_bytes([2; 20]),
        ]);

        let objects = vec![commit, tree, blob];
        let result = topological_sort_waves(objects).unwrap();

        // Should have 3 waves for a chain
        assert_eq!(result.waves.len(), 3);
        assert_eq!(result.waves[0].len(), 1); // blob
        assert_eq!(result.waves[1].len(), 1); // tree
        assert_eq!(result.waves[2].len(), 1); // commit
        assert_eq!(result.waves[0][0].object_type, GitObjectType::Blob);
        assert_eq!(result.waves[1][0].object_type, GitObjectType::Tree);
        assert_eq!(result.waves[2][0].object_type, GitObjectType::Commit);
    }

    #[test]
    fn test_topological_sort_waves_diamond() {
        // Diamond: commit -> (tree1, tree2) -> blob
        // Wave 0: blob
        // Wave 1: tree1, tree2 (parallel)
        // Wave 2: commit
        let blob = PendingObject::blob(Sha1Hash::from_bytes([1; 20]), vec![]);

        let tree1 =
            PendingObject::new(Sha1Hash::from_bytes([2; 20]), GitObjectType::Tree, vec![], vec![Sha1Hash::from_bytes(
                [1; 20],
            )]);

        let tree2 =
            PendingObject::new(Sha1Hash::from_bytes([3; 20]), GitObjectType::Tree, vec![], vec![Sha1Hash::from_bytes(
                [1; 20],
            )]);

        let commit = PendingObject::new(Sha1Hash::from_bytes([4; 20]), GitObjectType::Commit, vec![], vec![
            Sha1Hash::from_bytes([2; 20]),
            Sha1Hash::from_bytes([3; 20]),
        ]);

        let objects = vec![commit, tree2, blob, tree1];
        let result = topological_sort_waves(objects).unwrap();

        // Should have 3 waves
        assert_eq!(result.waves.len(), 3);
        assert_eq!(result.waves[0].len(), 1); // blob
        assert_eq!(result.waves[1].len(), 2); // tree1 and tree2 in parallel
        assert_eq!(result.waves[2].len(), 1); // commit
    }

    #[test]
    fn test_topological_sort_waves_wide_parallel() {
        // Wide: commit -> 5 blobs (all in parallel)
        // Wave 0: all 5 blobs
        // Wave 1: commit
        let blobs: Vec<PendingObject> =
            (1..=5).map(|i| PendingObject::blob(Sha1Hash::from_bytes([i; 20]), vec![])).collect();

        let blob_deps: Vec<Sha1Hash> = (1..=5).map(|i| Sha1Hash::from_bytes([i; 20])).collect();

        let commit = PendingObject::new(Sha1Hash::from_bytes([10; 20]), GitObjectType::Commit, vec![], blob_deps);

        let mut objects = blobs;
        objects.push(commit);

        let result = topological_sort_waves(objects).unwrap();

        assert_eq!(result.waves.len(), 2);
        assert_eq!(result.waves[0].len(), 5); // all blobs in parallel
        assert_eq!(result.waves[1].len(), 1); // commit
    }

    #[test]
    fn test_topological_sort_waves_empty() {
        let result = topological_sort_waves(vec![]).unwrap();
        assert!(result.waves.is_empty());
    }

    #[test]
    fn test_topological_sort_waves_single_object() {
        let blob = PendingObject::blob(Sha1Hash::from_bytes([1; 20]), vec![]);
        let result = topological_sort_waves(vec![blob]).unwrap();

        assert_eq!(result.waves.len(), 1);
        assert_eq!(result.waves[0].len(), 1);
    }

    #[test]
    fn test_topological_sort_waves_all_independent() {
        // All blobs with no dependencies - should be single wave
        let blobs: Vec<PendingObject> =
            (1..=10).map(|i| PendingObject::blob(Sha1Hash::from_bytes([i; 20]), vec![])).collect();

        let result = topological_sort_waves(blobs).unwrap();

        assert_eq!(result.waves.len(), 1);
        assert_eq!(result.waves[0].len(), 10); // all in parallel
    }

    #[test]
    fn test_object_collector_finish_waves() {
        let mut collector = ObjectCollector::new();

        let blob = PendingObject::blob(Sha1Hash::from_bytes([1; 20]), vec![]);
        let tree =
            PendingObject::new(Sha1Hash::from_bytes([2; 20]), GitObjectType::Tree, vec![], vec![Sha1Hash::from_bytes(
                [1; 20],
            )]);

        collector.add(blob).unwrap();
        collector.add(tree).unwrap();
        collector.mark_existing(Sha1Hash::from_bytes([99; 20]));

        let waves = collector.finish_waves().unwrap();

        assert_eq!(waves.waves.len(), 2);
        assert!(waves.skipped.contains(&Sha1Hash::from_bytes([99; 20])));
    }
}
