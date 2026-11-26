//! The bounded preorder walk.

enum Outcome<N> {
    Matched(super::TokenMatch),
    Children(alloc::vec::Vec<(super::PathSegment, N)>),
}

struct Budget {
    visited: u64,
    limits: super::TraversalLimits,
}

/// Open containers on the current path: the not yet visited children of each.
struct OpenFrames<N> {
    frames: alloc::vec::Vec<alloc::vec::IntoIter<(super::PathSegment, N)>>,
}

impl<N> OpenFrames<N> {
    /// Opens a container's children. The depth check in `visit` already admitted the container,
    /// so the stack never holds more frames than the depth budget.
    fn open(
        &mut self,
        children: alloc::vec::Vec<(super::PathSegment, N)>,
        limits: super::TraversalLimits,
    ) -> Result<(), super::TraversalError> {
        if children.is_empty() {
            return Ok(());
        }
        if !u64::try_from(self.frames.len()).is_ok_and(|open| open < limits.max_depth) {
            return Err(super::TraversalError::DepthExceeded {
                limit: limits.max_depth,
            });
        }
        self.frames.push(children.into_iter());
        Ok(())
    }
}

/// Returns the first token in preorder that `predicate` selects within `scope`.
///
/// The walk keeps an explicit stack of open containers. Each frame holds the not yet visited
/// children of one container on the current path, so the walk keeps the preorder, the first match,
/// and the first budget failure of a recursive walk.
// r[impl molten.runtime_spine.no_std_preserves_packed_kernel]
pub fn find_match<N, F>(
    root: N,
    scope: super::TokenScope,
    limits: super::TraversalLimits,
    mut predicate: F,
) -> Result<Option<super::TokenMatch>, super::TraversalError>
where
    N: super::TraversalNode,
    F: FnMut(super::TokenKind, &str) -> bool,
{
    let mut budget = Budget { visited: 0, limits };
    let mut path = alloc::vec![super::PathSegment::Root];
    let mut open = OpenFrames {
        frames: alloc::vec::Vec::new(),
    };
    match visit(&root, scope, &mut predicate, &mut budget, &path)? {
        Outcome::Matched(found) => return Ok(Some(found)),
        Outcome::Children(children) => open.open(children, limits)?,
    }
    while let Some(frame) = open.frames.last_mut() {
        let Some((segment, child)) = frame.next() else {
            open.frames.pop();
            path.pop();
            continue;
        };
        path.push(segment);
        match visit(&child, scope, &mut predicate, &mut budget, &path)? {
            Outcome::Matched(found) => return Ok(Some(found)),
            Outcome::Children(children) if children.is_empty() => {
                path.pop();
            }
            Outcome::Children(children) => open.open(children, limits)?,
        }
    }
    Ok(None)
}

/// Counts and bounds one node, applies the scope's predicates, and returns its children. Children
/// past the remaining node budget are not materialized, because reaching any of them would first
/// exceed the node budget.
fn visit<N, F>(
    node: &N,
    scope: super::TokenScope,
    predicate: &mut F,
    budget: &mut Budget,
    path: &[super::PathSegment],
) -> Result<Outcome<N>, super::TraversalError>
where
    N: super::TraversalNode,
    F: FnMut(super::TokenKind, &str) -> bool,
{
    budget.visited = budget.visited.checked_add(1).ok_or(super::TraversalError::NodeCountOverflow)?;
    if budget.visited > budget.limits.max_nodes {
        return Err(super::TraversalError::NodesExceeded {
            limit: budget.limits.max_nodes,
        });
    }
    if !u64::try_from(path.len()).is_ok_and(|depth| depth <= budget.limits.max_depth) {
        return Err(super::TraversalError::DepthExceeded {
            limit: budget.limits.max_depth,
        });
    }
    if let Some((kind, token)) = selected_token(node, scope, predicate) {
        return Ok(Outcome::Matched(super::TokenMatch {
            kind,
            token,
            path: path.to_vec(),
        }));
    }
    let child_budget = budget.limits.max_nodes.saturating_sub(budget.visited).saturating_add(1);
    Ok(Outcome::Children(node.children(child_budget)))
}

/// The token of `node` that the scope offers and the predicate selects, if any.
fn selected_token<N, F>(
    node: &N,
    scope: super::TokenScope,
    predicate: &mut F,
) -> Option<(super::TokenKind, alloc::string::String)>
where
    N: super::TraversalNode,
    F: FnMut(super::TokenKind, &str) -> bool,
{
    let mut offer = |kind: super::TokenKind, token: &str| {
        predicate(kind, token).then(|| (kind, alloc::string::String::from(token)))
    };
    match node.shape() {
        super::NodeShape::Record {
            symbol_label: Some(label),
        } if scope.record_labels => offer(super::TokenKind::RecordLabel, &label),
        super::NodeShape::Symbol(name) if scope.symbols => offer(super::TokenKind::Symbol, &name),
        super::NodeShape::String(text) => {
            let as_string = scope.strings.then(|| offer(super::TokenKind::String, &text)).flatten();
            as_string.or_else(|| {
                let is_ref = scope.content_refs && crate::identity::validate_ref(&text).is_ok();
                is_ref.then(|| offer(super::TokenKind::ContentRef, &text)).flatten()
            })
        }
        super::NodeShape::ByteString(bytes) if scope.byte_strings => {
            let reference = crate::identity::ContentRef::of_bytes(&bytes);
            offer(super::TokenKind::ByteString, reference.as_str())
        }
        super::NodeShape::Record { .. }
        | super::NodeShape::Symbol(_)
        | super::NodeShape::ByteString(_)
        | super::NodeShape::Other => None,
    }
}
