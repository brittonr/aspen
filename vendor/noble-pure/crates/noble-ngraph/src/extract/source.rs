//! Ordinary Core-Bootstrap source preparation for extraction subjects.
//!
//! This is the same public frontend the CLI session uses
//! (`noble_contracts::source::Session`), injected by the caller together
//! with its limits: definitions are prepared and committed one submission at
//! a time, then the root body is prepared with its ordered (bottom-first)
//! input stack types. The resulting [`noble_kernel::execution::Submission`] is untrusted producer
//! data; [`super::admit`] re-runs the kernel on it.

/// A prepared root submission and the producer's claimed closure (every
/// definition identity the frontend placed in the submission).
#[derive(Clone, Debug)]
pub struct Prepared {
    /// The root submission.
    pub submission: noble_kernel::execution::Submission,
    /// Producer-claimed closure (sorted, unique).
    pub claimed_closure: alloc::vec::Vec<u64>,
    /// Root stack arity before aggregate outputs are flattened into graph ports.
    pub root_stack_arity: usize,
}

/// Prepares `definitions` (each one `def NAME [ … ]` submission, committed in
/// order) and then `root` with `inputs`, in `session` under `limits`. Callers
/// pass a fresh `Session::new()` (the CLI's Core-Bootstrap namespace with the
/// `test.emit`/`test.abort` hosts) and `Limits::default()`; the session is
/// consumed. Every submission before the root must be a definition: an
/// executable submission would run in `noble session` but has no meaning
/// here.
///
/// # Errors
/// The frontend's refusal rendered as text.
pub fn prepare(
    mut session: noble_contracts::source::Session,
    limits: noble_contracts::Limits,
    definitions: &[&str],
    root: &str,
    inputs: &[noble_kernel::types::Ty],
) -> Result<Prepared, alloc::string::String> {
    for d in definitions {
        let p = session
            .prepare(d.as_bytes(), &[], limits)
            .map_err(|e| alloc::format!("definition {d:?}: {e:?}"))?;
        if p.definition_name().is_none() {
            return Err(alloc::format!(
                "submission {d:?} precedes the root but is not a definition"
            ));
        }
        session
            .commit(p)
            .map_err(|e| alloc::format!("definition {d:?}: {e:?}"))?;
    }
    let p = session
        .prepare(root.as_bytes(), inputs, limits)
        .map_err(|e| alloc::format!("root {root:?}: {e:?}"))?;
    let root_stack_arity = p.output().len();
    let submission = p
        .submission()
        .cloned()
        .ok_or_else(|| alloc::format!("root {root:?} produced no submission"))?;
    let mut claimed_closure: alloc::vec::Vec<u64> =
        submission.definitions.iter().map(|d| d.identity).collect();
    claimed_closure.sort_unstable();
    claimed_closure.dedup();
    Ok(Prepared {
        submission,
        claimed_closure,
        root_stack_arity,
    })
}

/// `n` `I64` input types.
#[must_use]
pub fn i64s(n: usize) -> alloc::vec::Vec<noble_kernel::types::Ty> {
    alloc::vec![noble_kernel::types::Ty::I64; n]
}
