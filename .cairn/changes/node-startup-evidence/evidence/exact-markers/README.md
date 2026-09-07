# Exact policy-marker repair: source gate 15

## Result

Task 10434 ran the unchanged canonical command on frozen product source `ab3efbe9788643caddd7f22bb0a13eca0d90426f`.
It reported **30 node-host errors, zero warnings**, exit 2 (Cargo 101).
The summary is byte-identical to run 14. No finding was suppressed by this repair.
This is not complete workspace coverage. Startup remains unauthorized; no normal-node VM launched.

The source archive matched run 14 byte-for-byte:
`5dc01614cbd7d3e3f4d7ad74b6673c62ede13bb7c04a75f45acc70f175fcbc5a`.
Only the lint library changed. Compiler, driver, recovered CLI/runtime, hook, command, package selection, all-targets scope, configuration, and profile stayed unchanged.
Recorded before/after identities matched; tracked source stayed unchanged; no ICE markers appeared.
Elapsed 54.066 seconds; peak memory 2G.

## Owner repair

Octet implementation `da2f6a1` replaces debug-text substring matching with exact HIR attribute-path matching.
`src/safety/policy_markers.rs` is the shared owner used by both fragile enum matches and FSM core/enum markers.
It accepts only matching custom tool attribute paths and rejects empty path segments.
Docs, wrong namespaces, name prefixes, extra segments, and unrelated payloads cannot establish a marker.
The compiler's `Attribute::Unparsed` variant contains the parsed custom-tool `AttrPath`; no source-text matching remains in this helper.

This repairs the doc-text false clean retained in `../enum-policy/`.
It does not declare any Molten enum sealed, classify observation enums as FSM domains, or alter a production marker/feature/arm/policy.
Ordinary unmarked-enum policy and FSM source-scope selection remain unchanged.

## Verification

Task 10430 audited one library derivation. Task 10432 built it in 26.554 seconds with the existing March-21 compiler and dependency artifacts.
Five existing FSM policy tests and seven existing purity-classifier tests passed.
Task 10433 passed 19 new probes and all 40 prior growth/arithmetic/const/workspace/driver probes and controls.
The new probes cover genuine/custom markers, doc and namespace spoof rejection, unrelated payloads, empty configuration, compiler E0004 preservation, FSM enum admission, wildcard denial, and exact core opt-in.
Task 10435 confirmed that the old library fails the corrected doc-spoof expectation by producing no diagnostics.

Configuration probes are Nickel-authored. Ambient Nickel lookup failed in task 10428 before formatting/building; an existing Nickel 1.17 binary was selected explicitly afterward.
Its digest and exported TOML inputs are retained. No Nickel acquisition/build ran.
No compiler, driver, CLI, runtime, Verus, Mantle, Darkhttpd, or Stage0 build ran.
Full Octet UI/workspace/flake/Clippy acceptance remains unestablished.

- Library: `/nix/store/h3zdp7y2wknxxc0krs8x5gl7qhxzzkkv-octet-0.1.0/lib/liboctet.so`
- BLAKE3: `61c02bb43a8c890d13f8f1d212b8ef7082e212bcae9ab86796163af55e09f228`
- Derivation: `/nix/store/508nv3pkijnfyhzvvmjd21q5d3b4wg5i-octet-0.1.0.drv`
- Focused build entry: Octet `verification/exact-markers/check.nix`, not the default package output.

A private GC root retains the library; local content verification passed.
No signature, reproducibility, or approved runtime-cohort claim follows.
Owner replay: `verification/exact-markers/verify.sh OUT DRIVER LIBRARY COMPILER NICKEL`, absolute inputs and fresh output.
Private evidence: `~/.local/state/onix/molten-node-vm/exact-markers/` and `lifecycle-source-gate-15/`.
Earlier attempts and the original false-clean reproducer remain intact.

## Next boundary

Marker admission is now tested, but declaring domains sealed is a separate source-contract decision.
Review explicit closed-domain declarations for LocalStoreKind, NodeStateNamespaceKind, and NodeStateFileObservation against the authority contract retained in `../enum-policy/`.
Preserve compiler rejection of unhandled variants; do not add markers merely to clear findings, catch-all arms, or blanket allows.
Other naming, assertion-density, compound-condition, and file-length findings remain open.
Startup pin, guards, lifecycle tasks 4–5, May-26 build/binding, normal-node VM, and replay remain unestablished or unchanged.
