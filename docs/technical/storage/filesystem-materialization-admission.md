# Filesystem materialization admission

Materialization connects a logical description of files to actual destination mutations. The difficult boundary is not writing bytes; it is ensuring that names, content, replacement policy, and destination authority remain connected through publication. This article assumes the [filesystem materialization contract](../../filesystem-materialization-authority.md) and explains its planning and shell phases. It is a [Technical companion](../README.md), not a stronger transactional or authorization specification.

## A plan is not a destination capability

`MaterializationPath` admits nonempty UTF-8 relative names with normal components, rejecting traversal, absolute names, platform prefixes, backslash aliases, empty components, trailing separators, NULs, and reserved staging names. This is intentionally stricter than merely joining a tar header to a destination string. The planner can reason about a canonical logical namespace without resolving any host path.

`MaterializationPolicy` records the workflow profile, replacement policy, and finite bounds on members, member bytes, aggregate bytes, and path length. The [plan definitions](../../../src/parts/materialization/p000/body.rs) distinguish member inputs, planned members, and payloads. According to the governing contract, planning validates all members before mutation, rejects duplicate logical paths, sorts by canonical path, and computes a Preserves-based BLAKE3 plan identity.

That identity binds logical content and policy, not the operator's host directory spelling. Two uses of the same plan may therefore describe identical intended bytes without sharing destination authority. The destination is separately represented by `MaterializationRoot`, whose acquired directory constrains effects. This is the same broad authority separation as [typed local stores](../../local-filesystem-authority.md), but materialization additionally defines staged publication and its own receipt.

## Staging links bytes to a plan

In the [stage and commit implementation](../../../src/parts/materialization/p002/body.rs), `stage` first validates the plan, validates payloads against it, computes the staging path, and performs staging. On staging failure it attempts cleanup and preserves cleanup failure in the returned error rather than reporting success.

The successful `StagedMaterialization` carries the root's shared identity, the plan ref, and the staging path. Staged files live beneath `.molten-materialize/<plan-hash>/tree` inside the destination capability. The contract requires size and BLAKE3 verification after writing. Staging is therefore not merely a temporary filename convention: it binds observed bytes to a previously admitted plan and to one acquired root.

Commit validates the plan again, rejects a stage from another root, rejects a mismatched plan ref, and preflights publication. Root equality here is process-local authority provenance, not canonical identity or serialized Rust layout. Passing this check says which acquired destination owns the stage; it does not identify the host directory in a receipt.

## Publication and failure handling

The default `NoReplace` policy denies an existing destination. `ReplaceRegularFiles` permits replacement only of regular files. Commit reobserves the target and, when replacement is allowed, moves the old regular file to an in-root backup. Links and special entries are not promoted to acceptable replacement targets.

Publication hard-links the verified staged file to the final path, then removes the staged link. The important distinction from a clobbering rename is that a newly appearing destination causes hard-link publication to fail rather than silently overwrite it. The implementation records publication states and routes errors through rollback helpers. After all publications it verifies destination members, removes staging state, and only then builds the receipt.

This is bounded error recovery around a sequence of filesystem effects. It is not a single atomic visibility event for all files, and rollback does not imply crash recovery. The governing contract explicitly excludes concurrent adversarial race freedom, distributed atomicity, crash consistency, and durability. The inspected implementation's propagation of cleanup errors also matters: an operation can fail after substantial filesystem work, so absence of a passing receipt is not proof that nothing changed.

## Illustrative two-file replacement

Consider an illustrative admitted plan for `manifest.pr` and `payload/data.bin`. Both payloads are verified in staging. Under `NoReplace`, an existing `manifest.pr` prevents successful publication; the mere equality of its bytes would not grant replacement permission.

Under a named replacement profile, an existing regular `manifest.pr` can move to backup before the staged version is linked into place. Suppose publication of `payload/data.bin` then fails. The recorded state enables rollback of earlier publication and restoration of the backup. The caller should treat the returned error as a failed materialization, not synthesize a receipt from the fact that one file was briefly visible.

A different failure occurs if the caller submits the stage to another `MaterializationRoot`, even one opened from the same textual pathname. Commit checks the acquired root identity, so textual coincidence does not transfer ownership. This illustrates why plan identity and destination capability are complementary rather than interchangeable.

## Verification and archive review

Suggested verification exercises duplicate paths, reserved staging paths, oversized members, wrong-root stages, stale plans, modified staged bytes, replacement denial, and injected partial-publication failure. The [existing materialization tests](../../../src/parts/materialization/tests/m000/p000/body.rs) include wrong-root, stale-plan, tampered-payload, and replacement scenarios. They are referenced, not reported as run here.

Archive review should follow the shared path and bound policy. The contract permits sorted regular-file writing and bounded streaming verification; it rejects links, devices, directories, traversal, and duplicate entries rather than calling a generic extraction API. Archive verification supplies payloads to the same planner, not a second ambient extraction mechanism.

## Receipt boundary

A passing materialization receipt records the plan, policy, authority kind, ordered member evidence, bytes, checks, and explicit non-claims. It attests to verified publication under the supplied capability. It does not authorize disclosure, establish provenance, validate artifact semantics, or approve a release. Those questions remain outside this filesystem admission boundary.

## Sources

- [Filesystem materialization authority](../../filesystem-materialization-authority.md)
- [Local filesystem authority](../../local-filesystem-authority.md)
- [Materialization policy and plan definitions](../../../src/parts/materialization/p000/body.rs)
- [Stage, commit, rollback routing, and receipt construction](../../../src/parts/materialization/p002/body.rs)
- [Materialization regression scenarios](../../../src/parts/materialization/tests/m000/p000/body.rs)
- [Technical companion](../README.md)
