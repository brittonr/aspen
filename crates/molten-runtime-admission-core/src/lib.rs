#![cfg_attr(not(test), no_std)]

//! Pure runtime capability and deny-rule admission.
//!
//! `ingress` evaluates capability grants and deny rules over requests generic in the runtime
//! value. A shape-valid value never grants authority, and the crate links no Preserves,
//! filesystem, clock, or network code: the `molten` shell (`src/runtime/admission/`) binds its
//! canonical `RuntimeValue` and projects runtime steps into requests. The nominal reference
//! grammar stays in `molten_core::nominal`, where Octet's nominal-domain catalog pins it, and the
//! wire-to-core boundary stays in `src/authority/nominal.rs`.
// r[impl molten.project.tier1_no_std_kernel_crates]

extern crate alloc;

pub mod ingress;
