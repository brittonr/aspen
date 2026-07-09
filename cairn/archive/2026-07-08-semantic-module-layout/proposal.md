## Why

Molten has many large implementation surfaces split through ordinal `include!` shards such as `parts/.../pNNN/body.rs`. That reduces physical file size, but it does not give reviewers a semantic map of the subsystem. The result is a source tree where module boundaries are hard to reason about, ownership is unclear, and future extractions into crates become risky.

## What Changes

- Introduce a semantic module layout for high-pressure included modules.
- Convert selected ordinal shards into named modules that describe model, parsing, validation, admission, receipts, storage, and shell concerns.
- Preserve behavior and public compatibility exports while reducing hidden coupling.
- Record any generated-code or staged-migration exemptions explicitly.

## Impact

This is an architecture-only modularity change. It should make review and future crate extraction easier without changing canonical receipts, Preserves bytes, CLI output contracts, or runtime admission behavior.
