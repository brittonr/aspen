//! Meter forward dependencies while scheduling module definitions.

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite."
)]
pub(super) fn blocked(
    tree: &crate::source::Tree,
    names: &[alloc::string::String],
    installed: &[bool],
    signed_owner: Option<&str>,
    mut meter: crate::Meter,
) -> (crate::Meter, Result<bool, crate::source::Error>) {
    let mut node_at = 0;
    let mut is_blocked = false;
    let mut failure = None;
    while node_at < tree.nodes.len() && !is_blocked && failure.is_none() {
        let (next, result) =
            blocked_at(&tree.nodes[node_at], names, installed, signed_owner, meter);
        meter = next;
        match result {
            Ok(blocked) => is_blocked = blocked,
            Err(problem) => failure = Some(problem),
        }
        node_at += 1;
    }
    let result = match failure {
        Some(problem) => Err(problem),
        None => Ok(is_blocked),
    };
    (meter, result)
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 dependency, E0277/E0658 Vec deref and E0493 metered result prevent const on both pinned Rust compilers; reassess const graph traversal."
)]
fn blocked_at(
    node: &crate::source::Node,
    names: &[alloc::string::String],
    installed: &[bool],
    signed_owner: Option<&str>,
    meter: crate::Meter,
) -> (crate::Meter, Result<bool, crate::source::Error>) {
    let (meter, charged) = charge_dependency(meter);
    if let Err(problem) = charged {
        return (meter, Err(problem));
    }
    if let crate::source::Kind::Word(word) = &node.kind {
        let (meter, found) = dependency(word, names, meter);
        return match found {
            Ok(Some(index)) => (
                meter,
                Ok(!installed[index] && signed_owner != Some(names[index].as_str())),
            ),
            Ok(None) => (meter, Ok(false)),
            Err(problem) => (meter, Err(problem)),
        };
    }
    (meter, Ok(false))
}

fn dependency(
    word: &[u8],
    names: &[alloc::string::String],
    mut meter: crate::Meter,
) -> (crate::Meter, Result<Option<usize>, crate::source::Error>) {
    let mut index = 0;
    let mut found = None;
    let mut failure = None;
    while index < names.len() && found.is_none() && failure.is_none() {
        let (next, matches) = checked_match(word, &names[index], meter);
        meter = next;
        match matches {
            Ok(true) => found = Some(index),
            Ok(false) => {}
            Err(problem) => failure = Some(problem),
        }
        index += 1;
    }
    match failure {
        Some(problem) => (meter, Err(problem)),
        None => (meter, Ok(found)),
    }
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 charge_dependency and E0658 slice PartialEq prevent const on both pinned Rust compilers; reassess const metering and equality."
)]
fn checked_match(
    word: &[u8],
    name: &str,
    meter: crate::Meter,
) -> (crate::Meter, Result<bool, crate::source::Error>) {
    let (meter, charged) = charge_dependency(meter);
    let matches = match charged {
        Ok(()) => Ok(name.as_bytes() == word),
        Err(problem) => Err(problem),
    };
    (meter, matches)
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 Meter::charge and declared::diagnostic plus E0493 charged result prevent const on both pinned Rust compilers; reassess const diagnostics."
)]
fn charge_dependency(mut meter: crate::Meter) -> (crate::Meter, Result<(), crate::source::Error>) {
    let result = match meter.charge(1, crate::source::declared::SPAN) {
        Ok(()) => Ok(()),
        Err(problem) => Err(crate::source::declared::diagnostic(
            crate::source::Stage::Check,
            problem,
        )),
    };
    (meter, result)
}
