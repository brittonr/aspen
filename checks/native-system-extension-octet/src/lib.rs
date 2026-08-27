//! Focused strict-Octet compilation surface for the native system-extension core.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

pub mod preserves_rail {
    pub fn content_ref_from_bytes(bytes: &[u8]) -> String {
        format!("blake3:{}", blake3::hash(bytes).to_hex())
    }
}

pub mod system_extension;
