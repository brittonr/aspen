use super::super::*;

pub(super) const ENTRY_COUNT: usize = 96;
pub(super) const VALUE_BYTES: usize = 80;
pub(super) const UPDATED_VALUE_BYTE: u8 = b'z';
pub(super) const ORIGINAL_VALUE_BYTE: u8 = b'v';
pub(super) const GIT_REVISION_HEX_CHARS: usize = 40;
pub(super) const POINT_INDEX: usize = 4;
pub(super) const RANGE_END_INDEX: usize = 8;
pub(super) const RANGE_EXPECTED_COUNT: usize = RANGE_END_INDEX - POINT_INDEX + 1;
pub(super) const UPDATE_INDEX: usize = ENTRY_COUNT / 2;
pub(super) const DELETE_INDEX: usize = UPDATE_INDEX + 1;
pub(super) const PROPERTY_ENTRY_COUNT: usize = 5;

pub(super) fn profile() -> ProllyProfile {
    standard_prolly_profile().expect("standard profile")
}

pub(super) fn entries() -> Vec<SemanticEntry> {
    (0..ENTRY_COUNT)
        .map(|index| SemanticEntry {
            key: format!("key-{index:04}").into_bytes(),
            value: vec![ORIGINAL_VALUE_BYTE; VALUE_BYTES],
        })
        .collect()
}

pub(super) fn entry(index: usize) -> SemanticEntry {
    entries().get(index).cloned().expect("entry index")
}

pub(super) fn build() -> MapBuild {
    build_map(&profile(), &entries()).expect("canonical build")
}

pub(super) fn text_entries() -> Vec<SemanticEntry> {
    vec![
        SemanticEntry {
            key: b"alpha".to_vec(),
            value: b"one".to_vec(),
        },
        SemanticEntry {
            key: b"beta".to_vec(),
            value: b"two".to_vec(),
        },
    ]
}
