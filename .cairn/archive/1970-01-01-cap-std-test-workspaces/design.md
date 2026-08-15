## Context

`src/test/support.rs` and many module-local helpers create paths under `std::env::temp_dir`, add process and counter suffixes, remove stale directories by ambient path, and return `PathBuf`. Some CLI tests also scan the global temporary root to remove old `molten-*` entries. The pattern is widespread enough that local fixes would continue to drift.

The goal is not to hide all host paths from process-spawning tests. The goal is to centralize ambient temporary-root acquisition and give each test only the workspace authority it needs.

## Decisions

### 1. One RAII workspace abstraction

**Choice:** Add a shared `TestWorkspace` backed by the cap-std project's temporary-directory crate and a capability directory handle. Construction acquires ambient temporary storage once; drop owns best-effort cleanup. Tests no longer choose or pre-delete process-wide names.

**Rationale:** The temporary-directory implementation already handles collision-resistant creation and lifecycle better than repeated pid/counter schemes.

### 2. Logical typed subroots

**Choice:** The workspace creates typed subroots for state, input, output, transport, ledger, cache, and adversarial setup through capability-relative operations. Tests pass the narrowest root or port to the system under test.

**Rationale:** Isolation is more useful when unrelated fixtures do not share one all-powerful root handle.

### 3. Adversarial setup authority is separate

**Choice:** Negative tests that must create symlinks, corrupt bytes, change modes, or replace entries use a distinct setup handle held by the test harness. Production APIs receive only the target capability. Helper names make setup mutations explicit.

**Rationale:** Tests need to model hostile state without teaching production code to recover ambient authority.

### 4. Child-process paths are a shell bridge

**Choice:** Multiprocess and CLI harnesses may render a workspace path only at the child-spawn boundary because command-line interfaces require path strings. The parent shell retains the capability, bounds child roots to workspace subdirectories, and records logical labels or content refs rather than host paths in canonical evidence.

**Rationale:** `cap-std` handles are not automatically transferable through existing CLIs, but the exception can remain narrow and visible.

### 5. No global stale-directory scan

**Choice:** Remove test cleanup that scans `std::env::temp_dir` for name prefixes. Cleanup targets only the workspace object created by the current fixture. If process-crash residue needs collection, it belongs in an explicit operator maintenance tool with separate authority and age policy, not normal tests.

**Rationale:** Prefix-based deletion grants broad ambient authority and can remove unrelated concurrent work.

### 6. Failure retention is explicit export

**Choice:** A test or harness may export selected logical artifacts to an explicit destination capability before workspace drop. The export uses the shared materialization boundary when available. A keep-on-failure flag may preserve diagnostics only when the operator explicitly selects it, and preserved host paths remain non-canonical.

**Rationale:** Debuggability should not depend on silently leaking temporary trees.

### 7. Migration is representative first, then enforced

**Choice:** Convert shared CLI helpers and high-concurrency/high-authority suites first: node, chunk, retention, remote transport, evidence, and multiprocess harnesses. After fixtures prove portability, enable a scoped rule against new pid/counter temp helpers and global temp scans.

**Rationale:** A staged migration reduces churn while preventing new copies of the old pattern.

## Functional core / imperative shell

- **Pure core:** logical subroot declarations, artifact-retention plans, path-label normalization, evidence redaction, and cleanup/export decisions.
- **Imperative shell:** temporary directory creation, capability handles, subdirectory creation, adversarial setup, child-process spawning, cleanup, and explicit artifact export.

## Risks / Trade-offs

- Some test libraries expect `Path`. Keep conversions in test shells and avoid adding path-taking APIs back to production cores.
- RAII cleanup is not guaranteed after abrupt process termination. The change removes unsafe broad cleanup rather than claiming crash-proof cleanup.
- Migrating every test at once is noisy. The structural gate activates only for converted helper/suite scopes until the remaining inventory reaches zero.
