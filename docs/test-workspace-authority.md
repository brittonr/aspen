# Test workspace authority

Molten tests use one reviewed temporary-root shell and capability-derived role roots instead of predictable process-id/counter directories or process-wide stale-prefix cleanup.

## Workspace acquisition and lifetime

r[impl molten.testing.cap_std_workspace] `TestWorkspace::new` validates a logical label, asks `cap-tempfile` for a collision-resistant RAII directory, and creates the fixed role directories through its `cap_std::fs::Dir`. Callers do not select or pre-delete a host temporary path. The last workspace guard owns best-effort cleanup; normal test startup never scans `std::env::temp_dir()` for `molten-*` entries.

Abrupt process termination can still leave host residue. Residue maintenance requires a separate operator command, explicit temporary-root authority, and an age/ownership policy; another test process must not infer deletion authority from a filename prefix.

## Typed role roots

r[impl molten.testing.cap_std_subroots] A workspace exposes distinct `TestRoot` marker types for:

- state;
- input;
- output;
- transport;
- ledger;
- cache; and
- adversarial setup.

Each root owns a cloned directory capability scoped to its role. Logical `WorkspacePath` values are bounded portable relative locators and reject parent traversal, absolute paths, platform prefixes, and URL-like locators. Tests should pass the narrowest role root to capability-aware APIs. A path compatibility API may receive a path only through the process bridge described below.

`AdversarialSetup` is a test-shell handle. It can corrupt, replace, remove, change mode, or create a hostile symlink for an owned target root. The production operation still receives only its normal target capability.

## Child-process bridge

r[impl molten.testing.cap_std_process_bridge] Existing command-line interfaces require host path arguments. `ProcessPathBridge` verifies that a typed root belongs to the workspace, then returns a `ChildProcessPlan` with:

- a diagnostic path used only by `Command`/CLI argument and current-directory setup; and
- a logical root label used by canonical observations.

On Unix the shell resolves the already-open temporary directory descriptor through `/proc/self/fd`; it does not scan or choose a temporary name. Hosts without this descriptor bridge return `Unsupported` until an equivalent reviewed handle-to-path implementation exists. An external path is not silently accepted as workspace authority; it must be supplied and classified as separate explicit authority.

`ProcessWorkspace` is the compatibility guard for migrated path-oriented tests. It retains the RAII workspace while implementing `AsRef<Path>` only for test process shells. Its `Debug` output omits the diagnostic host path.

## Artifact retention

r[impl molten.testing.cap_std_cleanup] Normal drop cleans the owned workspace. Selected failure artifacts are retained only with `ArtifactExportPlan` and a separately supplied output root. Export receipts bind logical source/destination labels and BLAKE3 content refs, never temporary host paths. Missing, traversing, or cross-workspace sources fail before export.

## Converted scopes and structural gate

The first enforced migration includes the shared library and binary test shell, all integration CLI/cluster/process fixtures using the common `temp_dir` bridge, the local-store tests, the shared chunk, dataspace, exchange, and evidence-chain helpers, a retention capability workflow, and node lifecycle plus async live-Iroh workflows. Unconverted module-local helpers may still use legacy predictable roots, but the former shared process-wide stale-prefix scanner is now an intentional no-op. Those helpers are outside the blocking rule until migrated; they receive no broad cleanup authority.

r[impl molten.testing.cap_std_regression_gate] `test-ambient-temp-workspace.yml` blocks `std::env::temp_dir`, process-id root construction, and broad `remove_dir_all(entry.path())` cleanup in converted shared CLI, store, retention, dataspace, exchange, evidence, node, and test-support helper scopes. Its positive fixture preserves the prohibited patterns; its negative fixture covers `cap-tempfile` bootstrap and explicit capability-rooted export.

The gate is syntax-level evidence only. It does not prove RAII cleanup after a crash, semantic test isolation, operating-system sandboxing, or that an arbitrary child process respects its supplied root.

## Evidence and non-claims

r[impl molten.testing.cap_std_validation] Workspace tests cover concurrent isolation, typed roles, async lifetime, child execution, normal cleanup, explicit export, invalid locators, cross-workspace substitution, symlink escape, adversarial mutation, replaced-entry cleanup, export denial, and host-path leakage checks.

These results do not make native test code untrusted, transfer production authority, prove cleanup after `SIGKILL`, prove confidentiality, or strengthen runtime correctness/release claims beyond the tests actually run. Temporary host paths remain diagnostic-only.
