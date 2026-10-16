## Context and evidence

Finding F01 applies to `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`p027/body.rs:312-320` dispatches shutdown directly. `p018/body.rs:120-171` writes shutdown evidence before control admission and removes the active lock afterward. `p029/body.rs:227-260` denies empty authority, policy, or resource references.

Trigger: start a node, queue a shutdown request with `authority_refs=[]`, and dispatch that request. Keep policy and resource references otherwise valid. The current path returns denial after shutdown writes and lock removal. The expected result is denial with unchanged active lifecycle state.

This sequence is static evidence only. No live shutdown or adapter effect ran in the audit. The audit reported 359 passing core baseline tests. Its separate harness reported eight failing regression assertions, none for F01. These facts do not establish F01 execution coverage.

## Decision boundary

A pure shutdown admission function receives the request, current lifecycle facts, and required authority, policy, and resource evidence. It returns denial or a typed shutdown plan. It performs no filesystem access or receipt writes.

The shell gathers facts through the existing capability-rooted boundary. It executes shutdown effects only after admission. Admission does not replace the existing subsystem gates or make nonempty references sufficient authority.

A denied request leaves the active lock, startup evidence, adapter state, and successful shutdown evidence unchanged. Separate denial evidence can describe rejection without claiming shutdown. An admitted effect error remains an explicit failed or uncertain observation. The shell does not publish a complete shutdown result from a planned effect.

## Compatibility and receipts

Existing request identities and valid shutdown ordering remain compatible. Passing shutdown receipts describe observed work only. Existing denial receipts remain diagnostic evidence, not permission to mutate lifecycle state.

The implementation review must decide whether typed internal outcomes suffice or a versioned receipt extension is necessary. Legacy receipts retain their original meaning. Replay must not reinterpret a denied request as an admitted effect plan.

## Tests and validation

The smallest baseline is `local_node_init_run_status_stop_and_restart_recovery_are_receipted`, followed by existing shutdown dispatch controls. Run these before core edits and again afterward.

Normal repository tests must cover valid shutdown, empty authority, empty policy, empty resources, malformed bindings, and adapter failure. Tests must compare pre-rejection and post-rejection state. Controlled adapters must record call order and prove that rejection performs no protected effect.

Run focused Octet and Clippy error gates, the workspace test matrix, relevant Nix node-state checks, and required Cairn gates. Preserve existing gate scope. Record blocked checks as blocked, not passed.

## Ownership, reuse, and order

Molten node-runtime maintainers own shutdown policy and shell ordering. Node-host maintainers own capability filesystem mechanics. Existing node-runtime and node-host components fit these boundaries. No new dependency is mandated.

F14 shares dispatch and loop code. Implement the admission boundary first where practical, then compose F14 current-run observations without repeating historical effects. F02 shares startup fixtures but independently owns source-gate admission. F03 owns ingress publication, not shutdown authority. These overlaps impose review order, not circular blocking dependencies.

## Nonclaims

This package does not prove remote authentication, crash atomicity, complete adapter shutdown, or whole-node correctness. Planned tests become evidence only after execution. Planning does not authorize implementation, archive, or publication.
