//! Control plane nodes view model
#![allow(dead_code)] // View models used in handlers

use askama::Template;
use crate::domain::cluster_status::ControlPlaneNode;

/// Individual control plane node for display
#[derive(Debug, Clone)]
pub struct NodeRow {
    pub node_number: usize,
    pub is_leader: bool,
    pub leader_css_class: String,
    pub badge_html: String,
}

impl From<ControlPlaneNode> for NodeRow {
    fn from(node: ControlPlaneNode) -> Self {
        Self {
            node_number: node.node_number,
            is_leader: node.is_leader,
            leader_css_class: if node.is_leader {
                " leader".to_string()
            } else {
                String::new()
            },
            badge_html: if node.is_leader {
                r#"<span class="node-badge badge-leader">Leader</span>"#.to_string()
            } else {
                r#"<span class="node-badge badge-follower">Follower</span>"#.to_string()
            },
        }
    }
}

/// View model for control plane nodes list
#[derive(Template)]
#[template(path = "partials/control_plane_nodes.html")]
pub struct ControlPlaneNodesView {
    pub nodes: Vec<NodeRow>,
}

impl ControlPlaneNodesView {
    pub fn new(nodes: Vec<ControlPlaneNode>) -> Self {
        Self {
            nodes: nodes.into_iter().map(NodeRow::from).collect(),
        }
    }

    pub fn empty() -> Self {
        Self { nodes: Vec::new() }
    }
}
