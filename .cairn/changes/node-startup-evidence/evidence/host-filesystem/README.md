# First node-host source cleanup

## Scope and ownership

Implementation range: `ff9d0a81441a7fa9fcb80e3e35d60178a670ab9f` through `22f3d3a5ebb53050848c6648dfc596481e9e8b12`.
The baseline is `495145226eda8810846703abb45036640ebfcf12`.

This batch changes only the node-host filesystem adapter and its tests.
It replaces non-trait imports with qualified owner paths.
The bounded reader now consumes the existing `NodeStateFile` observation rather than separate file and size arguments.
Its owner is `crates/molten-node-host/src/node/state/filesystem/read.rs`.

The metadata adapter owns the repeated regular-file checks before and after handle acquisition.
The existing classifier maps `file_type.is_file()` directly to `RegularFile`.
The shared check preserves that predicate and its error messages.
The optional directory walker retains no-follow behavior and missing-parent results.

No domain policy moves into the adapter. No public method changes its signature.
No assertions, catch-all enum arms, lint suppressions, or weaker flags were added to production code.
The capability root, operation order, byte bounds, error results, and retained-handle semantics remain unchanged.

## Tests

The initial baseline attempt failed because its restricted PATH omitted the existing `cc` linker.
Task 10766 repeated the seven baseline tests against the retained frozen source with the linker available. All seven passed.
No compiler was built.

Task 10774 passed nine tests and package-scoped all-target Clippy with `-D warnings` after the first implementation.
Tasks 10782, 10788, and 10795 each passed twelve tests and the same Clippy command after refinement.
The tests use the existing Rust 1.97.1 toolchain with `RUSTC_BOOTSTRAP=1`, not the March-21 lint compiler or May-26 production compiler.

Five new tests cover:

1. Zero, exact, excessive, and hard read bounds.
2. File growth after observation, without trusting a stale observed size.
3. Regular writes, non-truncating database opens, and directory denial.
4. Missing-parent observation without directory creation.
5. Symlink leaf and parent denial without target mutation (Unix).

The existing seven tests cover atomic leaf visibility, concurrent readers, dependency boundaries, and invalid locators.
These tests perform only local filesystem effects. They start no host listener or VM.
Selected March-21 rustfmt and Git whitespace checks passed.

## Retained intermediate results

Task 10773 exposed an explicit-module-path error after the first extraction.
The filesystem owner uses a `#[path]` declaration, so the child also needs an explicit path. The correction precedes task 10774.

Task 10776 ran the unchanged full command on frozen commit `ff9d0a81441a7fa9fcb80e3e35d60178a670ab9f`.
It reported 98 node-host errors, compared with 106 in the baseline.
This intermediate result included new line-density and underscore-filename findings.
The refinement splits metadata admission, directory walking, bound admission, and handle consumption into coherent functions.
It does not add meaningless assertions or change the lint policy.

Task 10785 reported 93 errors on commit `dd8f9467d871b0ae5f5a5b4a66ca3d89886d8d04`.
One new finding suggested `const fn` for a helper that constructs allocated errors.
The final reader instead keeps bound admission in `consume` and delegates byte consumption to an effectful helper.
No function visibility was widened to silence that finding.
Task 10791 still reported 94 errors, including that suggestion and a repeated helper name.
The final source uses `collect` inside the read owner. The const suggestion remains a separate tool defect, not a source fix.

## Confirmed const-candidate false positive

Task 10794 ran three direct probes with the existing March-21 compiler and retained driver/library:

| Probe | Exit | Observation |
| --- | --- | --- |
| Ordinary helper with nested `Err(format!(...))` | 0 | Rust accepts it |
| Same helper with `-D missing_const_fn` | 101 | Octet suggests `const fn` |
| Same helper declared `const fn` | 1 | Rust rejects formatting with E0015 |

The two input files differ only by `const` on the helper.
Neither input has an allow attribute. The probe flags are diagnostic controls, not the production command.

The pinned Octet owner is `src/purity/effect_lints.rs::function_is_const_candidate`.
Its string filter excludes `" format!("` but misses nested `Err(format!(...))`.
The current reader has the same nested allocation shape.
This evidence does not justify declaring allocating functions const, changing whitespace to evade detection, or suppressing the lint.
It also does not classify the other remaining source findings as false positives.
No Octet source, library, driver, compiler, or pin changed during this batch.

## Final observed result

Task 10796 ran the unchanged command on frozen commit `22f3d3a5ebb53050848c6648dfc596481e9e8b12`.
It reported 93 node-host errors, zero warnings, exit 2, and Cargo exit 101.
This reduces the reported count from 106 to 93. It is not a passing gate or complete workspace coverage.
The remaining count includes the confirmed const-candidate defect.

The final attempt took 58.593 seconds, with a 2G memory peak.
Its systemd invocation was `d0fd44dc54124327873c8a76a4ffc60f`.
The source archive BLAKE3 is `a75c68d493d4ab7eeb3cbadd23d5e166a9b75d9bf96c5b0200a07028e7d6c58a`.
All four attempts retained matching before/after input identities and empty tracked-source diffs.
Their raw status files are copied without modification beside this document.

The tasks gate passed structurally in task 10798, with three tasks done and two open.
An earlier invocation from the private log directory could not find the relative Cairn policy. That failure is retained.
Structural passage does not close the behavior tasks.

Next: repair const-candidate detection at its Octet owner, with positive and negative controls, before any lint-tool build or pin promotion.
The other source findings still need review. No replacement lint-library build is claimed or started.

## Full command and authority

The full command remains `cargo octet check --artifact-dir target/octet`.
Each attempt uses a fresh frozen source archive, target, home, and explicit offline dependency caches.
The retained repaired driver, CLI, library, compiler, hook, scope, and flags are unchanged from the preceding diagnostic run.
Before/after input identities and the final tracked-source diff remain separate checks from the lint result.

The current startup pin remains c9b06bc. The repaired driver is an explicitly identified diagnostic tool, not an approved complete runtime cohort.
Tasks 4 and 5 remain open. All startup, content, and consumer guards remain active.
No production build, toolchain rebuild, Stage0, VM, replay, physical deployment, or release promotion occurred.
