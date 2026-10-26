# Validation status

## Not run

The preparation environment does not provide `cargo` or `rustc`. No Rust package
in this bundle was compiled, type-checked, formatted with rustfmt, or linted with
Clippy. **None of the 65 Rust test annotations in the overlay represents an
executed test result.** The launch demo, Wasm guest build, cross-host launch and
root Aspen reference-scheduler tests have not run here.

A lexical scanner can miss syntax, type, borrow-checking, dependency, API and
runtime defects. Passing the checks below does not establish that the Rust code
builds or works. No dependency graph was resolved and there is no generated
`Cargo.lock`. There is no performance, security, durability or production-readiness
qualification.

## Executed on the expanded bundle

- Parsed all 10 TOML files with Python's `tomllib`.
- Ran `bash -n` over the four supplied shell scripts.
- Scanned 34 Rust files using Pygments' Rust lexer, checked delimiter balance and
  literal include paths; no lexical errors or unmatched delimiters were found.
- Ran the seven-crate manifest/dependency and lexical effect-boundary smoke check.
- Ran six Python patch-transformation regression tests.
- Separately generated all delivered text hunks, ran `git apply --check` and
  `git apply` in a synthetic Git fixture, and checked each resulting file against
  its intended bytes. This is **not** a check on the complete Aspen worktree.
- Compiled the Python scripts with Python's parser, without running deployment
  commands or changing the repository.

In the complete source bundle, see `validation/static-checks.json`, `validation/boundaries.log`,
`validation/patch-tests.log`, `validation/patch-roundtrip.log` and `validation/toolchain.log` for recorded results.
`validation/source-sha256.json` inventories the delivered source bytes.
The scanner itself is included as `validation/static_checks.py`; it optionally
requires Pygments. The normal user verification script does not require Pygments.

`first-slice-validation/` is historical evidence from the smaller initial bundle.
Its finite Python model was not the Rust implementation, and its old hashes do
not inventory this expanded source. Do not use it as validation of the new adapters.

## Independent execution

From `overlay/pilots/orchestration`:

```sh
bash scripts/verify.sh
bash scripts/demo.sh
```

`verify.sh` formats the authored Rust before testing, runs workspace tests including
real storage/execution and separate-process integration cases, invokes Clippy, and
builds the release binary. It writes `target/validation/verify.log`. Set
`CLIPPY_STRICT=1` to reject all Clippy warnings. Retain the resulting Cargo.lock,
source revision, compiler versions, and full logs when reporting validation.

After installing into the exact Aspen base and supplying its repository-specific
dependencies, separately run:

```sh
bash scripts/reference-tests.sh
```

The live tests use one host and loopback addresses. Passing them would still not
qualify cross-host networking, production scale, shared Molten authority, HA,
untrusted compiler inputs, or exactly-once external effects.
