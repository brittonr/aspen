# Preflight caller-built chunk manifest count

## Implemented and observed

- [x] [serial] Identify the parser-owned 65,536 chunk cap, public `ChunkManifest` fields, and both public readback/sync entry points without CLI production callsites. r[molten.chunk_store.caller_built_manifest_count_preflight]
- [x] [serial] Reproduce the 65,537-chunk remote-readback admission and traversal strategy-precedence failures before changing source. r[molten.chunk_store.caller_built_manifest_count_preflight]
- [x] [serial] Preflight the count at shared manifest identity admission, preserving exact-limit readback, admitted ref/strategy precedence, and existing receipt formats. r[molten.chunk_store.caller_built_manifest_count_preflight]
- [x] [serial] Confirm the original two over-limit regressions pass with the source guard and run scoped `cargo fmt --package molten --check`. r[molten.chunk_store.caller_built_manifest_count_preflight]
- [x] [serial] Exercise a compiled external API caller, including verified bytes and both over-limit denials; rerun focused tests after final precedence assertion. r[molten.chunk_store.caller_built_manifest_count_preflight]
- [x] [serial] Run scoped Clippy and Octet and strictly validate this change-local Cairn; retain explicit warning-only and full strict-ci limits. r[molten.chunk_store.caller_built_manifest_count_preflight]
- [x] [serial] Compose only this disjoint owner atop the detached 63-package integration target and repeat changed-path proof and Cairn validation. r[molten.chunk_store.caller_built_manifest_count_preflight]
- [x] [serial] Reproduce the 65,537 caller-supplied tuple bypass with tiny unused buffers before changing readback, retaining an exactly admitted 65,536-tuple case. r[molten.chunk_store.remote_supplied_chunk_preflight]
- [x] [serial] Check supplied tuple count before allocation, borrow refs/bytes in the ordered lookup, and cap required hashing plus appended output with the existing object-byte bound. r[molten.chunk_store.remote_supplied_chunk_preflight]
- [x] [serial] Confirm exact-bound, excess-tuple, admitted-range-error, and duplicate-last pass/deny byte behavior in all 23 focused chunk-store unit tests. r[molten.chunk_store.remote_supplied_chunk_preflight]
- [x] [serial] Compile and run a disposable external `molten::blocks` consumer proving duplicate-last deny/pass bytes and supplied tuple boundary admission/denial; remove temporary source afterward. r[molten.chunk_store.remote_supplied_chunk_preflight]
- [x] [serial] Mirror source, tests, docs and evidence into the 64-package composition; run final renamed-test filters, scoped formatting/Clippy/library Octet, and strict Cairn in both trees while preserving the unchanged full strict-ci gate. r[molten.chunk_store.remote_supplied_chunk_preflight]
