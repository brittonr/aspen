# Syndicate reference harness boundary

r[impl molten.syndicate_dataspace.no_wire_compat] r[verify molten.syndicate_dataspace.no_wire_compat]

Molten uses Syndicate only as a local reference-semantics harness for adopted dataspace scenarios.

Normative Molten boundaries remain canonical Preserves runtime steps, BLAKE3 value refs, policy gates, authority gates, resource gates, provenance/source-gate evidence, and Molten receipts. Syndicate bag/account/trace/capability observations are recorded as Molten evidence for parity, diagnostics, and backpressure review, but they do not grant authority, transport, policy, resource, provenance, source-gate, retention, plugin, or execution trust.

The harness does not claim Syndicate wire-protocol, relay, sturdyref, service, trace, capability, or authority compatibility. Any future compatibility surface needs its own scoped change with explicit requirements, pass/deny fixtures, and receipt evidence.

## Reference introduction boundary

The local/recorded remote-session profile is separately governed by
`.cairn/changes/recorded-remote-session-source/`. The receiver owns a transactional recording under its node-state
capability. Admitting that profile requires both initialized node-root capabilities, binds their actual identities
and topic, and allocates a monotonic receiver generation. The sender view borrows that owner and rereads committed
history and initialized identities on every envelope build. It is not an imported runtime snapshot, reference list,
fixture receipt, or claim of authenticated network-peer knowledge.

Only established identity/session bootstrap refs and refs in live committed assertions introduce a reference. A
message never introduces its own carried references. Retracting the last mentioning assertion withdraws an
introduction; close or a changed generation invalidates the old view. Missing, stale, closed, mismatched, or corrupt
source state refuses message construction with canonical local build-denial evidence before any envelope is returned
for publication. Typed protocol/job carriers inventory semantic reference fields rather than their schema metadata.

Local publication rechecks the source even for an imported or previously built envelope file. A prior build is not a
publication receipt: a retraction between build and send refuses before the transport writes the envelope. Canonical
wire fields must also match the envelope's decoded projection. The unbound live gossip entrypoint has no authenticated
receiver-observation capability, so it refuses referenced messages rather than substituting a local recording for a
network acknowledgment; reference-free messages and introducing operations remain available there.

These are Molten reference-lifetime and local recording rules, not Syndicate wire compatibility or a new authority
grant. Network observation admission and remote synchronization are outside this explicitly local/recorded profile.

## Exercising the local/recorded profile

The public source fixture initializes actual node identities, commits real runtime assertions/messages, stores a
content blob, publishes/delivers canonical local gossip, and checks retraction, close, generation fencing, corruption,
bounded retirement, and lost-database refusal. The CLI fixture launches the actual `molten` binary for session
admission/commit/receive/close and envelope build/publish/deliver, checks canonical denial artifacts and absence of
denied envelope outputs, then compares the committed receiver runtime snapshot with its canonical receipt.

```sh
cargo test -p molten --lib recorded_session_tests
cargo test -p molten --lib reference_scanning_handles_unicode_before_and_inside_candidates
cargo test -p molten --test cliharness cli_recorded_remote_session_refuses_unknown_and_revoked_references
```

These are runnable verification commands, not a claim that a particular checkout has executed or passed them. Native
change evidence and task completion must cite actual retained runs after CLI/source integration.

