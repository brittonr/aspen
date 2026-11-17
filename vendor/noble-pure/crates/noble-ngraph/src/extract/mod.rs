//! D7 admission and stack-to-graph extraction for `ngraph-i64/0`.
//!
//! The pipeline runs strictly in stage order and is a consumer of public
//! kernel data only:
//!
//! 1. [`admit`]: the exact subject (a [`Submission`]) is independently
//!    re-accepted by `noble_kernel::acceptance::check` — the root body under
//!    its request and every definition specialization under its own expected
//!    interface — with every arena node covered by a derivation
//!    (`E-SUBJECT`); the exact named-call closure is recomputed from resolved
//!    definition identities — live-slot dispatch has none (`E-LIVE-SLOT`),
//!    an identity reached again below itself is `E-RECURSION`, and each
//!    definition's supplied `Env::deps` list and the caller's claim must
//!    match it (`E-CLOSURE`); the root interface must be `I64`/`Bool` scalars, `Unit`
//!    or pairs of them (`E-RESOURCE`, `E-DYNAMIC-RUN`, `E-PROGRAM-OUTPUT`,
//!    `E-AGGREGATE`); the root effect bound must be empty (`E-EFFECT`).
//! 2. [`extract`]: symbolic stack execution over typed value ids. Node
//!    interfaces come only from the kernel's checked derivations (looked up
//!    with the `derivation()` consistency rule of the Wasm planner), never
//!    from Wasm-plan signatures or producer metadata (V-CHECK-01). Every
//!    processed step charges work units, every created node charges the
//!    **unpruned** expansion budget and every `run`/`dip`/named-call
//!    expansion charges inline depth.
//! 3. Normalization: pruning of nodes that reach no output (`ngraph-i64/0`
//!    extraction creates no guards and no facts), then the independent graph
//!    checker ([`crate::check`]). The named-call expansion bound is computed
//!    here, from the closure call graph plus one node per scalar leaf of the
//!    root inputs; the checker does not recompute it.
//!
//! Every refusal is a [`Diagnostic`] returned before any graph exists, so no
//! graph node is ever evaluated for a refused subject.
//!
//! Trust: unpruned `compose`/`run` expansion accounting stays inside the
//! extractor until NG-PO-A is discharged (D13). Extraction does not prove
//! that the graph preserves the ordinary observation; Phase B B2 only tests
//! it on finite vectors.
//!
//! [`Submission`]: noble_kernel::execution::Submission
//! [`Diagnostic`]: crate::diag::Diagnostic

mod admission;
pub mod source;
mod symbolic;

pub use admission::{admit, admit_with_tail, Admitted, Request};
pub use symbolic::{extract, Extraction, Stats};

/// Extraction budgets (D5 `/0` defaults; a request may only lower them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Extraction work units (symbolic steps, including repeated expansions).
    pub work_units: u64,
    /// Expanded nodes before pruning, CSE or fusion.
    pub expanded_nodes: u64,
    /// Inline depth (named call, `run`, `dip` nesting).
    pub inline_depth: u64,
}

impl Default for Budget {
    fn default() -> Self {
        let d = crate::check::Budgets::default();
        Self {
            work_units: d.work_units,
            expanded_nodes: d.expanded_nodes,
            inline_depth: d.inline_depth,
        }
    }
}
