//! The id table: every id is unique, every constant name is unique, and every id has the form
//! `molten.<segment>(.<segment>)*.v<N>` with lowercase `[a-z0-9-]` segments.
//!
//! The table holds no intentional duplicates or aliases. An id deliberately named by two constants
//! must be listed in `INTENTIONAL_ALIASES` with the reason it exists.

/// Ids that two constants intentionally share. None today.
const INTENTIONAL_ALIASES: &[&str] = &[];

fn entries() -> impl Iterator<Item = (&'static str, &'static str)> {
    super::DOMAINS.iter().flat_map(|domain| domain.iter().copied())
}

/// The values that occur more than once, in sorted order.
fn repeated(mut values: alloc::vec::Vec<&'static str>) -> alloc::vec::Vec<&'static str> {
    values.sort_unstable();
    let mut repeats: alloc::vec::Vec<&str> =
        values.windows(2).filter(|pair| pair[0] == pair[1]).map(|pair| pair[0]).collect();
    repeats.dedup();
    repeats
}

fn is_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_version(segment: &str) -> bool {
    segment
        .strip_prefix('v')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

/// `molten.<segment>(.<segment>)*.v<N>`, also a stable id.
fn is_well_formed(id: &str) -> bool {
    let segments: alloc::vec::Vec<&str> = id.split('.').collect();
    let [first, middle @ .., version] = segments.as_slice() else {
        return false;
    };
    let has_shape = *first == "molten" && !middle.is_empty() && is_version(version);
    let has_segments = middle.iter().all(|segment| is_segment(segment));
    has_shape && has_segments && crate::view::validate_stable_id(id, "schema id").is_ok()
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn ids_are_unique_apart_from_listed_aliases() {
    assert_eq!(repeated(entries().map(|(_, id)| id).collect()), INTENTIONAL_ALIASES);
}

#[test]
fn constant_names_are_unique_and_name_ids() {
    let names: alloc::vec::Vec<&str> = entries().map(|(name, _)| name).collect();
    assert_eq!(repeated(names.clone()), alloc::vec::Vec::<&str>::new());
    let unnamed: alloc::vec::Vec<&str> = names.into_iter().filter(|name| !name.contains("_SCHEMA")).collect();
    assert_eq!(unnamed, alloc::vec::Vec::<&str>::new());
}

#[test]
fn ids_are_well_formed() {
    let malformed: alloc::vec::Vec<(&str, &str)> = entries().filter(|(_, id)| !is_well_formed(id)).collect();
    assert_eq!(malformed, alloc::vec::Vec::new());
}

#[test]
fn the_shape_check_rejects_malformed_ids_and_repeats_are_found() {
    for id in [
        "molten.harness.suite.v1",
        "molten.node.control-lock.v2",
        "molten.wasm.abi.v10",
    ] {
        assert!(is_well_formed(id), "{id} rejected");
    }
    for id in [
        "",
        "molten.v1",
        "aspen.harness.suite.v1",
        "molten.harness.suite",
        "molten.harness.suite.v",
        "molten.harness.suite.1",
        "molten.Harness.suite.v1",
        "molten.harness..suite.v1",
        "molten.harness.su ite.v1",
        "molten.harness.suite_x.v1",
    ] {
        assert!(!is_well_formed(id), "{id} accepted");
    }
    assert_eq!(repeated(alloc::vec!["b", "a", "b", "c", "b"]), ["b"]);
}

#[test]
fn constants_are_reachable_from_the_table_root() {
    assert_eq!(super::HARNESS_SUITE_SCHEMA, "molten.harness.suite.v1");
    assert_eq!(super::NODE_CONTROL_LOCK_SCHEMA_V2, "molten.node.control-lock.v2");
    assert_eq!(super::SERVICE_SUPERVISION_GATE_RECEIPT_SCHEMA, "molten.service.supervision-gate-receipt.v1");
}
