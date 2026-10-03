# Reviewing node identity and secret handling

Mode: Review checklist

Use this checklist before accepting a node identity change, sharing an evidence bundle, or claiming endpoint continuity after restart. It is a source-review aid, not a key-rotation command sequence. The [node-state authority contract](../../node-state-filesystem-authority.md) governs containment and secret observation; the [capability-rooted state companion](../../technical/storage/capability-rooted-node-state.md) explains the model. All implementation observations here are source-checked, not runtime-verified for this document.

A review record should distinguish accepted evidence, denied evidence, and missing evidence. Private key material, bearer tokens, and raw tickets do not belong in that record. A public endpoint identity establishes transport continuity only; it does not grant peer admission, authority, policy, resource, provenance, or execution trust.

## Scope and entry-point acceptance

- [ ] **Which entry point actually supplies identity configuration?** Cite the callsite, not only the reusable `Config` type. The daemon's local init creates `Config::new`, supplies local policy refs, and resolves through the root's identity namespace. The inspected node-init CLI declarations do not expose all reusable identity configuration fields as flags.
- [ ] **Is the root explicitly selected and acquired once?** Require the bootstrap shell and root-bearing downstream call in the review. A path retained for diagnostics is not permission for inner code to reopen that path later.
- [ ] **Is the selected namespace appropriate?** Identity resolution accepts identity or secrets namespaces. A similarly named directory under another root is not equivalent authority.
- [ ] **Is the review claim correctly bounded?** No-profile initialization is a local fixture. Profile-backed initialization and an identity receipt do not by themselves establish current release source-gate evidence or a managed-secret deployment.

## Source-selection acceptance

The pure source resolver consumes facts; the shell observes and loads actual material. Review both boundaries.

| Question | Required source or artifact evidence |
| --- | --- |
| Was an explicit key selected before any backend fallback? | Effective configuration and the resolver's explicit-key branch; never include the key in the review. |
| Was a managed backend selected, or merely named? | Actual supplied backend material at the shell boundary and redacted backend ref/class in evidence. |
| If the backend was required but unavailable, was fallback denied? | Denial evidence and the required-backend branch, not a later generated endpoint. |
| Was a persisted leaf rejected when unsafe? | Acquired-file observation and permission decision tied to that observation. |
| Was first-boot generation admitted? | Absence/source facts plus generation policy, followed by restricted persistence evidence. |
| If nothing was available, was generation disabled and resolution denied? | Denial result without a usable identity, not a fabricated substitute key. |

Explicit key selection precedes the required-backend check in the inspected resolver. Do not describe `require_secret_backend` as an unconditional ban on explicit keys. That is a precedence boundary to review against deployment policy, not a reproduced failure.

## Filesystem and redaction acceptance

- [ ] **Were type, permissions, size, and bytes observed through the acquired file handle?** Require the `observe_file` → regular-file observation → bounded read path. Separate ambient metadata and pathname reads would not establish the documented replacement boundary.
- [ ] **Were non-regular leaves denied rather than followed?** Retain scoped test evidence for symlink or non-regular denial when available, without opening the secret through another diagnostic path.
- [ ] **Were Unix creation permissions restricted?** The identity code requests mode `0600`; the permission classifier rejects group/other permission bits. Record the platform. Non-Unix observations can be `Unsupported`, and the source resolver does not equate that enum value with `Unsafe`; do not claim a universal owner-only proof.
- [ ] **Does the bundle contain public metadata rather than secret material?** Review `identity-receipt.preserves`, not `identity/node-endpoint.secret`. The receipt carries operation, decision, node, optional identity/endpoint/previous endpoint/rotation refs, key-source metadata, policy, diagnostic, and checks.
- [ ] **Has redaction been checked beyond one receipt?** Diagnostic logs and operator exports are separate surfaces. The checked-in continuity regression verifies that raw secret-record bytes do not occur in its two receipts; it is not a proof that every future log or encoding is safe.

## Continuity and rotation acceptance

- [ ] **Does a same-root restart retain the expected endpoint?** Require before/after public endpoint IDs and provenance linking them to the same node scope. A receipt's existence alone is insufficient.
- [ ] **If the endpoint changed, was drift denied or rotation explicitly admitted?** The pure observation function denies drift when rotation is disabled, denies a missing rotation receipt, and denies a stale or mismatched supplied ref.
- [ ] **Does the admitted rotation ref match the expected transition?** A syntactically valid content ref is not enough. Keep the old/new endpoint relationship and reviewed policy context, not the secret bytes used to derive them.
- [ ] **Are containment and trust claims separate?** Holding the correct directory authority cannot authorize a peer or make an untrusted policy acceptable.

## Worked rejection case and sign-off

Consider a requested managed-backend deployment with no backend key supplied, an existing persisted secret, and generation allowed. With no explicit key, the resolver reaches required-backend denial before file fallback or generation. Accept the review only if the reported result preserves that denial and does not claim a successful managed deployment because a secret file happened to exist.

Record the entry point, source class, platform permission result, public endpoint transition, canonical receipt refs, and unresolved policy questions. Stop sign-off if source selection, redaction, or rotation binding is unknown. Do not delete persisted identity to regain a passing first-boot result. Secret recovery and authorized rotation require their own reviewed process; local lifecycle success is not permission to bypass it.

## Sources

- [Handbook](../README.md)
- [Node-state authority contract](../../node-state-filesystem-authority.md)
- [Capability-rooted state companion](../../technical/storage/capability-rooted-node-state.md)
- [Daemon identity entry point](../../../src/node/parts/daemon/p045/body.rs)
- [CLI initialization declarations](../../../src/cli/ops/node/command/base.rs)
- [Pure source selection](../../../src/node/parts/identity/p000/body.rs)
- [Shell resolution and rotation observation](../../../src/node/parts/identity/p003/body.rs)
- [Permission and bounded-read helpers](../../../src/node/parts/identity/p004/body.rs)
- [Receipt field construction](../../../src/node/parts/identity/p001/body.rs)
- [Continuity and backend regression sources](../../../src/node/parts/identity/p002/body.rs)
