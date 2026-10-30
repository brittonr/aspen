//! Complete measured build cohorts, including the distinct Octet compiler.
use std::collections;
use std::string;
use std::vec;

pub const SCHEMA: &str = "molten-measured-build-closure-v1";
/// Measured union of all six graphs; real candidate plus vendored gate closures exceed 16k roots.
pub const MAX_PATHS: usize = 65_536;
/// Raw exportReferencesGraph file bound (paths plus their reference lists).
pub const MAX_GRAPH_BYTES: u64 = 67_108_864;
/// Measured closure JSON bound: six role inventories plus the measured union.
pub const MAX_CLOSURE_BYTES: u64 = 134_217_728;
/// Upper bound for one store root rendered in path-info JSON references.
pub const MAX_PATH_JSON_BYTES: usize = 320;
pub const ROLES: [&str; 6] = [
    "candidate-build",
    "candidate-compiler",
    "candidate-runtime",
    "gate-build",
    "gate-compiler",
    "gate-runtime",
];

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub role: string::String,
    pub graph_ref: string::String,
    pub paths: vec::Vec<string::String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredPath {
    pub path: string::String,
    pub nar_ref: string::String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Closure {
    pub schema: string::String,
    pub graphs: vec::Vec<Graph>,
    pub paths: vec::Vec<MeasuredPath>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Issue {
    Schema,
    MissingGraph,
    GraphMismatch,
    InvalidPath,
    InvalidIdentity,
    IncompletePaths,
}

/// Requires every category and exact union coverage; observed graphs come from actual retained
/// bytes.
pub fn validate(closure: &Closure, observed: &[Graph]) -> Result<(), Issue> {
    if closure.schema != SCHEMA {
        return Err(Issue::Schema);
    }
    if closure.graphs.len() != ROLES.len() || observed.len() != ROLES.len() {
        return Err(Issue::MissingGraph);
    }
    let mut union = collections::BTreeSet::new();
    for (index, role) in ROLES.iter().enumerate() {
        let graph = &closure.graphs[index];
        if graph.role != *role || observed[index].role != *role {
            return Err(Issue::MissingGraph);
        }
        if graph != &observed[index] {
            return Err(Issue::GraphMismatch);
        }
        if !super::is_content_ref(&graph.graph_ref) {
            return Err(Issue::InvalidIdentity);
        }
        if graph.paths.is_empty() || graph.paths.len() > MAX_PATHS {
            return Err(Issue::IncompletePaths);
        }
        if graph.paths.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(Issue::InvalidPath);
        }
        for path in &graph.paths {
            if !is_store_root(path) {
                return Err(Issue::InvalidPath);
            }
            union.insert(path.as_str());
        }
        if union.len() > MAX_PATHS {
            return Err(Issue::IncompletePaths);
        }
    }
    if closure.paths.len() != union.len() {
        return Err(Issue::IncompletePaths);
    }
    for (declared, actual) in closure.paths.iter().zip(union) {
        if declared.path != actual {
            return Err(Issue::IncompletePaths);
        }
        if !super::is_content_ref(&declared.nar_ref) {
            return Err(Issue::InvalidIdentity);
        }
    }
    Ok(())
}

pub fn is_store_root(path: &str) -> bool {
    let Some(leaf) = path.strip_prefix("/nix/store/") else {
        return false;
    };
    !leaf.is_empty()
        && leaf.len() <= 256
        && leaf != "."
        && leaf != ".."
        && !leaf.bytes().any(|byte| byte == b'/' || byte.is_ascii_whitespace() || byte.is_ascii_control())
}

#[cfg(test)]
mod tests;
