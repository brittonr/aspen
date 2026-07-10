use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::error::MoltenError;
use crate::error::Result;

pub const CLUSTER_MANIFEST_FILE: &str = "cluster.nodes";
const CLUSTER_MANIFEST_HEADER: &str = "molten.cluster.nodes.v1";
const NODE_ID_PREFIX: &str = "node:";
const CURRENT_DIR_COMPONENT: &str = ".";
const PARENT_DIR_COMPONENT: &str = "..";
const NODE_PATH_DASH: char = '-';
const NODE_PATH_UNDERSCORE: char = '_';
const NODE_ID_SEPARATOR: char = ':';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterNodePlan {
    pub requested_node: String,
    pub node_id: String,
    pub path_component: String,
    pub state_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPlan {
    pub state_root: PathBuf,
    pub nodes: Vec<ClusterNodePlan>,
}

pub fn cluster_manifest_path(state_root: &Path) -> PathBuf {
    state_root.join(CLUSTER_MANIFEST_FILE)
}

pub fn plan_cluster(state_root: &Path, node_names: &[String]) -> Result<ClusterPlan> {
    validate_cluster_state_root(state_root)?;
    if node_names.is_empty() {
        return Err(MoltenError::invalid_harness("cluster requires at least one --node"));
    }
    let mut seen_node_ids = BTreeSet::new();
    let mut nodes = Vec::with_capacity(node_names.len());
    for node_name in node_names {
        let node = plan_node(state_root, node_name)?;
        if !seen_node_ids.insert(node.node_id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate cluster node {}", node.node_id)));
        }
        nodes.push(node);
    }
    Ok(ClusterPlan {
        state_root: state_root.to_path_buf(),
        nodes,
    })
}

pub fn render_cluster_manifest(plan: &ClusterPlan) -> String {
    let mut rendered = String::from(CLUSTER_MANIFEST_HEADER);
    rendered.push('\n');
    for node in &plan.nodes {
        rendered.push_str(&node.node_id);
        rendered.push('\n');
    }
    rendered
}

pub fn parse_cluster_manifest(source: &str) -> Result<Vec<String>> {
    let mut lines = source.lines();
    let header = lines.next().ok_or_else(|| MoltenError::invalid_harness("cluster manifest is empty"))?;
    if header != CLUSTER_MANIFEST_HEADER {
        return Err(MoltenError::invalid_harness("cluster manifest has unsupported header"));
    }
    let mut nodes = Vec::new();
    for line in lines {
        if !line.is_empty() {
            nodes.push(line.to_string());
        }
    }
    if nodes.is_empty() {
        return Err(MoltenError::invalid_harness("cluster manifest has no nodes"));
    }
    Ok(nodes)
}

fn validate_cluster_state_root(state_root: &Path) -> Result<()> {
    if state_root.as_os_str().is_empty() {
        return Err(MoltenError::invalid_harness("cluster requires explicit state root"));
    }
    if state_root == Path::new(CURRENT_DIR_COMPONENT) || state_root == Path::new(PARENT_DIR_COMPONENT) {
        return Err(MoltenError::invalid_harness("cluster state root must not be ambient current or parent directory"));
    }
    Ok(())
}

fn plan_node(state_root: &Path, requested_node: &str) -> Result<ClusterNodePlan> {
    let path_component = node_path_component(requested_node)?;
    let node_id = if requested_node.starts_with(NODE_ID_PREFIX) {
        requested_node.to_string()
    } else {
        format!("{NODE_ID_PREFIX}{requested_node}")
    };
    Ok(ClusterNodePlan {
        requested_node: requested_node.to_string(),
        node_id,
        state_root: state_root.join(&path_component),
        path_component,
    })
}

fn node_path_component(requested_node: &str) -> Result<String> {
    if requested_node.is_empty() || requested_node.trim() != requested_node {
        return Err(MoltenError::invalid_harness("cluster node name must be non-empty and unpadded"));
    }
    let component = requested_node.strip_prefix(NODE_ID_PREFIX).unwrap_or(requested_node);
    if component.is_empty() {
        return Err(MoltenError::invalid_harness("cluster node path component must be non-empty"));
    }
    if component == CURRENT_DIR_COMPONENT || component == PARENT_DIR_COMPONENT {
        return Err(MoltenError::invalid_harness("cluster node path component must not be relative syntax"));
    }
    if component.contains(NODE_ID_SEPARATOR) {
        return Err(MoltenError::invalid_harness("cluster node path component must not contain ':'"));
    }
    if !component.chars().all(is_safe_node_path_character) {
        return Err(MoltenError::invalid_harness(
            "cluster node path component must contain only ASCII letters, digits, '-' or '_'",
        ));
    }
    Ok(component.to_string())
}

fn is_safe_node_path_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == NODE_PATH_DASH || character == NODE_PATH_UNDERSCORE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn plans_cluster_nodes_and_round_trips_manifest() {
        const EXPECTED_CLUSTER_NODE_COUNT: usize = 2;

        let root = PathBuf::from("target/cluster");
        let plan = plan_cluster(&root, &node_names(&["node-a", "node_b"])).expect("cluster plan");
        assert_eq!(plan.nodes.len(), EXPECTED_CLUSTER_NODE_COUNT);
        assert_eq!(plan.nodes[0].node_id, "node:node-a");
        assert_eq!(plan.nodes[0].path_component, "node-a");
        assert_eq!(plan.nodes[0].state_root, root.join("node-a"));
        assert_eq!(plan.nodes[1].node_id, "node:node_b");
        assert_eq!(cluster_manifest_path(&root), root.join(CLUSTER_MANIFEST_FILE));

        let manifest = render_cluster_manifest(&plan);
        let parsed = parse_cluster_manifest(&manifest).expect("parse manifest");
        let reparsed_plan = plan_cluster(&root, &parsed).expect("reparsed plan");
        let reparsed_node_ids: Vec<&str> = reparsed_plan.nodes.iter().map(|node| node.node_id.as_str()).collect();
        let planned_node_ids: Vec<&str> = plan.nodes.iter().map(|node| node.node_id.as_str()).collect();
        assert_eq!(reparsed_node_ids, planned_node_ids);
        assert_eq!(reparsed_plan.state_root, plan.state_root);
    }

    #[test]
    fn denies_empty_duplicate_and_unsafe_nodes() {
        let root = PathBuf::from("target/cluster");
        let empty = plan_cluster(&root, &[]).expect_err("empty denied");
        assert!(empty.to_string().contains("at least one"));

        let duplicate = plan_cluster(&root, &node_names(&["node-a", "node:node-a"])).expect_err("duplicate denied");
        assert!(duplicate.to_string().contains("duplicate cluster node"));

        let relative = plan_cluster(&root, &node_names(&["../node-a"])).expect_err("relative denied");
        assert!(relative.to_string().contains("ASCII letters"));

        let colon = plan_cluster(&root, &node_names(&["node:a:b"])).expect_err("colon denied");
        assert!(colon.to_string().contains("must not contain ':'"));

        let current_root = plan_cluster(Path::new("."), &node_names(&["node-a"])).expect_err("current root denied");
        assert!(current_root.to_string().contains("must not be ambient"));

        let parent_root = plan_cluster(Path::new(".."), &node_names(&["node-a"])).expect_err("parent root denied");
        assert!(parent_root.to_string().contains("must not be ambient"));
    }

    #[test]
    fn denies_malformed_manifests() {
        let empty = parse_cluster_manifest("").expect_err("empty manifest denied");
        assert!(empty.to_string().contains("manifest is empty"));

        let header = parse_cluster_manifest("not-a-cluster\nnode:node-a\n").expect_err("bad header denied");
        assert!(header.to_string().contains("unsupported header"));

        let no_nodes = parse_cluster_manifest("molten.cluster.nodes.v1\n").expect_err("empty nodes denied");
        assert!(no_nodes.to_string().contains("no nodes"));
    }
}
