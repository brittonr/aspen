//! Focused strict-Octet compilation surface for world distribution.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod fabric {
    //! Production-identical mirror of the `molten-core` fabric helper the copied
    //! modules call (`crates/molten-core/src/fabric/mod.rs`).

    const BLAKE3_REF_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_CHAR_COUNT: usize = 64;
    const BLAKE3_REF_CHAR_COUNT: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_CHAR_COUNT;

    pub(crate) fn valid_blake3_ref(value: &str) -> bool {
        let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
            return false;
        };
        value.len() == BLAKE3_REF_CHAR_COUNT
            && hex.len() == BLAKE3_HEX_CHAR_COUNT
            && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    }
}

pub mod content_replication;
pub mod dag_sync;
pub mod world_commit;
pub mod world_distribution;
pub mod world_head;
