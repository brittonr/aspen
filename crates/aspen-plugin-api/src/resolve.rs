//! Dependency resolution for plugin manifests.
//!
//! This module implements topological sorting of plugin load order based on
//! declared dependencies, version checking, and API compatibility validation.
//!
//! Tiger Style: All validation happens before side effects. Errors are collected
//! and returned together, not one at a time.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;

use crate::PLUGIN_API_VERSION;
use crate::PluginManifest;

/// Errors from dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyError {
    /// A required plugin is not installed.
    Missing {
        plugin: String,
        requires: String,
        min_version: Option<String>,
    },
    /// Installed dependency version is too old.
    VersionMismatch {
        plugin: String,
        requires: String,
        min_version: String,
        actual_version: String,
    },
    /// Dependency graph has a cycle.
    Cycle(Vec<String>),
    /// Plugin requires a newer API version than available.
    ApiVersionTooNew {
        plugin: String,
        requires: String,
        current: String,
    },
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyError::Missing {
                plugin,
                requires,
                min_version,
            } => {
                write!(f, "Plugin '{}' requires '{}'", plugin, requires)?;
                if let Some(ver) = min_version {
                    write!(f, " >= {}", ver)?;
                }
                write!(f, " which is not installed")
            }
            DependencyError::VersionMismatch {
                plugin,
                requires,
                min_version,
                actual_version,
            } => {
                write!(
                    f,
                    "Plugin '{}' requires '{}' >= {}, but found version {}",
                    plugin, requires, min_version, actual_version
                )
            }
            DependencyError::Cycle(path) => {
                write!(f, "Dependency cycle detected: {}", path.join(" -> "))
            }
            DependencyError::ApiVersionTooNew {
                plugin,
                requires,
                current,
            } => {
                write!(
                    f,
                    "Plugin '{}' requires API version >= {}, but current API version is {}",
                    plugin, requires, current
                )
            }
        }
    }
}

impl std::error::Error for DependencyError {}

/// Compute topological load order for plugin manifests.
///
/// Returns manifests sorted so dependencies load before dependents.
/// Disabled plugins are excluded. Optional missing deps are skipped.
/// Hard missing deps produce errors.
///
/// Uses Kahn's algorithm with bounded iteration (max MAX_PLUGINS).
///
/// # Arguments
/// * `manifests` - All installed plugin manifests
///
/// # Returns
/// * `Ok(Vec<&PluginManifest>)` - Manifests in load order
/// * `Err(Vec<DependencyError>)` - All validation errors
///
/// # Tiger Style
/// - All errors collected before returning
/// - No partial state on error
/// - Bounded iteration prevents infinite loops
pub fn resolve_load_order(manifests: &[PluginManifest]) -> Result<Vec<&PluginManifest>, Vec<DependencyError>> {
    // Filter to enabled manifests only
    let enabled: Vec<&PluginManifest> = manifests.iter().filter(|m| m.enabled).collect();

    // Build name → index lookup map
    let mut name_to_idx: HashMap<&str, usize> = HashMap::new();
    for (idx, manifest) in enabled.iter().enumerate() {
        name_to_idx.insert(&manifest.name, idx);
    }

    // Build dependency graph and collect errors
    let mut errors: Vec<DependencyError> = Vec::new();
    let mut in_degree: Vec<usize> = vec![0; enabled.len()];
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); enabled.len()];

    for (idx, manifest) in enabled.iter().enumerate() {
        for dep in &manifest.dependencies {
            if dep.optional {
                // Optional dep missing → skip silently
                if let Some(&dep_idx) = name_to_idx.get(dep.name.as_str()) {
                    // Check version if dependency exists
                    let dep_manifest = enabled[dep_idx];
                    if let Some(min_ver) = &dep.min_version
                        && !version_satisfies(&dep_manifest.version, min_ver)
                    {
                        errors.push(DependencyError::VersionMismatch {
                            plugin: manifest.name.clone(),
                            requires: dep.name.clone(),
                            min_version: min_ver.clone(),
                            actual_version: dep_manifest.version.clone(),
                        });
                    }
                    // Add edge: dep → dependent
                    edges[dep_idx].push(idx);
                    in_degree[idx] += 1;
                }
                // If optional dep doesn't exist, silently skip
            } else {
                // Hard dep missing → error
                if let Some(&dep_idx) = name_to_idx.get(dep.name.as_str()) {
                    let dep_manifest = enabled[dep_idx];
                    if let Some(min_ver) = &dep.min_version
                        && !version_satisfies(&dep_manifest.version, min_ver)
                    {
                        errors.push(DependencyError::VersionMismatch {
                            plugin: manifest.name.clone(),
                            requires: dep.name.clone(),
                            min_version: min_ver.clone(),
                            actual_version: dep_manifest.version.clone(),
                        });
                    }
                    // Add edge: dep → dependent
                    edges[dep_idx].push(idx);
                    in_degree[idx] += 1;
                } else {
                    errors.push(DependencyError::Missing {
                        plugin: manifest.name.clone(),
                        requires: dep.name.clone(),
                        min_version: dep.min_version.clone(),
                    });
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Kahn's algorithm: queue all with in-degree 0
    let mut queue: VecDeque<usize> = VecDeque::new();
    for (idx, &degree) in in_degree.iter().enumerate() {
        if degree == 0 {
            queue.push_back(idx);
        }
    }

    let mut sorted: Vec<&PluginManifest> = Vec::new();
    while let Some(idx) = queue.pop_front() {
        sorted.push(enabled[idx]);
        for &dependent_idx in &edges[idx] {
            in_degree[dependent_idx] -= 1;
            if in_degree[dependent_idx] == 0 {
                queue.push_back(dependent_idx);
            }
        }
    }

    // If output.len() < filtered.len() → cycle among remaining
    if sorted.len() < enabled.len() {
        // Find nodes in cycle (those with in_degree > 0)
        let mut cycle_nodes: Vec<String> = enabled
            .iter()
            .enumerate()
            .filter(|(idx, _)| in_degree[*idx] > 0)
            .map(|(_, m)| m.name.clone())
            .collect();
        cycle_nodes.sort();
        return Err(vec![DependencyError::Cycle(cycle_nodes)]);
    }

    Ok(sorted)
}

/// Validate a manifest's dependencies against currently installed plugins.
/// Used at install time before writing to KV.
///
/// # Arguments
/// * `manifest` - Manifest to validate
/// * `installed` - Currently installed plugin manifests
///
/// # Returns
/// * `Ok(())` - All dependencies satisfied
/// * `Err(Vec<DependencyError>)` - All validation errors
///
/// # Tiger Style
/// - All errors collected before returning
/// - Check API version first
/// - Then check all dependencies
pub fn validate_install(manifest: &PluginManifest, installed: &[PluginManifest]) -> Result<(), Vec<DependencyError>> {
    let mut errors: Vec<DependencyError> = Vec::new();

    // Check API version
    if let Err(e) = check_api_version(manifest) {
        errors.push(e);
    }

    // Build name → manifest lookup
    let mut installed_map: HashMap<&str, &PluginManifest> = HashMap::new();
    for m in installed {
        installed_map.insert(&m.name, m);
    }

    // Check each dependency
    for dep in &manifest.dependencies {
        if dep.optional {
            // Optional dep: check version if it exists
            if let Some(&dep_manifest) = installed_map.get(dep.name.as_str())
                && let Some(min_ver) = &dep.min_version
                && !version_satisfies(&dep_manifest.version, min_ver)
            {
                errors.push(DependencyError::VersionMismatch {
                    plugin: manifest.name.clone(),
                    requires: dep.name.clone(),
                    min_version: min_ver.clone(),
                    actual_version: dep_manifest.version.clone(),
                });
            }
            // Missing optional dep is OK
        } else {
            // Hard dep: must exist
            if let Some(&dep_manifest) = installed_map.get(dep.name.as_str()) {
                if let Some(min_ver) = &dep.min_version
                    && !version_satisfies(&dep_manifest.version, min_ver)
                {
                    errors.push(DependencyError::VersionMismatch {
                        plugin: manifest.name.clone(),
                        requires: dep.name.clone(),
                        min_version: min_ver.clone(),
                        actual_version: dep_manifest.version.clone(),
                    });
                }
            } else {
                errors.push(DependencyError::Missing {
                    plugin: manifest.name.clone(),
                    requires: dep.name.clone(),
                    min_version: dep.min_version.clone(),
                });
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// Find which installed plugins depend on the named plugin.
/// Used at remove time to prevent breaking dependents.
///
/// # Arguments
/// * `name` - Name of the plugin to check
/// * `installed` - Currently installed plugin manifests
///
/// # Returns
/// * Vec of plugin names that depend on `name`
pub fn reverse_dependents(name: &str, installed: &[PluginManifest]) -> Vec<String> {
    let mut dependents: Vec<String> = Vec::new();
    for manifest in installed {
        for dep in &manifest.dependencies {
            if dep.name == name && !dep.optional {
                dependents.push(manifest.name.clone());
                break;
            }
        }
    }
    dependents.sort();
    dependents
}

/// Check a plugin's min_api_version against PLUGIN_API_VERSION.
///
/// # Arguments
/// * `manifest` - Manifest to check
///
/// # Returns
/// * `Ok(())` - API version is compatible
/// * `Err(DependencyError::ApiVersionTooNew)` - Plugin requires newer API
pub fn check_api_version(manifest: &PluginManifest) -> Result<(), DependencyError> {
    if let Some(min_api) = &manifest.min_api_version
        && !version_satisfies(PLUGIN_API_VERSION, min_api)
    {
        return Err(DependencyError::ApiVersionTooNew {
            plugin: manifest.name.clone(),
            requires: min_api.clone(),
            current: PLUGIN_API_VERSION.to_string(),
        });
    }
    Ok(())
}

/// Compare versions using semver. Falls back to string equality if not valid semver.
///
/// # Arguments
/// * `actual` - Actual version string
/// * `min_version` - Minimum required version string
///
/// # Returns
/// * `true` if actual >= min_version
fn version_satisfies(actual: &str, min_version: &str) -> bool {
    // Try semver comparison first
    if let (Ok(actual_ver), Ok(min_ver)) = (semver::Version::parse(actual), semver::Version::parse(min_version)) {
        return actual_ver >= min_ver;
    }
    // Fallback to string equality
    actual == min_version
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginDependency;
    use crate::PluginPermissions;

    fn manifest(name: &str, version: &str, deps: Vec<PluginDependency>) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: version.to_string(),
            wasm_hash: "hash".to_string(),
            handles: vec![],
            priority: 900,
            fuel_limit: None,
            memory_limit: None,
            enabled: true,
            app_id: None,
            execution_timeout_secs: None,
            kv_prefixes: vec![],
            permissions: PluginPermissions::default(),
            signature: None,
            description: None,
            author: None,
            tags: vec![],
            min_api_version: None,
            dependencies: deps,
        }
    }

    fn dep(name: &str, min_version: Option<&str>, optional: bool) -> PluginDependency {
        PluginDependency {
            name: name.to_string(),
            min_version: min_version.map(|s| s.to_string()),
            optional,
        }
    }

    #[test]
    fn test_no_dependencies() {
        let manifests = vec![
            manifest("a", "1.0.0", vec![]),
            manifest("b", "1.0.0", vec![]),
            manifest("c", "1.0.0", vec![]),
        ];
        let order = resolve_load_order(&manifests).unwrap();
        assert_eq!(order.len(), 3);
        // All have in-degree 0, order is stable (same as input)
        assert_eq!(order[0].name, "a");
        assert_eq!(order[1].name, "b");
        assert_eq!(order[2].name, "c");
    }

    #[test]
    fn test_linear_chain() {
        // A → B → C means C loads first, then B, then A
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("b", None, false)]),
            manifest("b", "1.0.0", vec![dep("c", None, false)]),
            manifest("c", "1.0.0", vec![]),
        ];
        let order = resolve_load_order(&manifests).unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0].name, "c");
        assert_eq!(order[1].name, "b");
        assert_eq!(order[2].name, "a");
    }

    #[test]
    fn test_diamond() {
        // A → B, A → C, B → D, C → D
        // D loads first, B/C middle, A last
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("b", None, false), dep("c", None, false)]),
            manifest("b", "1.0.0", vec![dep("d", None, false)]),
            manifest("c", "1.0.0", vec![dep("d", None, false)]),
            manifest("d", "1.0.0", vec![]),
        ];
        let order = resolve_load_order(&manifests).unwrap();
        assert_eq!(order.len(), 4);
        assert_eq!(order[0].name, "d");
        // B and C can be in any order
        assert!(order[1].name == "b" || order[1].name == "c");
        assert!(order[2].name == "b" || order[2].name == "c");
        assert_eq!(order[3].name, "a");
    }

    #[test]
    fn test_cycle_detection() {
        // A → B → A
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("b", None, false)]),
            manifest("b", "1.0.0", vec![dep("a", None, false)]),
        ];
        let err = resolve_load_order(&manifests).unwrap_err();
        assert_eq!(err.len(), 1);
        match &err[0] {
            DependencyError::Cycle(path) => {
                assert_eq!(path.len(), 2);
                assert!(path.contains(&"a".to_string()));
                assert!(path.contains(&"b".to_string()));
            }
            _ => panic!("Expected Cycle error"),
        }
    }

    #[test]
    fn test_missing_hard_dependency() {
        let manifests = vec![manifest("a", "1.0.0", vec![dep("b", None, false)])];
        let err = resolve_load_order(&manifests).unwrap_err();
        assert_eq!(err.len(), 1);
        match &err[0] {
            DependencyError::Missing {
                plugin,
                requires,
                min_version,
            } => {
                assert_eq!(plugin, "a");
                assert_eq!(requires, "b");
                assert_eq!(min_version, &None);
            }
            _ => panic!("Expected Missing error"),
        }
    }

    #[test]
    fn test_missing_optional_dependency() {
        // Optional dep missing should not error
        let manifests = vec![manifest("a", "1.0.0", vec![dep("b", None, true)])];
        let order = resolve_load_order(&manifests).unwrap();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name, "a");
    }

    #[test]
    fn test_version_mismatch() {
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("b", Some("2.0.0"), false)]),
            manifest("b", "1.5.0", vec![]),
        ];
        let err = resolve_load_order(&manifests).unwrap_err();
        assert_eq!(err.len(), 1);
        match &err[0] {
            DependencyError::VersionMismatch {
                plugin,
                requires,
                min_version,
                actual_version,
            } => {
                assert_eq!(plugin, "a");
                assert_eq!(requires, "b");
                assert_eq!(min_version, "2.0.0");
                assert_eq!(actual_version, "1.5.0");
            }
            _ => panic!("Expected VersionMismatch error"),
        }
    }

    #[test]
    fn test_version_satisfied() {
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("b", Some("1.0.0"), false)]),
            manifest("b", "1.5.0", vec![]),
        ];
        let order = resolve_load_order(&manifests).unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0].name, "b");
        assert_eq!(order[1].name, "a");
    }

    #[test]
    fn test_api_version_too_new() {
        let mut m = manifest("a", "1.0.0", vec![]);
        m.min_api_version = Some("999.0.0".to_string());
        let err = check_api_version(&m).unwrap_err();
        match err {
            DependencyError::ApiVersionTooNew {
                plugin,
                requires,
                current,
            } => {
                assert_eq!(plugin, "a");
                assert_eq!(requires, "999.0.0");
                assert_eq!(current, PLUGIN_API_VERSION);
            }
            _ => panic!("Expected ApiVersionTooNew error"),
        }
    }

    #[test]
    fn test_api_version_ok() {
        let mut m = manifest("a", "1.0.0", vec![]);
        m.min_api_version = Some("0.1.0".to_string());
        assert!(check_api_version(&m).is_ok());
    }

    #[test]
    fn test_disabled_plugins_excluded() {
        let mut m = manifest("a", "1.0.0", vec![dep("b", None, false)]);
        m.enabled = false;
        let manifests = vec![m, manifest("b", "1.0.0", vec![])];
        let order = resolve_load_order(&manifests).unwrap();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name, "b");
    }

    #[test]
    fn test_reverse_dependents() {
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("c", None, false)]),
            manifest("b", "1.0.0", vec![dep("c", None, false)]),
            manifest("c", "1.0.0", vec![]),
        ];
        let deps = reverse_dependents("c", &manifests);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], "a");
        assert_eq!(deps[1], "b");
    }

    #[test]
    fn test_reverse_dependents_excludes_optional() {
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("c", None, false)]),
            manifest("b", "1.0.0", vec![dep("c", None, true)]), // optional
            manifest("c", "1.0.0", vec![]),
        ];
        let deps = reverse_dependents("c", &manifests);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "a");
    }

    #[test]
    fn test_validate_install_missing_dep() {
        let m = manifest("a", "1.0.0", vec![dep("b", None, false)]);
        let installed = vec![];
        let err = validate_install(&m, &installed).unwrap_err();
        assert_eq!(err.len(), 1);
        match &err[0] {
            DependencyError::Missing { plugin, .. } => {
                assert_eq!(plugin, "a");
            }
            _ => panic!("Expected Missing error"),
        }
    }

    #[test]
    fn test_validate_install_ok() {
        let m = manifest("a", "1.0.0", vec![dep("b", Some("1.0.0"), false)]);
        let installed = vec![manifest("b", "1.5.0", vec![])];
        assert!(validate_install(&m, &installed).is_ok());
    }

    #[test]
    fn test_non_semver_version_strings() {
        // Fallback to string equality for non-semver versions
        assert!(version_satisfies("foo", "foo"));
        assert!(!version_satisfies("foo", "bar"));
        assert!(version_satisfies("1.0.0", "1.0.0"));
        assert!(version_satisfies("1.5.0", "1.0.0"));
        assert!(!version_satisfies("0.9.0", "1.0.0"));
    }

    #[test]
    fn test_empty_manifest_list() {
        let manifests: Vec<PluginManifest> = vec![];
        let order = resolve_load_order(&manifests).unwrap();
        assert_eq!(order.len(), 0);
    }

    #[test]
    fn test_optional_dependency_version_mismatch() {
        // Optional dep exists but version too low should error
        let manifests = vec![
            manifest("a", "1.0.0", vec![dep("b", Some("2.0.0"), true)]),
            manifest("b", "1.0.0", vec![]),
        ];
        let err = resolve_load_order(&manifests).unwrap_err();
        assert_eq!(err.len(), 1);
        match &err[0] {
            DependencyError::VersionMismatch {
                plugin,
                requires,
                min_version,
                actual_version,
            } => {
                assert_eq!(plugin, "a");
                assert_eq!(requires, "b");
                assert_eq!(min_version, "2.0.0");
                assert_eq!(actual_version, "1.0.0");
            }
            _ => panic!("Expected VersionMismatch error"),
        }
    }
}
