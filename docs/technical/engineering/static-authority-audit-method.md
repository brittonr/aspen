# Static Authority Audit Method

Molten uses structural scans to make authority-sensitive code review more repeatable. This article explains how to interpret rule scope, fixture posture, and evidence binding without turning syntax findings into semantic authorization. It assumes familiarity with ast-grep patterns and Rust modules. The [runtime-authority audit guide](../../ast-grep-runtime-authority-audits.md) governs the profile; implementation details below are deliberately narrower where sources differ. Return to the [Technical companion](../README.md).

## Begin with an authority question

A useful scan asks where code acquires or reacquires authority, not simply whether a file contains I/O. Public shells may legitimately bootstrap a capability. Inner operations are expected to carry the resulting authority object rather than reconstruct it from an ambient path. Test setup may intentionally create hostile filesystem state, while production descendant I/O belongs behind capability-aware adapters.

This makes path scope part of the rule's meaning. The inspected [node-state root-reacquisition rule](../../../tools/ast-grep/runtime-authority/rules/node-state-root-reacquisition.yml) is `error` severity with blocking posture, lists specific converted daemon, identity, and job pages, and ignores `**/tests/**`. Its patterns cover qualified and unqualified `NodeStateRoot::open`, namespace reopening, and ambient directory acquisition. That is not equivalent to banning these operations everywhere in the repository.

A no-finding result therefore means no configured pattern matched the supplied scanned surface under the tool's interpretation. It does not imply that every path to authority has been eliminated. Aliases, helper indirection, unsupported syntax, omitted files, or a newly relocated implementation can all change coverage without changing the intended architectural boundary.

## Inventory and blocking evidence answer different questions

The governing guide distinguishes hint-level inventory candidates from narrowly blocking converted scopes. An inventory finding directs review toward a potentially relevant call; it is not by itself a violation. A blocking finding indicates that a prohibited structural shape remains in a surface whose migration contract excludes that shape.

The [profile model](../../../src/audit/parts/ast_grep/p000/body.rs) represents `Inventory`, `Warning`, and `Blocking` as `RulePosture`. Profile validation requires positive and negative fixture references for warning or blocking rules. The positive fixture preserves the prohibited shape; the negative fixture records permitted shell or carried-authority structure. This pair protects against two different mistakes: a rule that misses the regression and a rule that rejects the intended replacement.

The inspected validator checks that those fixture references are present. It does not read their files or execute ast-grep. Actual fixture execution remains a separate verification step. Likewise, a profile containing every required surface name does not prove that a scanner visited all files belonging to those surfaces.

## Binding an observation without overclaiming it

`AstGrepScanInput` carries the profile, tool-version string, rule-bundle hash, scan-scope hash, evidence-gate run reference, and findings. The [receipt-check implementation](../../../src/audit/parts/ast_grep/p001/body.rs) checks profile validity, tool-prefix shape, content-reference forms, known finding rule identifiers, absence of findings for declared blocking rules, and required non-claims.

Those are checks over supplied observations. They do not attest that a named executable actually ran or independently recompute source-file measurements. `requires_fresh_scan` compares the receipt's rule-bundle reference with a supplied current reference; it does not itself execute a fresh scan.

The helper named `rule_bundle_hash` hashes sorted profile metadata: surfaces, scope strings, rule summaries, postures, fixture paths, and non-claims. It does not read YAML rule bytes. Similarly, `scan_scope_hash` hashes sorted supplied surface identifiers, not scanned source contents. These observed mechanics are narrower than interpreting the guide's “rule bundle identity” and “scan scope identity” as automatic measurements of every rule and input byte. Reviewers need to inspect the producer of those references before reusing a receipt.

## Worked stale-rule scenario

Suppose, illustratively, a YAML pattern is broadened to catch an additional spelling of ambient root acquisition, but the in-memory profile summary is unchanged. A prior scan cannot establish that the new spelling is absent. The guide consequently requires a fresh scan after a rule-bundle change.

The inspected metadata-hash helper alone may remain unchanged in this scenario because it does not consume the edited YAML bytes. That is a binding limitation, not permission to reuse old findings. A review should compare the actual scan invocation, rule content, and supplied receipt references, then require evidence for the changed rule under the governing process. This article does not invent an additional measurement API or claim the gap is already closed.

## A second scoped source discrepancy

The governing guide lists node-state adapter coverage and two node-state blocking rules. The inspected YAML rule exists with blocking posture, but `runtime_authority_profile()` builds its default rule list from `inventory_rules()`, which currently adds only the store, test-workspace, and materialization blocking rules. Its required/default surfaces also omit `node-state-adapters` while including `filesystem-materializers`.

Thus the existence of the YAML rule does not establish that the default Rust receipt profile declares it. Because receipt checks require findings to reference known rules, callers cannot assume those two representations are interchangeable. This discrepancy is left explicit rather than repaired or silently normalized in documentation.

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
