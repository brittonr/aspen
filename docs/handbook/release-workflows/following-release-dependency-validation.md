# Following release dependency validation

Mode: Walkthrough

This walkthrough follows the checked-in `exact-pins.ncl` fixture through the release dependency shell and its pure validator. It is a source-checked path, not a report of an executed release check. No runtime verification was performed for this documentation batch. The available path is concrete; successfully materializing the pinned toolchain and upstream evidence remains an execution prerequisite.

Use the [dependency cohort companion](../../technical/engineering/dependency-cohorts-and-reproducible-builds.md) for the identity model. Here the task is narrower: identify exactly what a reviewer can observe at each stage, and avoid mistaking a profile export for repository validation.

## 1. Start with the actual positive fixture

The entire [positive fixture](../../../config/release-dependencies/fixtures/positive/exact-pins.ncl) imports `../../profile.ncl`. It does not contain a second set of independently maintained pins. Follow that import to the typed profile and its `contracts.ncl` contract before inspecting Cargo or Nix.

For a concrete row, `valence` is the manifest dependency key while `valence-core` is the package name. Its expected version is `0.1.0`, its Nix input is `valence-src`, and its revision is the shared `ValenceRevision` value. The canonical-Valence record uses that same revision. This explains why a reviewer must not compare only the manifest key with a lockfile package name.

**Observable boundary:** Nickel can export a contract-accepted profile. That proves neither that the checkout matches it nor that archive files are available.

## 2. Resolve shell inputs explicitly

The executable defaults to root `.` and profile `config/release-dependencies/profile.ncl`. A relative profile path is joined to the selected root. Evidence-source paths, however, are supplied directly to the observer. Use absolute paths when reviewing a checkout from another directory.

Source-checked, not executed: the package invocation follows the [governing workflow](../../reproducible-dependencies.md), and every application flag is declared in the [release-policy parser](../../../crates/molten-release-policy/src/main.rs). Set the variables to existing, reviewed checkouts; the guards intentionally provide no fallback evidence.

```sh
: "${RELEASE_ROOT:?Set the absolute reviewed Molten checkout path}"
: "${VALENCE_EVIDENCE_ROOT:?Set the absolute reviewed Valence evidence path}"
: "${OCTET_EVIDENCE_ROOT:?Set the absolute reviewed Octet evidence path}"
nix develop -c cargo run -p molten-release-policy -- \
  --root "$RELEASE_ROOT" \
  --evidence-source "valence-integrity=$VALENCE_EVIDENCE_ROOT" \
  --evidence-source "octet-cutover=$OCTET_EVIDENCE_ROOT"
```

Run from the repository root so the Nix shell and Cargo workspace are the intended ones. The shell starts Nickel itself. If any explicit evidence source is supplied, the parser does not fill in the other default source automatically. Both configured IDs are needed for this profile.

## 3. Observe the repository without promoting paths into identity

The observer parses root `Cargo.toml`, `Cargo.lock`, and `flake.lock`. It compares the profile's dependency rows with manifest source/revision fields, resolved Git identities, and locked Nix revisions. It also collects unprofiled direct Git declarations and graph identities used by the pure checks.

The Valence archive root must contain the configured `cairn/archive/2026-07-12-harden-preserves-integrity-boundaries` directory and its `tasks.md`. The Octet root contributes the archived cutover tasks and `standards/valence-core-cutover/generated-manifest.json`. Each configured file is hashed, rather than accepted because its filename looks right. Files larger than 1,048,576 bytes cannot produce a hash observation through this helper.

Distribution evidence has a different boundary: the observer checks whether configured notice and source-export paths are files. It does not establish the legal adequacy or completeness of a distribution.

## 4. Cross into the pure decision

The shell constructs `ReleaseDependencyInput`; the core validates dependency rows, graph identities, canonical Valence, archive expectations, and distribution observations. The core does not fetch dependencies or read files. Diagnostics are sorted and deduplicated before report material is assembled.

**Observable boundary:** a decision concerns the supplied observations and profile. It is not a build, an upstream correctness proof, or deployment authority. The report material is sorted textual material in this particular validator; the shell hashes its bytes with BLAKE3. Do not relabel that hash as a canonical Preserves receipt or as the identity of an output binary.

## 5. Read the real output contract

On a valid report, the executable prints a decision summary with row/archive counts and `report_blake3`. On a validation denial, it prints the report hash and diagnostics through its error path and exits unsuccessfully. Earlier failures, such as Nickel rejection or unreadable repository files, can occur before a report exists. There is no `--out` flag on this executable and no automatic release receipt export.

The flake's `release-dependency-profile` check runs this shell with exact Nix evidence inputs, stores its stdout in the derivation output, exports the positive fixture, and requires negative Nickel fixtures to reject. These are separate observations within one rail.

## 6. Follow one expected failure

The checked-in `floating-revision.ncl` fixture sets `immutable_revision` to `main`. Its intended denial is a Nickel contract boundary, before repository observation. A stale locked Nix revision is instead a core `NixDrift` case. Do not repair either by overriding an input and calling the resulting development build release evidence. Follow the coordinated pin-update procedure, preserve the failed evidence, and rerun only after the relevant reviewed inputs have changed.

## Sources

- [Handbook](../README.md)
- [Reproducible release dependencies](../../reproducible-dependencies.md)
- [Dependency cohort companion](../../technical/engineering/dependency-cohorts-and-reproducible-builds.md)
- [Profile and archive expectations](../../../config/release-dependencies/profile.ncl)
- [Floating-revision negative fixture](../../../config/release-dependencies/fixtures/negative/floating-revision.ncl)
- [Profile export shell](../../../crates/molten-release-policy/src/profile.rs)
- [Repository observer](../../../crates/molten-release-policy/src/observe.rs)
- [Pure validator and report material](../../../crates/molten-core/src/release_dependency.rs)
- [Flake release rails](../../../flake.nix)
