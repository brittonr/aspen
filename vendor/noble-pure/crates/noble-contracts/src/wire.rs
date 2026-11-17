//! Versioned semantic projection of accepted candidate nodes.
//!
//! Instantiation witnesses establish typing; they are retained in Prepared and
//! do not alter execution. This projection keeps every executable word, literal,
//! quotation edge and body order. Its generated functions are the PO-19 bridge.

/// A node in the pure behavioral model, indexed in the original arena.
#[derive(Clone)]
pub enum SemanticNode {
    I64(i64),
    Boolean(bool),
    UnitValue,
    Word(u32),
    Quotation(alloc::vec::Vec<u32>),
}

/// Explicit reasons why a kernel candidate has no MC1 semantic projection.
#[derive(Clone, Copy)]
pub enum ProjectionError {
    Revision,
    PayloadlessText,
    HostWord,
    LiveSlot,
    SignedSelf,
}

/// The executable graph plus entry order. No producer acceptance flag exists.
pub struct SemanticSubject {
    pub nodes: alloc::vec::Vec<SemanticNode>,
    pub body: alloc::vec::Vec<u32>,
}

/// Preserve the ordered node references without interpretation or substitution.
pub fn lower_body(body: &[noble_kernel::untrusted::NodeId]) -> alloc::vec::Vec<u32> {
    let mut result = alloc::vec::Vec::with_capacity(body.len());
    let mut index = 0usize;
    while index < body.len() {
        result.push(body[index].0);
        index += 1;
    }
    result
}

/// Project one node into the reviewed pure normal-return model.
pub fn lower_node(node: &noble_kernel::untrusted::Node) -> Result<SemanticNode, ProjectionError> {
    match node {
        noble_kernel::untrusted::Node::Literal { lit, .. } => match lit {
            noble_kernel::untrusted::Lit::I64(n) => Ok(SemanticNode::I64(*n)),
            noble_kernel::untrusted::Lit::Bool(b) => Ok(SemanticNode::Boolean(*b)),
            noble_kernel::untrusted::Lit::Unit => Ok(SemanticNode::UnitValue),
            noble_kernel::untrusted::Lit::Text => Err(ProjectionError::PayloadlessText),
        },
        noble_kernel::untrusted::Node::Invocation { def, .. } => {
            if def.0 < 22 {
                Ok(SemanticNode::Word(def.0))
            } else {
                Err(ProjectionError::HostWord)
            }
        }
        noble_kernel::untrusted::Node::Quotation { body, .. } => {
            Ok(SemanticNode::Quotation(lower_body(body)))
        }
        noble_kernel::untrusted::Node::SlotInvoke { .. } => Err(ProjectionError::LiveSlot),
        noble_kernel::untrusted::Node::SignedSelf { .. } => Err(ProjectionError::SignedSelf),
    }
}

/// Project the exact retained graph, refusing foreign semantics or word ids.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; revision and every semantic node are checked through ProjectionError, preserving rejection rather than asserting over an external candidate."
)]
pub fn lower_subject(
    candidate: &noble_kernel::untrusted::Candidate,
) -> Result<SemanticSubject, ProjectionError> {
    if candidate.format != noble_kernel::untrusted::CANDIDATE_FORMAT
        || candidate.revision != noble_kernel::untrusted::SEMANTIC_REVISION
    {
        return Err(ProjectionError::Revision);
    }
    let mut nodes = alloc::vec::Vec::with_capacity(candidate.nodes.len());
    let mut index = 0usize;
    let mut failure = None;
    while index < candidate.nodes.len() {
        match lower_node(&candidate.nodes[index]) {
            Ok(node) => nodes.push(node),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
        index += 1;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(SemanticSubject {
        nodes,
        body: lower_body(&candidate.body),
    })
}
