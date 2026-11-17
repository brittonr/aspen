//! Untrusted executable payloads accompanying an ordinary typing candidate.
//!
//! Text bytes and definition bodies are retained separately from the kernel's
//! type-only literal representation. Consumers must validate these payloads and
//! independently check every body before emitting executable instructions.

/// One text literal's UTF-8 payload in its body's node arena.
#[derive(Clone, Debug)]
pub struct TextLiteral {
    pub node: crate::untrusted::NodeId,
    pub bytes: alloc::vec::Vec<u8>,
}

/// A finite typing candidate and its executable literal payloads.
#[derive(Clone, Debug)]
pub struct Body {
    pub candidate: crate::untrusted::Candidate,
    pub texts: alloc::vec::Vec<TextLiteral>,
}

/// One monomorphic specialization of an immutable resolved definition.
#[derive(Clone, Debug)]
pub struct Definition {
    /// Exact contract-table entry used by invocation nodes.
    pub definition: crate::contracts::Definition,
    /// Session-local immutable semantic identity, independent of source names.
    pub identity: u64,
    /// Untrusted original source identity before concrete adapter specialization.
    pub source_identity: u64,
    pub body: Body,
    pub expected: crate::untrusted::Expected,
    /// Untrusted source namespace row; sealed ingress authenticates it.
    pub owner_slot: Option<u32>,
    /// Untrusted concrete witness for the owner body's rank-1 binders. Only
    /// source-bound signed ingress may authorize it; kernel checks structure.
    pub owner_inst: Option<crate::words::Inst>,
}

/// A source submission ready for independent acceptance and lowering.
///
/// None of these public fields constitutes acceptance evidence. In particular,
/// a specialization's body must derive its advertised environment contract.
#[derive(Clone, Debug)]
pub struct Submission {
    pub environment: crate::contracts::Env,
    pub definitions: alloc::vec::Vec<Definition>,
    pub body: Body,
    pub request: crate::untrusted::Request,
}
