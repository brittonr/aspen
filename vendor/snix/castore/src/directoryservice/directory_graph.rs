use std::collections::HashMap;

use petgraph::{
    Direction,
    graph::{DiGraph, NodeIndex},
    visit::{Bfs, DfsPostOrder, EdgeRef, Walker},
};
use tracing::instrument;

use super::order_validator::{LeavesToRootValidator, OrderValidator, RootToLeavesValidator};
use crate::{B3Digest, Directory, Node, path::PathComponent};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    ValidationError(String),
}

struct EdgeWeight {
    name: PathComponent,
    size: u64,
}

/// This can be used to validate and/or re-order a Directory closure (DAG of
/// connected Directories), and their insertion order.
///
/// The DirectoryGraph is parametrized on the insertion order, and can be
/// constructed using the Default trait, or using `with_order` if the
/// OrderValidator needs to be customized.
///
/// Users normally insert directories via `add` in the specified order.
///
/// During insertion, we validate as much as we can at that time:
///
///  - validation of insertion order
///  - validation of size fields of referred Directories
///
/// Internally this keeps all received Directories in a directed graph,
/// with node weights being the Directories and edges pointing to child/parent
/// directories.
///
/// Once all Directories have been inserted, a `validate` function can be called
/// to perform a check for graph connectivity and ensure there's no disconnected
/// components or missing nodes.
///
/// This returns a [ValidatedDirectoryGraph] type, with
/// [ValidatedDirectoryGraph::drain_leaves_to_root] and
/// [ValidatedDirectoryGraph::drain_root_to_leaves] methods, returning an
/// iterator over the (deduplicated and) validated list of directories in either
/// order.
///
/// Additionally, in the root to leaves case, there's `digest_allowed` and
/// `add_ordered_unchecked` functions, allowing to query whether a potential
/// Directory is expected, and only then insert it.
/// This allows rejecting unexpected directories in serialized form before even
/// parsing them.
///
#[derive(Default)]
pub struct DirectoryGraph<O> {
    // A directed graph, using Directory as node weight.
    // Edges point from parents to children.
    //
    // Nodes with None weights might exist when a digest has been referred to but the directory
    // with this digest has not yet been sent.
    //
    // The option in the edge weight tracks the pending validation state of the respective edge, for example if
    // the child has not been added yet.
    graph: DiGraph<Option<Directory>, Option<EdgeWeight>>,

    // A lookup table from directory digest to node index.
    digest_to_node_ix: HashMap<B3Digest, NodeIndex>,

    order_validator: O,
}

fn check_edge(edge: &EdgeWeight, child: &Directory) -> Result<(), Error> {
    // Ensure the size specified in the child node matches our records.
    if edge.size != child.size() {
        return Err(Error::ValidationError(format!(
            "'{}' has wrong size, specified {}, recorded {}",
            edge.name,
            edge.size,
            child.size(),
        )));
    }
    Ok(())
}

impl DirectoryGraph<LeavesToRootValidator> {
    /// Insert a new Directory into the closure
    #[instrument(level = "trace", skip_all, fields(directory.digest=%directory.digest(), directory.size=%directory.size()), err)]
    pub fn add(&mut self, directory: Directory) -> Result<(), Error> {
        if !self.order_validator.add_directory(&directory) {
            return Err(Error::ValidationError(
                "unknown directory was referenced".into(),
            ));
        }
        self.add_ordered_unchecked(directory)
    }
}

impl DirectoryGraph<RootToLeavesValidator> {
    /// Allows checking a digest on whether it's expected at this time.
    /// If used in combination with `add_ordered_unchecked`, avoids doing the
    /// same lookup twice and parsing invalid, still serialized directories.
    pub fn digest_allowed(&self, digest: &B3Digest) -> bool {
        self.order_validator.digest_allowed(digest)
    }

    /// Insert a new Directory into the closure
    #[instrument(level = "trace", skip_all, fields(directory.digest=%directory.digest(), directory.size=%directory.size()), err)]
    pub fn add(&mut self, directory: Directory) -> Result<(), Error> {
        let digest = directory.digest();
        if !self.order_validator.digest_allowed(&digest) {
            return Err(Error::ValidationError("unexpected digest".into()));
        }
        self.order_validator.add_directory_unchecked(&directory);
        self.add_ordered_unchecked(directory)
    }
}

impl<O: OrderValidator> DirectoryGraph<O> {
    /// Customize the ordering, i.e. for pre-setting the root of the RootToLeavesValidator
    pub fn with_order(order_validator: O) -> Self {
        Self {
            graph: Default::default(),
            digest_to_node_ix: Default::default(),
            order_validator,
        }
    }

    /// Adds a directory which has already been confirmed to be in-order to the graph
    pub fn add_ordered_unchecked(&mut self, directory: Directory) -> Result<(), Error> {
        let digest = directory.digest();

        // Teach the graph about the existence of a node with this digest
        let ix = *self
            .digest_to_node_ix
            .entry(digest)
            .or_insert_with(|| self.graph.add_node(None));

        if self.graph[ix].is_some() {
            // The node is already in the graph, there is nothing to do here.
            return Ok(());
        }

        // set up edges to all child directories
        for (name, node) in directory.nodes() {
            if let Node::Directory { digest, size } = node {
                let child_ix = *self
                    .digest_to_node_ix
                    .entry(digest.clone())
                    .or_insert_with(|| self.graph.add_node(None));

                let pending_edge_check = match &self.graph[child_ix] {
                    Some(child) => {
                        // child is already available, validate the edge now
                        check_edge(
                            &EdgeWeight {
                                name: name.clone(),
                                size: *size,
                            },
                            child,
                        )?;
                        None
                    }
                    None => Some(EdgeWeight {
                        name: name.clone(),
                        size: *size,
                    }), // pending validation
                };
                self.graph.add_edge(ix, child_ix, pending_edge_check);
            }
        }

        // validate the edges from parents to this node
        // this collects edge ids in a Vec because there is no edges_directed_mut :'c
        for edge_id in self
            .graph
            .edges_directed(ix, Direction::Incoming)
            .map(|edge_ref| edge_ref.id())
            .collect::<Vec<_>>()
            .into_iter()
        {
            let edge_weight = self
                .graph
                .edge_weight_mut(edge_id)
                .expect("edge not found")
                .take()
                .expect("edge is already validated");

            check_edge(&edge_weight, &directory)?;
        }

        // finally, store the directory information in the node weight
        self.graph[ix] = Some(directory);

        Ok(())
    }

    #[instrument(level = "trace", skip_all, err)]
    pub fn validate(self) -> Result<ValidatedDirectoryGraph, Error> {
        // find all initial nodes (nodes without incoming edges)
        // there must exactly be one, which is the root.
        let root_idx = {
            let mut init_nodes = self.graph.externals(Direction::Incoming);

            match (init_nodes.next(), init_nodes.next()) {
                (None, _) => return Err(Error::ValidationError("graph has no init nodes".into())),
                (Some(root_idx), None) => root_idx,
                (Some(_idx_1), Some(_idx_2)) => {
                    return Err(Error::ValidationError("graph has no single root".into()));
                }
            }
        };

        // While doing so, ensure none of the node weights are still None (all
        // digests introduced via digests were actually `add()`ed).
        if self.graph.raw_nodes().iter().any(|n| n.weight.is_none()) {
            return Err(Error::ValidationError("graph is incomplete".into()));
        }

        Ok(ValidatedDirectoryGraph {
            graph: self.graph,
            root_idx,
        })
    }
}

/// This represents a validated directory graph, meaning:
/// - it's not empty
/// - it has one root
/// - it is connected
///
/// It can be drained in root-to-leaves, or leaves-to-root order.
pub struct ValidatedDirectoryGraph {
    // NOTE: we only use Option<_> here to avoid ownership problems while draining,
    // and don't look at the edge weights anymore.
    // We don't keep these graphs around for too long, and looping over them
    // multiple times would be worse.
    graph: DiGraph<Option<Directory>, Option<EdgeWeight>>,

    root_idx: NodeIndex,
}

impl ValidatedDirectoryGraph {
    /// Return the list of directories in from-root-to-leaves order.
    #[instrument(level = "trace", skip_all)]
    pub fn drain_root_to_leaves(self) -> impl Iterator<Item = Directory> {
        // do a BFS traversal of the graph, starting with the root node
        let order = Bfs::new(&self.graph, self.root_idx)
            .iter(&self.graph)
            .collect::<Vec<_>>();

        let (mut nodes, _edges) = self.graph.into_nodes_edges();

        order
            .into_iter()
            .map(move |i| nodes[i.index()].weight.take().expect("node taken twice"))
    }

    /// Return the list of directories in from-leaves-to-root order.
    #[instrument(level = "trace", skip_all)]
    pub fn drain_leaves_to_root(self) -> impl Iterator<Item = Directory> {
        // do a DFS Post-Order traversal of the graph, starting with the root node
        let order = DfsPostOrder::new(&self.graph, self.root_idx)
            .iter(&self.graph)
            .collect::<Vec<_>>();

        let (mut nodes, _edges) = self.graph.into_nodes_edges();

        order
            .into_iter()
            .map(move |i| nodes[i.index()].weight.take().expect("node taken twice"))
    }

    /// Returns the root [Directory].
    /// **Panics** if nothing has been inserted.
    #[instrument(level = "trace", skip_all)]
    pub fn root(&self) -> &Directory {
        self.graph
            .node_weight(self.root_idx)
            .expect("no root found")
            .as_ref()
            .expect("weight may not be none")
    }
}

#[cfg(test)]
mod tests {
    use crate::fixtures::{DIRECTORY_A, DIRECTORY_B, DIRECTORY_C};
    use crate::{Directory, Node};
    use rstest::rstest;
    use std::sync::LazyLock;

    use super::{DirectoryGraph, LeavesToRootValidator, RootToLeavesValidator};

    pub static BROKEN_PARENT_DIRECTORY: LazyLock<Directory> = LazyLock::new(|| {
        Directory::try_from_iter([(
            "foo".try_into().unwrap(),
            Node::Directory {
                digest: DIRECTORY_A.digest(),
                size: DIRECTORY_A.size() + 42, // wrong!
            },
        )])
        .unwrap()
    });

    #[rstest]
    /// Uploading an empty directory should succeed.
    #[case::empty_directory(&[&*DIRECTORY_A], false, Some(vec![&*DIRECTORY_A]))]
    /// Uploading A, then B (referring to A) should succeed.
    #[case::simple_closure(&[&*DIRECTORY_A, &*DIRECTORY_B], false, Some(vec![&*DIRECTORY_A, &*DIRECTORY_B]))]
    /// Uploading A, then A, then C (referring to A twice) should succeed.
    /// We pretend to be a dumb client not deduping directories.
    #[case::same_child(&[&*DIRECTORY_A, &*DIRECTORY_A, &*DIRECTORY_C], false, Some(vec![&*DIRECTORY_A, &*DIRECTORY_C]))]
    /// Uploading A, then C (referring to A twice) should succeed.
    #[case::same_child_dedup(&[&*DIRECTORY_A, &*DIRECTORY_C], false, Some(vec![&*DIRECTORY_A, &*DIRECTORY_C]))]
    /// Uploading A, then C (referring to A twice), then B (itself referring to A) should fail during close,
    /// as B itself would be left unconnected.
    #[case::unconnected_node(&[&*DIRECTORY_A, &*DIRECTORY_C, &*DIRECTORY_B], false, None)]
    /// Uploading B (referring to A) should fail immediately, because A was never uploaded.
    #[case::dangling_pointer(&[&*DIRECTORY_B], true, None)]
    /// Uploading a directory which refers to another Directory with a wrong size should fail.
    #[case::wrong_size_in_parent(&[&*DIRECTORY_A, &*BROKEN_PARENT_DIRECTORY], true, None)]
    fn test_uploads(
        #[case] directories_to_upload: &[&Directory],
        #[case] exp_fail_upload_last: bool,
        #[case] exp_finalize: Option<Vec<&Directory>>, // Some(_) if finalize successful, None if not.
    ) {
        let mut dcv = DirectoryGraph::<LeavesToRootValidator>::default();
        let len_directories_to_upload = directories_to_upload.len();

        for (i, d) in directories_to_upload.iter().enumerate() {
            let resp = dcv.add((*d).clone());
            if i == len_directories_to_upload - 1 && exp_fail_upload_last {
                assert!(resp.is_err(), "expect last put to fail");

                // We don't really care anymore what finalize() would return, as
                // the add() failed.
                return;
            } else {
                assert!(resp.is_ok(), "expect put to succeed");
            }
        }

        // everything was uploaded successfully. Test finalize().
        let resp = dcv
            .validate()
            .map(|validated| validated.drain_leaves_to_root().collect::<Vec<_>>());

        match exp_finalize {
            Some(directories) => {
                assert_eq!(
                    Vec::from_iter(directories.iter().map(|e| (*e).to_owned())),
                    resp.expect("drain should succeed")
                );
            }
            None => {
                resp.expect_err("drain should fail");
            }
        }
    }

    #[rstest]
    /// Downloading an empty directory should succeed.
    #[case::empty_directory(&*DIRECTORY_A, &[&*DIRECTORY_A], false, Some(vec![&*DIRECTORY_A]))]
    /// Downlading B, then A (referenced by B) should succeed.
    #[case::simple_closure(&*DIRECTORY_B, &[&*DIRECTORY_B, &*DIRECTORY_A], false, Some(vec![&*DIRECTORY_A, &*DIRECTORY_B]))]
    /// Downloading C (referring to A twice), then A should succeed.
    #[case::same_child_dedup(&*DIRECTORY_C, &[&*DIRECTORY_C, &*DIRECTORY_A], false, Some(vec![&*DIRECTORY_A, &*DIRECTORY_C]))]
    /// Downloading C, then B (both referring to A but not referring to each other) should fail immediately as B has no connection to C (the root)
    #[case::unconnected_node(&*DIRECTORY_C, &[&*DIRECTORY_C, &*DIRECTORY_B], true, None)]
    /// Downloading B (specified as the root) but receiving A instead should fail immediately, because A has no connection to B (the root).
    #[case::dangling_pointer(&*DIRECTORY_B, &[&*DIRECTORY_A], true, None)]
    /// Downloading a directory which refers to another Directory with a wrong size should fail.
    #[case::wrong_size_in_parent(&*BROKEN_PARENT_DIRECTORY, &[&*BROKEN_PARENT_DIRECTORY, &*DIRECTORY_A], true, None)]
    fn test_downloads(
        #[case] root: &Directory,
        #[case] directories_to_upload: &[&Directory],
        #[case] exp_fail_upload_last: bool,
        #[case] exp_finalize: Option<Vec<&Directory>>, // Some(_) if finalize successful, None if not.
    ) {
        let mut dcv =
            DirectoryGraph::with_order(RootToLeavesValidator::new_with_root_digest(root.digest()));
        let len_directories_to_upload = directories_to_upload.len();

        for (i, d) in directories_to_upload.iter().enumerate() {
            let resp = dcv.add((*d).clone());
            if i == len_directories_to_upload - 1 && exp_fail_upload_last {
                assert!(resp.is_err(), "expect last put to fail");

                // We don't really care anymore what finalize() would return, as
                // the add() failed.
                return;
            } else {
                assert!(resp.is_ok(), "expect put to succeed");
            }
        }

        // everything was uploaded successfully. Test finalize().
        let resp = dcv
            .validate()
            .map(|validated| validated.drain_leaves_to_root().collect::<Vec<_>>());

        match exp_finalize {
            Some(directories) => {
                assert_eq!(
                    Vec::from_iter(directories.iter().map(|e| (*e).to_owned())),
                    resp.expect("drain should succeed")
                );
            }
            None => {
                resp.expect_err("drain should fail");
            }
        }
    }
}
