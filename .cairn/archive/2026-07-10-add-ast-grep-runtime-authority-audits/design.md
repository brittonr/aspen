## Design

1. Define ast-grep audit profiles for Aspen/Molten runtime authority seams: core runtime, node control socket, effect handlers, plugin host, policy/evidence gates, sealed repro, Iroh transport, and operator workflow.
2. Start with inventory-only rules for ambient filesystem, process, network, clock, random, credential, plugin-loading, unsafe, panic, and direct authority-bypass source shapes.
3. Bind scan receipts to ast-grep version, rule bundle BLAKE3 identity, scan scope, runtime/evidence-gate run identity, findings summary, and non-claim labels.
4. Require positive and negative fixtures before warning or blocking posture.
5. Preserve findings in evidence-gate receipts as structural candidates only; replay, authority, and release claims remain owned by existing gates.
6. Keep ast-grep invocation in CLI/harness shell code and keep runtime cores independent of scan state.
7. Treat codemod recipes as explicit migration plans with reviewed diffs, post-check commands, and non-claims.

## Non-Goals

- No replacement for deterministic replay, sealed-repro, policy preflight, Basalt/UCAN authority checks, Octet, Valence, or Cairn release gates.
- No claim that a clean ast-grep scan admits runtime authority or proves replay correctness.
- No automatic codemods during runtime node or operator workflows.
