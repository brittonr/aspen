#![no_main]
//! Fuzz target: NAR archive ingestion.
//!
//! Feeds arbitrary bytes to snix-castore's NAR ingestion to find parsing
//! panics, hangs, or memory safety issues.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to parse data as a NAR archive.
    // The NAR format starts with "nix-archive-1\0".
    // We don't expect valid NARs from the fuzzer — we're looking for crashes.
    let cursor = std::io::Cursor::new(data);
    let _ = nix_compat::nar::reader::open(cursor);
});
