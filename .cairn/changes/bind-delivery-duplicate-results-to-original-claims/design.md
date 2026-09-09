# F08 design

## Boundary and source

Source revision: `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`crates/molten-core/src/coordination_delivery/transition/support.rs::duplicate_transition` finds a token through `existing.item_ref` and current `in_flight`. It does not match `AppliedDeliveryOperation.token_ref`. `transition.rs` already saves the original token reference.

`src/coordination_delivery/service.rs::apply_delivery_request` returns non-applied transitions without compare-and-commit. The accepted delivery specification requires original semantic-result binding and duplicate suppression. Coordination-delivery docs retain separate current-token and completion-owner checks.

## Proposed decision

The pure core owns duplicate identity and response reconstruction. It returns a token only if its identity matches the saved operation token. A current token for the same item is not sufficient.

Prefer the existing optional token representation. If the original token cannot be reconstructed, return no token while retaining the original operation reference. This preserves evidence without storing unbounded token history. Parent review must accept the explicit token-unavailable meaning before implementation. A richer typed diagnostic needs separate canonical compatibility review.

The shell keeps no-commit behavior for duplicate and denied requests. It emits no timer intent, worker dispatch, or replacement claim. Adapters preserve the optional token and original operation reference in canonical records. No new dependency or trivial internal port is needed.

A duplicate response is not a renewed lease. Even an original token remains subject to current fencing, owner, deadline, and delegated completion checks. Rejection preserves queue state, timers, and durable revision.

## Compatibility and replay

Prefer unchanged state and token schemas. The saved token reference supports exact identity comparison. The public response changes from an incorrect later token to the original token or `None`.

Parent review must settle caller handling of unavailable original tokens and add explicit documentation. Old records remain readable. Replay tests must keep request and original operation identity stable and reject result substitution. Do not silently rewrite stored historical receipts.

## Coordination and order

F03 owns ingress dedup and enqueue publication, not coordination claim response reconstruction. F12 owns generic exponential retry arithmetic. F08 uses the existing fixed-delay profile and requires neither change.

The replication packages share the distinction between historical results and current state, but no implementation dependency exists. Keep package scopes separate and avoid a shared generic history abstraction without matching contracts.

## Validation and ownership

Coordination-delivery maintainers own core, service, Redb adapter, and docs. Run focused existing duplicate and completion tests before edits. Move the named F08 reproduction into normal core tests with named tick and epoch constants.

Cover immediate exact replay, expiry and reclaim, completed items, unavailable original tokens, changed requests, and non-claim duplicates. Add service tests for no commit or timer effects. Add Redb reopen and canonical round-trip tests for original binding and malformed records.

Retain wrong-owner, stale-token, and delegated-completion positive and negative controls. Run focused Octet and Clippy, workspace tests, relevant Nix checks, and required Cairn gates without weakening checks. This plan supplies no authority-bypass or acceptance claim.
