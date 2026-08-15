## Why

Aspen's tests create many temporary roots by combining the process id with counters, deleting any pre-existing matching path, and sometimes scanning the process-wide temporary directory for stale `molten-*` trees. This repeats ambient authority across unit, integration, node, chunk, retention, transport, and evidence tests. It risks collision, symlink-sensitive cleanup, cross-test interference, leaked state, and nondeterministic host-path data entering diagnostics.

The `cap-std` project includes capability-rooted temporary directory support. A shared RAII test workspace can provide narrower roots to the system under test and a separate adversarial setup handle to negative fixtures without granting every test process-wide temporary-directory authority.

## What Changes

- Add one shared `TestWorkspace` shell backed by `cap-tempfile` and `cap_std::fs::Dir`, with typed logical subroots for state, input, output, transport, ledger, cache, and adversarial setup.
- Replace hand-rolled process-id/counter temp helpers and global stale-directory scans in targeted suites.
- Keep child-process path conversion in a thin process shell while tests and cores use logical labels and capability roots.
- Make artifact retention after failure an explicit export operation to an operator-selected destination rather than implicit leaked temp state.
- Add positive and negative tests for isolation, cleanup, symlink escape, wrong-root substitution, concurrent workspaces, retained-artifact export, and portable evidence.

## Impact

- **Files**: `src/test/support.rs`, test helpers across runtime/store/node/retention/transport/evidence suites, cluster and multiprocess harness shells, dev dependencies, structural authority rules, and testing documentation.
- **Testing**: workspace lifecycle unit tests plus migrated representative unit, CLI, async, and multiprocess suites.
- **Sequencing**: depends on `complete-cap-std-store-threading` for common locator and capability-shell conventions.
- **Claims**: test workspace isolation reduces ambient filesystem authority; it does not make native test code untrusted, prove cleanup after process kill, or strengthen runtime semantic claims beyond the tests executed.
