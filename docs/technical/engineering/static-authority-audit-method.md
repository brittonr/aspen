# Static Authority Audit Method

Molten uses structural scans to make authority-sensitive code review more repeatable. This article explains how to interpret rule scope, fixture posture, and evidence binding without turning syntax findings into semantic authorization. It assumes familiarity with ast-grep patterns and Rust modules. The [runtime-authority audit guide](../../ast-grep-runtime-authority-audits.md) governs the profile. Return to the [Technical companion](../README.md).

## Begin with an authority question

A useful scan asks where code acquires or reacquires authority, not simply whether a file contains I/O. Public shells may legitimately bootstrap a capability. Inner operations are expected to carry the resulting authority object rather than reconstruct it from an ambient path. Test setup may intentionally create hostile filesystem state, while production descendant I/O belongs behind capability-aware adapters.

This makes path scope part of the rule's meaning. The inspected [node-state root-reacquisition rule](../../../tools/ast-grep/runtime-authority/rules/node-state-root-reacquisition.yml) is `error` severity with blocking posture, lists specific converted daemon, identity, and job pages, and ignores `**/tests/**`. Its patterns cover qualified and unqualified `DirectoryRoot::open`, namespace reopening, and ambient directory acquisition. That is not equivalent to banning these operations everywhere in the repository.

A no-finding result therefore means no configured pattern matched the supplied scanned surface under the tool's interpretation. It does not imply that every path to authority has been eliminated. Aliases, helper indirection, unsupported syntax, omitted files, or a newly relocated implementation can all change coverage without changing the intended architectural boundary.

## Inventory and blocking evidence answer different questions

The governing guide distinguishes hint-level inventory candidates from narrowly blocking converted scopes. An inventory finding directs review toward a potentially relevant call; it is not by itself a violation. A blocking finding indicates that a prohibited structural shape remains in a surface whose migration contract excludes that shape.

The [profile model](../../../src/audit/parts/ast_grep/p000/body.rs) represents `Inventory`, `Warning`, and `Blocking` as `RulePosture`. Profile validation requires positive and negative fixture references for warning or blocking rules. The positive fixture preserves the prohibited shape; the negative fixture records permitted shell or carried-authority structure. This pair protects against two different mistakes: a rule that misses the regression and a rule that rejects the intended replacement.

The inspected validator checks that those fixture references are present. It does not read their files or execute ast-grep. Actual fixture execution remains a separate verification step. Likewise, a profile containing every required surface name does not prove that a scanner visited all files belonging to those surfaces.

## Binding an observation without overclaiming it

`ast_grep_runtime_authority_core::ScanInput` carries the profile, tool-version string, rule-bundle hash, scan-scope hash, evidence-gate run reference, and findings. The [receipt-check implementation](../../../src/audit/parts/ast_grep/p001/body.rs) checks profile validity, tool-prefix shape, 64-digit BLAKE3 content-reference forms, known finding rule identifiers, absence of findings for declared blocking rules, finding shape (declared surface, repository-relative path, non-empty message), and required non-claims. Every check's `passed` value is derived from its own diagnostics: a check passes exactly when it produced none, and a failing check names the offending finding, reference, or missing non-claim.

Those are checks over supplied observations plus the bytes the audit shell measured. `build_audit_receipt` takes a `ScanMeasurement` with the rule YAML and scanned source bytes, recomputes both identities, and invalidates the receipt when a supplied identity differs from (or cannot be derived from) those bytes, or when a finding names a path outside the measured scan. It does not attest that a named executable actually ran. `requires_fresh_scan` compares the receipt's rule-bundle reference with a supplied current reference; it does not itself execute a fresh scan.

The identity helpers are content-bound. `rule_bundle_hash` takes the profile and the bytes of every declared YAML rule file (`tools/ast-grep/runtime-authority/rules/<rule-id>.yml`) and hashes canonical packed Preserves of the sorted profile metadata plus sorted `(path, BLAKE3(yaml bytes))` pairs. It fails closed when a declared rule's YAML is missing, supplied twice, undeclared, or does not state its own `id` and the profile's posture. `scan_scope_hash` hashes canonical packed Preserves of the sorted `(repository-relative path, BLAKE3(source bytes))` pairs actually scanned, rejecting empty inventories, duplicate paths, and non-normalized paths. Reading those files stays in the audit shell; the pure helpers only hash the supplied bytes.

## Worked stale-rule scenario

Suppose, illustratively, a YAML pattern is broadened to catch an additional spelling of ambient root acquisition, but the in-memory profile summary is unchanged. A prior scan cannot establish that the new spelling is absent. The guide consequently requires a fresh scan after a rule-bundle change.

Because `rule_bundle_hash` consumes the edited YAML bytes, the current rule-bundle reference changes, and `requires_fresh_scan` reports the old receipt as stale. Likewise, editing, adding, removing, or renaming any scanned source changes the scan-scope reference, so a receipt cannot silently stand for a different source inventory.

## Default profile coverage

The default Rust profile declares the `node-state-adapters` surface (converted daemon, ingress, identity, and target-job pages) and both node-state blocking rules, `node-state-ambient-filesystem-call` and `node-state-root-reacquisition`, alongside the store, test-workspace, and materialization blocking rules. Surface scopes are concrete paths or globs taken from the rule files; none is a prose placeholder. A finding from `node-state-root-reacquisition` is therefore a known blocking finding that invalidates the receipt. The Linux eBPF and BPF-loader rules under the same directory belong to their own focused checks and are not part of this default profile.

The archived Cairn change [`harden-traceability-proof-composition-and-static-audits`](../../../.cairn/archive/2026-11-26-harden-traceability-proof-composition-and-static-audits/proposal.md) fixed the earlier rule-binding and profile gaps described here.

## Verification and limits

Suggested verification follows the guide's positive fixture, negative fixture, and converted-production scan commands. Record the actual tool version and scope, inspect exclusions, and review file moves against explicit path lists. No scans were executed for this article. The [modularity inventory](../../modularity-boundaries.md) provides the architectural ownership context needed to interpret results.

Structural hygiene is useful evidence, not runtime admission, UCAN authorization, replay correctness, sealed-repro correctness, distributed safety, or release readiness. A clean scan cannot replace tests of capability propagation, negative admission, and shell behavior.

## Sources

- [Runtime-authority audit guide](../../ast-grep-runtime-authority-audits.md)
- [Modularity boundary inventory](../../modularity-boundaries.md)
- [Profile, fixture validation, and identity helpers](../../../src/audit/parts/ast_grep/p000/body.rs)
- [Receipt checks and default rule list](../../../src/audit/parts/ast_grep/p001/body.rs)
- [Node-state root-reacquisition YAML rule](../../../tools/ast-grep/runtime-authority/rules/node-state-root-reacquisition.yml)
- [Technical companion](../README.md)
