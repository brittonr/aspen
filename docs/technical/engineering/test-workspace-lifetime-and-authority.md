# Test Workspace Lifetime and Authority

Temporary test storage is both a lifetime problem and an authority problem. A unique name addresses neither by itself. This article explains the reviewed workspace shell, typed role roots, child-process bridge, and selected export path. It assumes familiarity with Rust ownership and capability-relative filesystem APIs. The [test workspace authority guide](../../test-workspace-authority.md) governs the migrated scope. Return to the [Technical companion](../README.md).

## Acquisition and retention are distinct

`TestWorkspace::new` validates a logical label, acquires a `cap_tempfile` temporary directory at the reviewed ambient bootstrap, derives a diagnostic host path, and creates seven role directories. The [construction implementation](../../../src/test/parts/support/p000/body.rs) places the temporary directory inside `WorkspaceInner`, held by `Arc`. Both `TestWorkspace` and individual `TestRoot<R>` values retain that shared owner.

Consequently, the outer workspace variable is not the only lifetime guard. A role root retained across asynchronous work also retains the underlying workspace. Cleanup follows the final owner rather than a process identifier, counter, or directory prefix. This prevents a common illustrative mistake: deleting a shared temporary directory when one concurrent task finishes while another still holds a role root.

This is normal-drop behavior, not crash recovery. The governing guide explicitly allows residue after abrupt termination. A filename that resembles a Molten workspace does not confer deletion authority on another test process. The old shared stale-prefix cleanup function is an intentional no-op in the [compatibility implementation](../../../src/test/parts/support/p001/body.rs), not a residual broad cleanup mechanism.

## Role and locator carry different information

The roles are state, input, output, transport, ledger, cache, and adversarial setup. `TestRoot<R>` combines a role marker, a directory capability, and the shared workspace owner. `WorkspacePath` is instead a relative locator. Its parser rejects empty or escaping forms, backslashes, URL-like locators, and excessive component counts; it does not turn an arbitrary host path into authority.

These types answer separate questions: which workspace and role may be accessed, and which relative entry within it is requested? A role marker is not a universal operating-system read-only permission. The inspected generic root implementation exposes read and write methods for all markers. The type helps callers pass the intended role; actual operations still depend on the supplied directory capability and API.

`AdversarialSetup` is deliberately test-shell authority. It can arrange corruption, replacement, hostile links, and mode changes without requiring the production operation to receive that broad setup handle. Tests then exercise the production operation through its normal narrow root.

## Child processes cross a representation boundary

Existing command-line programs consume paths, so `ProcessPathBridge::plan` verifies workspace ownership and produces a `ChildProcessPlan` with a diagnostic path and logical root label. The path is for command arguments or current-directory setup; logical labels are used for portable observations.

The plan itself contains a `PathBuf` and label, not an `Arc<WorkspaceInner>`. Retaining a cloned plan therefore does not independently retain cleanup ownership. `ProcessWorkspace` addresses path-oriented compatibility by retaining a `TestWorkspace` alongside the plan. For direct bridge usage, reviewers should ensure a workspace or root guard outlives the child operation rather than infer lifetime from the plan's name.

The inspected Unix helper resolves `/proc/self/fd/<descriptor>` with `std::fs::read_link`. Non-Unix builds explicitly return `Unsupported`. There is a scoped discrepancy with the governing guide's broader wording that hosts without the descriptor bridge return `Unsupported`: on a Unix host lacking usable `/proc/self/fd`, this implementation propagates the filesystem error instead. This article does not claim uniform error classification or portability across all Unix hosts.

## Worked export and substitution scenario

Suppose, illustratively, workspace A creates `state/receipts/run.preserves`, and workspace B supplies an output root for selected artifacts. Export is allowed because `export_selected` verifies that the source root belongs to A, reads the selected relative source, and writes through the separately supplied output capability. It returns logical source and destination paths plus a BLAKE3 content reference. The [async export test](../../../src/test/parts/support/p002/body.rs) exercises this cross-workspace destination arrangement.

Now substitute B's state root as A's source. Ownership validation rejects that source before reading it. This is not a prohibition on all cross-workspace transfer: an explicit destination is intentionally permitted, while pretending another workspace's source belongs to A is denied. Keeping that asymmetry visible avoids weakening export review into a vague “same workspace everywhere” rule.

The receipt describes selected exported bytes. It is not a canonical runtime admission receipt, a guarantee of durable retention after destination cleanup, or proof that an external child obeyed its root. Retention still depends on the destination's actual lifetime and owner.

## Verification and limits

Suggested review starts with the embedded concurrency, async export, child execution, cleanup, and substitution cases. Exercise both invalid locators and capability-level escape attempts; lexical path validation and filesystem confinement are complementary. The [structural audit guide](../../ast-grep-runtime-authority-audits.md) describes the converted helper scopes that reject predictable roots and broad cleanup syntax. Neither those fixtures nor the workspace tests were executed while writing this article.

The conversion is scoped, not a claim that every historical module-local helper is migrated. RAII does not survive `SIGKILL`, typed roles do not sandbox arbitrary native code, and diagnostic-path suppression does not establish confidentiality. The useful guarantee is explicit ownership of the temporary root and deliberate narrowing or bridging of that ownership at each test boundary.

## Sources

- [Test workspace authority](../../test-workspace-authority.md)
- [Runtime-authority audits](../../ast-grep-runtime-authority-audits.md)
- [Workspace construction and export](../../../src/test/parts/support/p000/body.rs)
- [Bridge lifetime, path validation, and host handling](../../../src/test/parts/support/p001/body.rs)
- [Workspace behavior tests](../../../src/test/parts/support/p002/body.rs)
- [Technical companion](../README.md)
