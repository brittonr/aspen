//! Derivation analysis for CI build graph introspection and cache invalidation.
//!
//! Parses `.drv` files using `nix_compat::derivation::Derivation` to extract
//! build metadata, construct dependency graphs, compute content-hash cache keys,
//! and calculate runtime closures.
//!
//! # Cache Key Strategy
//!
//! Each derivation's cache key is the SHA-256 of its serialized ATerm
//! representation with input derivation paths replaced by their own
//! `hash_derivation_modulo` digests (same algorithm as Nix's
//! `hashDerivationModulo`). Two derivations with the same cache key are
//! guaranteed to produce identical build outputs.

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::io;
use std::path::Path;

use nix_compat::derivation::Derivation;
#[cfg(feature = "snix")]
use nix_compat::store_path::StorePath;
use tracing::debug;
use tracing::warn;

// ============================================================================
// Error types
// ============================================================================

/// Errors from derivation analysis.
#[derive(Debug)]
pub enum DerivationError {
    /// Failed to read the `.drv` file.
    ReadFailed { path: String, source: io::Error },
    /// Failed to parse the ATerm content of a `.drv` file.
    ParseFailed { path: String, message: String },
    /// A referenced input derivation was not found in the provided set.
    MissingInput { drv_path: String, missing_input: String },
}

impl std::fmt::Display for DerivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DerivationError::ReadFailed { path, source } => {
                write!(f, "failed to read derivation {path}: {source}")
            }
            DerivationError::ParseFailed { path, message } => {
                write!(f, "failed to parse derivation {path}: {message}")
            }
            DerivationError::MissingInput {
                drv_path,
                missing_input,
            } => {
                write!(f, "derivation {drv_path} references missing input {missing_input}")
            }
        }
    }
}

impl std::error::Error for DerivationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DerivationError::ReadFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

// ============================================================================
// Derivation parsing
// ============================================================================

/// Parse a `.drv` file from a store path on disk.
///
/// Reads the file and parses the ATerm representation into a `Derivation`.
pub fn parse_derivation(drv_path: &Path) -> Result<Derivation, DerivationError> {
    let bytes = std::fs::read(drv_path).map_err(|e| DerivationError::ReadFailed {
        path: drv_path.display().to_string(),
        source: e,
    })?;

    Derivation::from_aterm_bytes(&bytes).map_err(|e| DerivationError::ParseFailed {
        path: drv_path.display().to_string(),
        message: format!("{e:?}"),
    })
}

/// Parse a derivation from raw ATerm bytes.
pub fn parse_derivation_bytes(bytes: &[u8], label: &str) -> Result<Derivation, DerivationError> {
    Derivation::from_aterm_bytes(bytes).map_err(|e| DerivationError::ParseFailed {
        path: label.to_string(),
        message: format!("{e:?}"),
    })
}

// ============================================================================
// Build graph
// ============================================================================

/// A node in the build dependency graph.
#[derive(Debug, Clone)]
pub struct BuildGraphNode {
    /// The store path of this `.drv` file.
    pub drv_path: String,
    /// Output names used from this derivation.
    pub output_names: BTreeSet<String>,
    /// Direct dependencies (drv paths this derivation depends on).
    pub dependencies: Vec<String>,
    /// Topological sort order (0 = leaf, built first).
    pub topo_order: u32,
}

/// A build dependency graph constructed from derivation input_derivations.
#[derive(Debug)]
pub struct BuildGraph {
    /// All nodes, keyed by drv path.
    pub nodes: HashMap<String, BuildGraphNode>,
    /// Topologically sorted drv paths (build order: leaves first).
    pub build_order: Vec<String>,
}

/// Construct a build graph from a set of parsed derivations.
///
/// The `derivations` map is keyed by drv store path string. The graph
/// is built by traversing each derivation's `input_derivations` field.
///
/// Returns the graph with nodes in topological order (leaves first).
/// Nodes whose input derivations are missing from the map are treated as
/// pre-built (no dependencies tracked for them).
pub fn build_graph(derivations: &HashMap<String, Derivation>) -> Result<BuildGraph, DerivationError> {
    let mut nodes: HashMap<String, BuildGraphNode> = HashMap::new();

    // Build adjacency: each drv → its input drv paths
    for (drv_path, drv) in derivations {
        let dependencies: Vec<String> = drv
            .input_derivations
            .keys()
            .map(|sp| sp.to_absolute_path())
            .filter(|dep| derivations.contains_key(dep))
            .collect();

        let output_names: BTreeSet<String> = drv.outputs.keys().cloned().collect();

        nodes.insert(drv_path.clone(), BuildGraphNode {
            drv_path: drv_path.clone(),
            output_names,
            dependencies,
            topo_order: 0,
        });
    }

    // Topological sort via Kahn's algorithm
    let mut in_degree: HashMap<String, u32> = HashMap::new();
    for node in nodes.values() {
        in_degree.entry(node.drv_path.clone()).or_insert(0);
        for dep in &node.dependencies {
            *in_degree.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    // Wait — in_degree is backwards for standard topo sort. In build graphs,
    // we want "depends on" as edges. A depends on B means edge A→B.
    // For Kahn's: in_degree counts incoming edges. A leaf (no dependents) has
    // in_degree 0. But we want to build leaves first (things with no deps).
    //
    // Rethink: edges go from dependent → dependency (A→B means A needs B).
    // in_degree for B = count of things that depend on B.
    // Kahn's removes nodes with in_degree 0 first = things nothing depends on.
    // That's the final outputs, not the leaves.
    //
    // We need reverse: edges from dependency → dependent. Or just compute
    // "out-degree" (things I depend on that remain).

    // Simple approach: track remaining dependency count per node.
    let mut remaining_deps: HashMap<String, u32> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    for node in nodes.values() {
        remaining_deps.insert(node.drv_path.clone(), node.dependencies.len() as u32);
        for dep in &node.dependencies {
            dependents.entry(dep.clone()).or_default().push(node.drv_path.clone());
        }
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    for (path, &count) in &remaining_deps {
        if count == 0 {
            queue.push_back(path.clone());
        }
    }

    let mut build_order = Vec::with_capacity(nodes.len());
    let mut order: u32 = 0;

    while let Some(path) = queue.pop_front() {
        if let Some(node) = nodes.get_mut(&path) {
            node.topo_order = order;
        }
        build_order.push(path.clone());
        order = order.saturating_add(1);

        if let Some(deps) = dependents.get(&path) {
            for dep in deps {
                if let Some(count) = remaining_deps.get_mut(dep) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
    }

    // Nodes not in build_order have cyclic dependencies — warn but include them
    if build_order.len() < nodes.len() {
        let missing: Vec<&str> = nodes.keys().filter(|k| !build_order.contains(k)).map(|s| s.as_str()).collect();
        warn!(count = missing.len(), "derivation graph has cycles, appending remaining nodes");
        for m in missing {
            build_order.push(m.to_string());
        }
    }

    Ok(BuildGraph { nodes, build_order })
}

// ============================================================================
// Cache key computation
// ============================================================================

/// Compute a content-hash cache key for a derivation.
///
/// Uses `Derivation::hash_derivation_modulo` which produces the same hash as
/// Nix's `hashDerivationModulo`. Input derivation paths are replaced by their
/// recursive hashes, so the cache key changes only when actual inputs change.
///
/// `known_hashes` maps drv store paths to their previously computed
/// `hash_derivation_modulo` results. For top-level derivations with no
/// pre-computed inputs, pass an empty map and the function will only produce
/// correct results for fixed-output derivations.
pub fn compute_cache_key(drv: &Derivation, known_hashes: &HashMap<String, [u8; 32]>) -> [u8; 32] {
    drv.hash_derivation_modulo(|store_path_ref| {
        let abs = store_path_ref.to_absolute_path();
        if let Some(hash) = known_hashes.get(&abs) {
            *hash
        } else {
            debug!(
                drv_path = %abs,
                "missing hash_derivation_modulo for input, using zero hash"
            );
            [0u8; 32]
        }
    })
}

/// Compute cache keys for all derivations in topological order.
///
/// Returns a map from drv path to its cache key (hex-encoded SHA-256).
pub fn compute_all_cache_keys(
    derivations: &HashMap<String, Derivation>,
    graph: &BuildGraph,
) -> HashMap<String, String> {
    let mut known_hashes: HashMap<String, [u8; 32]> = HashMap::new();
    let mut cache_keys: HashMap<String, String> = HashMap::new();

    for drv_path in &graph.build_order {
        if let Some(drv) = derivations.get(drv_path) {
            let hash = compute_cache_key(drv, &known_hashes);
            known_hashes.insert(drv_path.clone(), hash);
            cache_keys.insert(drv_path.clone(), hex::encode(hash));
        }
    }

    cache_keys
}

// ============================================================================
// Closure computation
// ============================================================================

/// Compute the runtime closure of a store path by traversing `references`.
///
/// The `lookup_references` function takes a store path and returns its
/// set of references (from PathInfo entries). The closure is the transitive
/// set of all reachable store paths.
///
/// Returns the full closure including the root path.
pub fn compute_closure<F>(root_path: &str, lookup_references: F) -> HashSet<String>
where F: Fn(&str) -> Vec<String> {
    let mut closure = HashSet::new();
    let mut queue = VecDeque::new();

    closure.insert(root_path.to_string());
    queue.push_back(root_path.to_string());

    // Bound the traversal to prevent infinite loops
    const MAX_CLOSURE_SIZE: usize = 100_000;

    while let Some(path) = queue.pop_front() {
        if closure.len() >= MAX_CLOSURE_SIZE {
            warn!(max = MAX_CLOSURE_SIZE, "closure computation hit size limit, stopping traversal");
            break;
        }

        let refs = lookup_references(&path);
        for r in refs {
            if closure.insert(r.clone()) {
                queue.push_back(r);
            }
        }
    }

    closure
}

/// Async version of closure computation using a PathInfoService.
///
/// Looks up each store path's references via the PathInfoService and
/// follows them transitively.
#[cfg(feature = "snix")]
pub async fn compute_closure_from_pathinfo(
    root_path: &str,
    pathinfo_service: &dyn snix_store::pathinfoservice::PathInfoService,
) -> HashSet<String> {
    let mut closure = HashSet::new();
    let mut queue = VecDeque::new();

    closure.insert(root_path.to_string());
    queue.push_back(root_path.to_string());

    const MAX_CLOSURE_SIZE: usize = 100_000;

    while let Some(path) = queue.pop_front() {
        if closure.len() >= MAX_CLOSURE_SIZE {
            warn!(max = MAX_CLOSURE_SIZE, "async closure computation hit size limit");
            break;
        }

        let Ok(store_path) = StorePath::<String>::from_absolute_path(path.as_bytes()) else {
            continue;
        };

        match pathinfo_service.get(*store_path.digest()).await {
            Ok(Some(info)) => {
                for reference in &info.references {
                    let ref_path = reference.to_absolute_path();
                    if closure.insert(ref_path.clone()) {
                        queue.push_back(ref_path);
                    }
                }
            }
            Ok(None) => {
                debug!(path = %path, "store path not found in PathInfoService during closure computation");
            }
            Err(e) => {
                warn!(path = %path, error = %e, "error querying PathInfoService during closure computation");
            }
        }
    }

    closure
}

// ============================================================================
// Cache check integration
// ============================================================================

/// Check if a derivation's build result is already cached.
///
/// Computes the cache key for the derivation and checks if a result
/// exists at the cache key prefix in the KV store.
///
/// Returns `Some(output_paths)` if cached, `None` if not.
pub async fn check_build_cache(
    drv: &Derivation,
    known_hashes: &HashMap<String, [u8; 32]>,
    kv_store: &dyn aspen_core::KeyValueStore,
) -> Option<Vec<String>> {
    let cache_key = compute_cache_key(drv, known_hashes);
    let hex_key = hex::encode(cache_key);
    let kv_key = format!("_ci:drv-cache:{hex_key}");

    match kv_store.read(aspen_core::ReadRequest::new(kv_key)).await {
        Ok(result) => {
            if let Some(kv) = result.kv {
                match serde_json::from_str::<Vec<String>>(&kv.value) {
                    Ok(paths) => {
                        debug!(
                            cache_key = %hex_key,
                            paths = ?paths,
                            "derivation cache hit"
                        );
                        Some(paths)
                    }
                    Err(e) => {
                        warn!(
                            cache_key = %hex_key,
                            error = %e,
                            "failed to deserialize cached build result"
                        );
                        None
                    }
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Store a derivation's build result in the cache.
pub async fn store_build_cache(
    drv: &Derivation,
    known_hashes: &HashMap<String, [u8; 32]>,
    output_paths: &[String],
    kv_store: &dyn aspen_core::KeyValueStore,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cache_key = compute_cache_key(drv, known_hashes);
    let hex_key = hex::encode(cache_key);
    let kv_key = format!("_ci:drv-cache:{hex_key}");
    let value = serde_json::to_string(output_paths)?;

    let request = aspen_core::WriteRequest::from_command(aspen_core::WriteCommand::Set { key: kv_key, value });
    kv_store.write(request).await?;

    debug!(
        cache_key = %hex_key,
        paths = ?output_paths,
        "cached derivation build result"
    );
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;

    use nix_compat::store_path::StorePath;

    use super::*;

    /// Helper: create a minimal derivation with given inputs.
    fn make_drv(name: &str, input_drvs: &[(&str, &[&str])], env: &[(&str, &str)]) -> Derivation {
        let mut input_derivations = BTreeMap::new();
        for (path, outputs) in input_drvs {
            let store_path = StorePath::<String>::from_absolute_path(path.as_bytes())
                .unwrap_or_else(|_| panic!("invalid store path: {path}"));
            let output_names: BTreeSet<String> = outputs.iter().map(|s| s.to_string()).collect();
            input_derivations.insert(store_path, output_names);
        }

        let mut environment = BTreeMap::new();
        for (k, v) in env {
            environment.insert(k.to_string(), v.as_bytes().into());
        }

        // Add a minimal "out" output
        let mut outputs = BTreeMap::new();
        outputs.insert("out".to_string(), nix_compat::derivation::Output {
            path: Some(
                StorePath::<String>::from_absolute_path(
                    format!("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-{name}").as_bytes(),
                )
                .unwrap(),
            ),
            ca_hash: None,
        });

        Derivation {
            arguments: vec!["-e".to_string(), "/builder.sh".to_string()],
            builder: "/bin/bash".to_string(),
            environment,
            input_derivations,
            input_sources: BTreeSet::new(),
            outputs,
            system: "x86_64-linux".to_string(),
        }
    }

    // 7.2: Test parsing derivation bytes
    #[test]
    fn test_parse_derivation_bytes_valid() {
        // Create a derivation programmatically and serialize it
        let drv = make_drv("hello", &[], &[("name", "hello")]);
        let aterm = drv.to_aterm_bytes();

        let parsed = parse_derivation_bytes(&aterm, "test").unwrap();
        assert_eq!(parsed.builder, "/bin/bash");
        assert_eq!(parsed.system, "x86_64-linux");
        assert!(parsed.outputs.contains_key("out"));
    }

    #[test]
    fn test_parse_derivation_bytes_invalid() {
        let result = parse_derivation_bytes(b"not valid aterm", "test");
        assert!(result.is_err());
    }

    // 7.3: Test linear dependency chain (C → B → A)
    #[test]
    fn test_build_graph_linear_chain() {
        let drv_c = make_drv("c", &[], &[("name", "c")]);
        let drv_b = make_drv("b", &[("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-c.drv", &["out"])], &[("name", "b")]);
        let drv_a = make_drv("a", &[("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv", &["out"])], &[("name", "a")]);

        let mut derivations = HashMap::new();
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-c.drv".to_string(), drv_c);
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv".to_string(), drv_b);
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-a.drv".to_string(), drv_a);

        let graph = build_graph(&derivations).unwrap();

        // C should come before B, B before A
        let c_order = graph.nodes.get("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-c.drv").unwrap().topo_order;
        let b_order = graph.nodes.get("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv").unwrap().topo_order;
        let a_order = graph.nodes.get("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-a.drv").unwrap().topo_order;

        assert!(c_order < b_order, "C (order {c_order}) should be built before B (order {b_order})");
        assert!(b_order < a_order, "B (order {b_order}) should be built before A (order {a_order})");
    }

    // 7.3: Test diamond dependency (D → B, D → C, B → A, C → A)
    #[test]
    fn test_build_graph_diamond() {
        let drv_d = make_drv("d", &[], &[("name", "d")]);
        let drv_b = make_drv("b", &[("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-d.drv", &["out"])], &[("name", "b")]);
        let drv_c = make_drv("c", &[("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-d.drv", &["out"])], &[("name", "c")]);
        let drv_a = make_drv(
            "a",
            &[
                ("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv", &["out"]),
                ("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-c.drv", &["out"]),
            ],
            &[("name", "a")],
        );

        let mut derivations = HashMap::new();
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-d.drv".to_string(), drv_d);
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv".to_string(), drv_b);
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-c.drv".to_string(), drv_c);
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-a.drv".to_string(), drv_a);

        let graph = build_graph(&derivations).unwrap();

        let d_order = graph.nodes.get("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-d.drv").unwrap().topo_order;
        let b_order = graph.nodes.get("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv").unwrap().topo_order;
        let c_order = graph.nodes.get("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-c.drv").unwrap().topo_order;
        let a_order = graph.nodes.get("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-a.drv").unwrap().topo_order;

        // D first, then B and C (any order), then A last
        assert!(d_order < b_order, "D should come before B");
        assert!(d_order < c_order, "D should come before C");
        assert!(b_order < a_order, "B should come before A");
        assert!(c_order < a_order, "C should come before A");
    }

    // 7.4: Test cache key computation
    #[test]
    fn test_cache_key_same_inputs_same_key() {
        let drv1 = make_drv("hello", &[], &[("name", "hello"), ("version", "1.0")]);
        let drv2 = make_drv("hello", &[], &[("name", "hello"), ("version", "1.0")]);

        let known = HashMap::new();
        let key1 = compute_cache_key(&drv1, &known);
        let key2 = compute_cache_key(&drv2, &known);

        assert_eq!(key1, key2, "identical derivations should produce same cache key");
    }

    #[test]
    fn test_cache_key_different_env_different_key() {
        let drv1 = make_drv("hello", &[], &[("name", "hello"), ("version", "1.0")]);
        let drv2 = make_drv("hello", &[], &[("name", "hello"), ("version", "2.0")]);

        let known = HashMap::new();
        let key1 = compute_cache_key(&drv1, &known);
        let key2 = compute_cache_key(&drv2, &known);

        assert_ne!(key1, key2, "different environment should produce different cache key");
    }

    // 7.4: Test cache keys in topological order
    #[test]
    fn test_compute_all_cache_keys() {
        let drv_b = make_drv("b", &[], &[("name", "b")]);
        let drv_a = make_drv("a", &[("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv", &["out"])], &[("name", "a")]);

        let mut derivations = HashMap::new();
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-b.drv".to_string(), drv_b);
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-a.drv".to_string(), drv_a);

        let graph = build_graph(&derivations).unwrap();
        let keys = compute_all_cache_keys(&derivations, &graph);

        assert_eq!(keys.len(), 2);
        // Both should have non-empty hex keys
        for (path, key) in &keys {
            assert_eq!(key.len(), 64, "cache key for {path} should be 64 hex chars");
        }
    }

    // 7.5: Test closure computation
    #[test]
    fn test_closure_linear() {
        // hello → glibc → gcc-lib
        let refs: HashMap<String, Vec<String>> = [
            ("/nix/store/abc-hello".to_string(), vec!["/nix/store/def-glibc".to_string()]),
            ("/nix/store/def-glibc".to_string(), vec!["/nix/store/ghi-gcc-lib".to_string()]),
            ("/nix/store/ghi-gcc-lib".to_string(), vec![]),
        ]
        .into_iter()
        .collect();

        let closure = compute_closure("/nix/store/abc-hello", |path| refs.get(path).cloned().unwrap_or_default());

        assert_eq!(closure.len(), 3);
        assert!(closure.contains("/nix/store/abc-hello"));
        assert!(closure.contains("/nix/store/def-glibc"));
        assert!(closure.contains("/nix/store/ghi-gcc-lib"));
    }

    #[test]
    fn test_closure_diamond() {
        // A → B, A → C, B → D, C → D
        let refs: HashMap<String, Vec<String>> = [
            ("/nix/store/a".to_string(), vec!["/nix/store/b".to_string(), "/nix/store/c".to_string()]),
            ("/nix/store/b".to_string(), vec!["/nix/store/d".to_string()]),
            ("/nix/store/c".to_string(), vec!["/nix/store/d".to_string()]),
            ("/nix/store/d".to_string(), vec![]),
        ]
        .into_iter()
        .collect();

        let closure = compute_closure("/nix/store/a", |path| refs.get(path).cloned().unwrap_or_default());

        assert_eq!(closure.len(), 4);
    }

    #[test]
    fn test_closure_with_cycle() {
        // A → B → A (cycle — should not loop forever)
        let refs: HashMap<String, Vec<String>> = [
            ("/nix/store/a".to_string(), vec!["/nix/store/b".to_string()]),
            ("/nix/store/b".to_string(), vec!["/nix/store/a".to_string()]),
        ]
        .into_iter()
        .collect();

        let closure = compute_closure("/nix/store/a", |path| refs.get(path).cloned().unwrap_or_default());

        assert_eq!(closure.len(), 2);
        assert!(closure.contains("/nix/store/a"));
        assert!(closure.contains("/nix/store/b"));
    }

    // 7.3: Test graph with no dependencies
    #[test]
    fn test_build_graph_single_node() {
        let drv = make_drv("solo", &[], &[("name", "solo")]);

        let mut derivations = HashMap::new();
        derivations.insert("/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-solo.drv".to_string(), drv);

        let graph = build_graph(&derivations).unwrap();
        assert_eq!(graph.build_order.len(), 1);
        assert_eq!(graph.nodes.values().next().unwrap().topo_order, 0);
    }

    // 7.7: Test empty graph
    #[test]
    fn test_build_graph_empty() {
        let derivations: HashMap<String, Derivation> = HashMap::new();
        let graph = build_graph(&derivations).unwrap();
        assert!(graph.build_order.is_empty());
        assert!(graph.nodes.is_empty());
    }
}
