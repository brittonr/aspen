//! External-environment-data validation walks (B-CHECK-02).
//!
//! Recursive definition dependencies and user-declared recursive schemas
//! are unsupported: both are rejected during preflight, before any
//! candidate body is checked, so external environment data never enters
//! checking unvalidated. Each walk is bounded, iterative, and charges the
//! declared work limit before every dependency edge and schema entry; no
//! step recurses and no loop body returns early.

/// The state one dependency walk threads through.
struct DepWalk {
    /// Per-definition color: 0 unvisited, 1 on the current path, 2 done.
    colors: alloc::vec::Vec<u8>,
    /// The depth-first stack of definitions with edges still pending.
    stack: alloc::vec::Vec<crate::contracts::Definition>,
    /// Work units spent on edges.
    spent: u32,
}

/// The dependency list of one definition, when it declares any.
fn deps_of(
    env: &crate::contracts::Env,
    def: crate::contracts::Definition,
) -> alloc::vec::Vec<crate::contracts::Definition> {
    match usize::try_from(def.0) {
        Ok(index) => match env.deps.get(index) {
            Some(list) => list.clone(),
            None => alloc::vec::Vec::new(),
        },
        Err(_) => alloc::vec::Vec::new(),
    }
}

/// Resolve a color only for a representable definition inside the arena.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; colored_slot uses TryFrom and Option::map for checked arena lookup, neither available here without experimental const traits; reassess when those operations become stable const."
)]
fn colored_slot(colors: &[u8], definition: crate::contracts::Definition) -> Option<(usize, u8)> {
    match usize::try_from(definition.0) {
        Ok(index) => colors.get(index).map(|color| (index, *color)),
        Err(_) => None,
    }
}

/// Validate the environment's definition dependencies (B-CHECK-02).
///
/// A definition that transitively depends on itself — directly or through
/// a chain — is rejected as unsupported, naming the definition the cycle
/// closes on. Every followed edge charges the declared work limit before
/// it is taken; a walk that would exceed it fails closed as exhausted. The
/// bootstrap table declares no dependencies, so its walk charges nothing.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; dependencies bounds root access by the color arena and propagates the DFS cycle/work outcomes; externally supplied dependency graphs must reject without assertion panics."
)]
pub(super) fn dependencies(
    env: &crate::contracts::Env,
    remaining: u32,
) -> Result<u32, super::Fail> {
    let count = env.defs.len();
    let mut walk = DepWalk {
        colors: alloc::vec![0; count],
        stack: alloc::vec::Vec::with_capacity(crate::capacity::at_least(count, 4)),
        spent: 0,
    };
    let mut failure: Option<super::Fail> = None;
    let mut root: usize = 0;
    while root < count {
        if walk.colors[root] == 0 {
            let (next, step) = dep_root(
                env,
                crate::contracts::Definition(root as u32),
                walk,
                remaining,
            );
            walk = next;
            match step {
                Ok(()) => root += 1,
                Err(problem) => failure = Some(problem),
            }
        } else {
            root += 1;
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => remaining
            .checked_sub(walk.spent)
            .ok_or(super::Fail::Exhausted(crate::untrusted::LimitKind::Work)),
    }
}

/// Run one depth-first walk from an unvisited root to completion.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; dep_root threads the DFS state until its stack empties or dep_step returns a typed failure; stack exhaustion and malformed graphs are handled as outcomes."
)]
fn dep_root(
    env: &crate::contracts::Env,
    root: crate::contracts::Definition,
    walk: DepWalk,
    remaining: u32,
) -> (DepWalk, Result<(), super::Fail>) {
    let mut walk = walk;
    walk.stack.push(root);
    let mut failure: Option<super::Fail> = None;
    while !walk.stack.is_empty() {
        let (next, step) = dep_step(env, walk, remaining);
        walk = next;
        match step {
            Ok(()) => {}
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => (walk, Err(problem)),
        None => (walk, Ok(())),
    }
}

/// Take one depth-first step over the stack top.
///
/// An unvisited definition turns on-path and enqueues every unvisited
/// successor; an edge to an already on-path definition closes a cycle and
/// rejects, naming that definition; a resurfaced on-path definition has
/// finished its successors and turns done. Every arena position passes
/// through on-path at most once, so the walk itself terminates; the work
/// limit is its fail-closed bound, charged per followed edge.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; dep_step checks definition/color membership, charges each edge before following it and returns RecursiveDependency or Work exhaustion; missing definitions remain sinks rather than panic conditions."
)]
fn dep_step(
    env: &crate::contracts::Env,
    mut walk: DepWalk,
    remaining: u32,
) -> (DepWalk, Result<(), super::Fail>) {
    let top = match walk.stack.last() {
        Some(top) => *top,
        None => return (walk, Ok(())),
    };
    let (index, color) = match colored_slot(&walk.colors, top) {
        Some(found) => found,
        None => {
            walk.stack.pop();
            return (walk, Ok(()));
        }
    };
    if color == 1 {
        // Resurfaced: every successor enqueued below has finished.
        // The color read above returned `Some`, so the position is inside.
        walk.colors[index] = 2;
        walk.stack.pop();
        return (walk, Ok(()));
    }
    if color == 2 {
        walk.stack.pop();
        return (walk, Ok(()));
    }
    // Newly reached: turn on-path, then charge and follow each edge.
    walk.colors[index] = 1;
    let deps = deps_of(env, top);
    let mut failure: Option<super::Fail> = None;
    let mut dep_index: usize = 0;
    while dep_index < deps.len() {
        if walk.spent >= remaining {
            failure = Some(super::Fail::Exhausted(crate::untrusted::LimitKind::Work));
            break;
        }
        walk.spent += 1;
        let dep = deps[dep_index];
        match colored_slot(&walk.colors, dep) {
            Some((_, 1)) => {
                failure = Some(super::Fail::Unsupported(
                    crate::untrusted::UnsupportedKind::RecursiveDependency(dep),
                ));
                break;
            }
            Some((_, 0)) => walk.stack.push(dep),
            // Missing definitions are sinks; completed definitions need no work.
            Some(_) | None => {}
        }
        dep_index += 1;
    }
    let outcome = match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    };
    (walk, outcome)
}

/// Validate the environment's user-declared schemas (B-CHECK-02).
///
/// A declaration that names itself is a user-declared recursive schema and
/// is rejected as unsupported, naming the declaration; a non-recursive
/// declaration still has to be a well-formed scheme. Each entry charges
/// the declared work limit before it is inspected; the bootstrap
/// environment declares none, so its scan charges nothing.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; schemas charges work before each declaration, then returns RecursiveSchema or SchemeForm in order; invalid external schemas must not trigger assertions."
)]
pub(super) fn schemas(env: &crate::contracts::Env, remaining: u32) -> Result<u32, super::Fail> {
    let mut spent: u32 = 0;
    let mut failure: Option<super::Fail> = None;
    let mut index: usize = 0;
    while index < env.schemas.len() {
        if spent >= remaining {
            failure = Some(super::Fail::Exhausted(crate::untrusted::LimitKind::Work));
            break;
        }
        spent += 1;
        let decl = &env.schemas[index];
        if decl.recursive {
            failure = Some(super::Fail::Unsupported(
                crate::untrusted::UnsupportedKind::RecursiveSchema(decl.id),
            ));
            break;
        }
        if decl.scheme.validate().is_err() {
            failure = Some(super::Fail::Unsupported(
                crate::untrusted::UnsupportedKind::SchemeForm,
            ));
            break;
        }
        index += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => remaining
            .checked_sub(spent)
            .ok_or(super::Fail::Exhausted(crate::untrusted::LimitKind::Work)),
    }
}
