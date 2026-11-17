#![no_std]
#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![expect(
    tigerstyle::module_file_count,
    reason = "Owner: noble-maintainers; the crate modules follow the D1-D9 design sections rather than an arbitrary file count"
)]
#![expect(
    tigerstyle::platform_dependent_cast,
    reason = "Owner: noble-maintainers; noble-ngraph supports only targets with usize >= 32 bits (x86_64, aarch64, wasm32); every flagged cast is a u32 index converted to usize"
)]
#![expect(
    tigerstyle::excessive_file_length,
    reason = "Owner: noble-maintainers; each D1-D9 checker/codec file is a cohesive review unit and splitting it would churn canonical paths and the extracted subject"
)]
#![expect(
    tigerstyle::function_length,
    reason = "Owner: noble-maintainers; checker, parser and evaluator functions are kept linear so their rule order remains reviewable in one place"
)]
#![expect(
    tigerstyle::usize_in_public_api,
    reason = "Owner: noble-maintainers; host-side slice/row lengths use usize and are not part of the guest Wasm ABI"
)]
#![expect(
    tigerstyle::sentinel_fallback,
    reason = "Owner: noble-maintainers; GraphBuilder converts producer-owned node/slot counts and u32::MAX is an unrepresentable marker that graph admission refuses"
)]
#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; every &mut parameter is a caller-owned explicit workspace, builder or output buffer; the crate has no ambient state, I/O or hidden mutation"
)]
#![expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; every operand is an admitted count, index or measured bound constrained by the checker's explicit node/layer/byte limits; overflow is refused at admission and wrapping arithmetic is written with wrapping_*"
)]
#![expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the checker, codec and fixture helpers are ordered validators; their explicit assertions and refusals are the review subject, and splitting them would hide the check order"
)]
#![expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; the semantic enums are closed vocabularies whose new variants must force every match and format site to change"
)]
#![expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; helper constness is not part of the crate's stability contract; the flagged helpers read runtime tables or allocate while preserving diagnostics"
)]
#![expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; the flagged names are the published mathematical vocabulary (left/right, min/max, limit) and every call site already fixes their order in the reviewed formulas"
)]
//! `Numeric-Graph-v1`: the pure library of the opt-in numeric graph profile
//! (`NumericGraph-Design`, design changes `numeric-graph-profile` and
//! `numeric-graph-v1`).
//!
//! `F32` graphs are authored directly in `ngraph-ir/0` and admitted only by
//! [`check`] (option B0(b)); source extraction exists for `I64` only. The
//! crate is `no_std` + `alloc`, executes no filesystem, process, environment,
//! clock, thread or output effect, and depends only on `noble-kernel` and
//! `noble-contracts`. Identity hashing is injected ([`identity`]).
//!
//! Non-claims: finite tests and evaluator agreement do not prove the
//! extractor (NG-PO-A), the checker (NG-PO-B), the evaluators (NG-PO-C), the
//! binary32 model (NG-PO-D) or any other NG-PO obligation. No performance
//! claim is made.

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod batch;
pub mod belt;
pub mod cert;
#[path = "cert/scoped.rs"]
pub mod cert_scoped;
pub mod check;
pub mod compiled;
pub mod diag;
pub mod eval;
pub mod extract;
pub mod facts;
pub mod format;
pub mod grad;
pub mod identity;
pub mod ir;
pub mod layout;
pub mod llvm;
pub mod net;
pub mod prng;
pub mod resource;
pub mod schedule;
pub mod sem;
pub mod snapshot;
pub mod specialize;
pub mod train;

pub use batch::{declared_row_work, evaluate, evaluate_into, Row, RowOp};
pub use check::{check, check_with, Budgets, CheckedGraph};
pub use diag::{BudgetKind, Code, Diagnostic, Malformed, Outcome, Site};
pub use eval::{reference, Control, Obs, Report};
pub use identity::{IdentityHasher, IdentityScheme};
pub use ir::{Graph, GraphBuilder, NodeId, Op, ScalarType, SubProfile};
pub use llvm::{Instruction, Module, Opcode, Operand, Refusal};
pub use sem::Value;
pub use specialize::{
    emit_source, interface_types, merge, rank, readmit, specialize, Residual, ACCOUNTING_REVISION,
};
