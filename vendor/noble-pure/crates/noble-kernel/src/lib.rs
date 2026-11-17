#![no_std]
//! Deterministic internal transitions. This crate does not evaluate Noble programs.

// The extraction model renames the constructors that would otherwise shadow
// Lean's own `Bool` and `Unit` inside the namespaces the translated methods
// live in. The attribute is inert for the Rust build and for the workspace
// tests; only Charon reads it.
#![feature(register_tool)]
#![register_tool(charon)]
#![register_tool(octet)]
#![register_tool(tigerstyle)]
// The call-budget module name is part of the kernel budget surface; a path
// rename would churn the extracted subject for a naming-discipline lint.
#![expect(
    tigerstyle::underscore_in_module_filename,
    reason = "the call-budget module name is part of the kernel budget surface and the extracted subject"
)]
// The kernel modules are cohesive units; splitting them to satisfy the line
// budget would change module paths and the extracted subject.
#![expect(
    tigerstyle::excessive_file_length,
    reason = "kernel modules are cohesive units; splitting them for a line budget would churn the extracted subject"
)]

extern crate alloc;

mod capacity;

/// Evaluate one fallible step, returning its failure from the enclosing function.
///
/// This stands in for the `?` operator: the Octet architecture collector marks
/// `Desugaring(QuestionMark)` as an unsupported expansion, while a crate-local
/// macro expansion resolves to this definition.
macro_rules! attempt {
    ($step:expr) => {
        match $step {
            Ok(value) => value,
            Err(failure) => return Err(failure),
        }
    };
}

pub mod acceptance;
pub mod async_tasks;
pub mod authority;
pub mod contracts;
pub mod dataspace;
pub mod execution;
pub mod integer;
pub mod resources;
pub mod shapes;
pub mod types;
pub mod untrusted;
pub mod words;

/// Outcome of consuming one unit from an explicitly supplied budget.
pub enum BudgetOutcome {
    Remaining(u32),
    Exhausted,
}

/// Consume one unit without underflow or an external effect.
// r[impl VT-M1-02]
pub fn consume_budget(remaining: u32) -> BudgetOutcome {
    match remaining.checked_sub(1) {
        Some(next) => BudgetOutcome::Remaining(next),
        None => BudgetOutcome::Exhausted,
    }
}
