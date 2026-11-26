//! Prolly map: history independence, bounded nodes, exact readback, and incremental/reference
//! agreement.

type State = std::collections::BTreeMap<Vec<u8>, Vec<u8>>;

const MAX_ENTRIES: usize = 48;
const MAX_KEY_BYTES: usize = 6;
const MAX_VALUE_BYTES: usize = 96;
const KEY_ALPHABET_LAST: u8 = b'h';

fn draw_state(tc: &hegel::TestCase) -> State {
    let key = || {
        hegel::generators::vecs(hegel::generators::integers::<u8>().min_value(b'a').max_value(KEY_ALPHABET_LAST))
            .min_size(1)
            .max_size(MAX_KEY_BYTES)
    };
    let count = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(MAX_ENTRIES));
    (0..count)
        .map(|_| (tc.draw(key()), tc.draw(hegel::generators::binary().max_size(MAX_VALUE_BYTES))))
        .collect()
}

fn entry(key: &[u8], value: &[u8]) -> molten_core::prolly_map::SemanticEntry {
    molten_core::prolly_map::SemanticEntry {
        key: key.to_vec(),
        value: value.to_vec(),
    }
}

fn entries(state: &State) -> Vec<molten_core::prolly_map::SemanticEntry> {
    state.iter().map(|(key, value)| entry(key, value)).collect()
}

fn snapshot(
    profile: &molten_core::prolly_map::ProllyProfile,
    state: &State,
) -> Option<molten_core::prolly_map::MapSnapshot> {
    molten_core::prolly_map::build_map(profile, &entries(state)).ok().map(|build| build.snapshot)
}

fn shuffle<T>(tc: &hegel::TestCase, items: &mut [T]) {
    for index in (1..items.len()).rev() {
        let other = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(index));
        items.swap(index, other);
    }
}

/// The edits that turn `from` into `to`, in an arbitrary drawn order.
fn transition_edits(tc: &hegel::TestCase, from: &State, to: &State) -> Vec<molten_core::prolly_map::MapEdit> {
    let upserts = to.iter().filter_map(|(key, value)| match from.get(key) {
        None => Some(molten_core::prolly_map::MapEdit::Insert(entry(key, value))),
        Some(previous) if previous != value => Some(molten_core::prolly_map::MapEdit::Update(entry(key, value))),
        Some(_) => None,
    });
    let deletes = from
        .keys()
        .filter(|key| !to.contains_key(*key))
        .cloned()
        .map(molten_core::prolly_map::MapEdit::Delete);
    let mut edits = upserts.chain(deletes).collect::<Vec<_>>();
    shuffle(tc, &mut edits);
    edits
}

// The root depends only on the final key/value state: any edit order from any starting state
// reaches the canonical root of a fresh build, and the incremental planner agrees with the
// reference planner.
#[hegel::test(test_cases = 60, derandomize = true, database = None)]
fn hegel_prolly_root_is_history_independent_and_matches_the_reference(tc: hegel::TestCase) {
    let profile = molten_core::prolly_map::standard_prolly_profile();
    assert!(profile.is_ok(), "the standard profile must validate");
    let Ok(profile) = profile else { return };
    let start = draw_state(&tc);
    let target = draw_state(&tc);
    let edits = transition_edits(&tc, &start, &target);
    let from = snapshot(&profile, &start);
    let expected = snapshot(&profile, &target);
    assert!(from.is_some(), "the drawn start state must build");
    assert!(expected.is_some(), "the drawn target state must build");
    let (Some(from), Some(expected)) = (from, expected) else {
        return;
    };

    let incremental = molten_core::prolly_map::plan_edits(&profile, &from, &edits).ok().map(|plan| plan.next.snapshot);
    let reference = molten_core::prolly_map::plan_edits_reference(&profile, &from, &edits)
        .ok()
        .map(|plan| plan.next.snapshot);
    assert_eq!(
        incremental.as_ref().map(|next| &next.root.root_ref),
        Some(&expected.root.root_ref),
        "edit history leaked into the root"
    );
    assert_eq!(reference.as_ref().map(|next| &next.root.root_ref), Some(&expected.root.root_ref));
    assert_eq!(incremental.map(|next| next.blocks), Some(expected.blocks), "equal states must share every block");
}

// Every encoded node respects the profile bound, and readback returns exactly the sorted state.
#[hegel::test(test_cases = 60, derandomize = true, database = None)]
fn hegel_prolly_nodes_are_bounded_and_read_back_exactly(tc: hegel::TestCase) {
    let profile = molten_core::prolly_map::standard_prolly_profile();
    assert!(profile.is_ok(), "the standard profile must validate");
    let Ok(profile) = profile else { return };
    let state = draw_state(&tc);
    let built = snapshot(&profile, &state);
    assert!(built.is_some(), "drawn state must build");
    let Some(built) = built else { return };
    assert!(
        built
            .blocks
            .iter()
            .all(|block| u32::try_from(block.bytes.len()).is_ok_and(|size| size <= profile.max_node_bytes))
    );
    let read = molten_core::prolly_map::validate_snapshot(&profile, &built).ok();
    assert_eq!(read.as_ref().map(|result| &result.entries), Some(&entries(&state)));
    assert_eq!(read.map(|result| result.root_ref), Some(built.root.root_ref));
}

// Mutating any single stored block is detected: readback of a tampered snapshot never succeeds.
#[hegel::test(test_cases = 40, derandomize = true, database = None)]
fn hegel_prolly_tampered_blocks_never_validate(tc: hegel::TestCase) {
    let profile = molten_core::prolly_map::standard_prolly_profile();
    assert!(profile.is_ok(), "the standard profile must validate");
    let Ok(profile) = profile else { return };
    let mut state = draw_state(&tc);
    state.entry(b"anchor".to_vec()).or_insert_with(|| b"value".to_vec());
    let built = snapshot(&profile, &state);
    assert!(built.is_some(), "drawn state must build");
    let Some(mut tampered) = built else { return };
    let block_index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(tampered.blocks.len() - 1));
    let block = &mut tampered.blocks[block_index];
    let byte_index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(block.bytes.len() - 1));
    let flip = tc.draw(hegel::generators::integers::<u8>().min_value(1).max_value(u8::MAX));
    block.bytes[byte_index] ^= flip;
    assert!(
        molten_core::prolly_map::validate_snapshot(&profile, &tampered).is_err(),
        "tampered block {block_index} byte {byte_index} validated"
    );
}
