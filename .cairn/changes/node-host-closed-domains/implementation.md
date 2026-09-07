# Closed-domain implementation checkpoint

## Scope

Three conditional exact sealing annotations and crate-level tool registration implement the decision in review.md.
The package declares only the expected Dylint cfg; unexpected-cfg severity remains warn and external -D warnings still applies.
No production variant, public name, payload, match arm, directory mapping, error message, operation order, or authority check changed.
No FSM marker, fallback arm, blanket allow, unconditional registration feature, or startup permission was added.

`tests/classifications.rs` adds four tests: eight fixed store labels; the ordered 14-kind namespace inventory; bidirectional alias-entry denial; and explicit missing/nonregular/acquired-file behavior and errors.
Existing capability-backed tests remain intact, including symlink denial, bounded reads, mode behavior, and independently acquired root views.

## Package checks

Task 10399 passed all 22 node-host tests and package all-target Clippy with -D warnings (159.393 seconds, peak 1.5G).
The existing source-tarball Rust 1.97.1 compiler and matching-version Cargo/Clippy tools were explicitly selected; this is not equivalence to the missing rust-minimal compiler or May-26 production binding.
The focused environment disables the ambient rustc cache wrapper and ambient encoded rustflags, uses two jobs, existing runtime linker tools, and a private TMPDIR.
RUSTC_BOOTSTRAP=1 remains the existing focused-package compatibility scope; separate ordinary stable fixture controls did not set it.

Task 10365 failed before tests because ring's C compilation exhausted /tmp. Its full log is retained as tests-tmp-full.log.
Task 10367 stalled during df and was killed after the streamed wait expired; it did not reach its copy/format commands.
No unrelated files were removed. Redirecting temporary files to the private work directory allowed the next run to complete.
The initial stable rustfmt invocation reported unsupported nightly configuration options. No format-gate acceptance is claimed.

## Actual-source compiler mutations

Task 10408 passed eight metadata probes on complete private copies of the actual node-host crate:

| Case | Plain March-21 rustc | Active Octet driver | E0004 sites |
|---|---:|---:|---|
| Baseline | 0 | 0 | none |
| LocalStoreKind gains Future | 1 | 101 | local_store/mod.rs |
| NodeStateNamespaceKind gains Future | 1 | 101 | node/state/authority.rs |
| NodeStateFileObservation gains Future | 1 | 101 | node/state/filesystem.rs and node/state/namespace.rs |

The mutations add only one variant to the respective declaration. They do not change consumers.
The probes reuse existing March-21 dependency metadata from source gate 15, including cap-std, cap-fs-ext, cap-tempfile, and molten-core.
They compile the actual crate root directly, not a reduced scalar model. No dependency, compiler, driver, or lint-library build occurs in this probe.
They are targeted enum/compiler checks, not the canonical complete source gate or a runtime build.
The active baseline emits 19 non-enum warnings under this probe's limited lint flags; these are retained and not accepted as a clean source gate.
The runner checks expected codes, E0004 counts and file sites, rejects missing/ambiguous direct dependency metadata, and compares tool, script, and complete case-tree hashes before/after.
No ICE markers appeared. These hashes are not a complete transitive runtime-cohort attestation.
Task 10404 also passed the initial probes; task 10408 reran after tightening preflight and input-hash checks.
Earlier reduced route and marker-spoof controls remain separately scoped evidence.

## Replay

`evidence/verify-actual-mutations.sh OUT CASES DRIVER LIBRARY COMPILER DEPS` requires absolute inputs and a fresh OUT.
Create CASES/baseline from the recorded implementation's complete node-host crate. Copy it to store, namespace, and observation; add only Future to each named enum.
Retained source diffs identify the exact changes. Reuse a compatible existing dependency corpus; do not bootstrap or acquire a compiler on cache miss.

Private evidence: `~/.local/state/onix/molten-node-vm/closed-domains-implementation/`.
The next step is freezing this implementation and running the unchanged canonical command.
The last full gate is still run15 (30 errors); no clean workspace, approved cohort, May-26 build/binding, VM launch, or replay is claimed here.
