# Verification: session-owned remote assertions

## Baseline

The pre-change checkout at `38cb87acd` (branch `molten`) had no session registry, no
applied-assertion record, and no close or disconnect cleanup for remote dataspace
deliveries: `apply_delivered_envelope` applied every envelope with the peer-derived
actor id, and nothing retracted a peer's facts when its session ended.

## Implementation evidence

- `src/remote/parts/dataspace/p006/body.rs`: `RemoteSessionRegistry` with a bounded
  session count, canonical session refs (`remote_session_ref`), the declared session
  owner (`session:{peer}/{topic}:{generation}`), admission that denies an unknown or
  closed declared owner before staging, `apply_delivered_envelope_owned_by`, and the
  applied-assertion record that carries owner, session ref, session state, envelope
  ref, payload ref, and binding checks.
- `src/remote/parts/dataspace/p007/body.rs`: `close_remote_session` (disconnect and
  stop causes) retracts the session owner's assertions through `RuntimeStep::Retract`,
  runs the existing `cleanup_actor_scope` plus `scope_cleanup_receipt` lifecycle path,
  records before/after state refs, and marks the session closed.
  `replay_delivery_log_for_session` keeps replays for an unknown or closed session
  diagnostic-only, so a closed session's assertions cannot be resurrected.
- `src/preserves/parts/rail/p000/body.rs`: `molten.remote-dataspace.session.v1` and
  `molten.remote-dataspace.applied-assertion.v1` schema ids.
- `src/ledger/parts/mod/artifacts/p000/body.rs`: ledger classification for the new
  records.
- `docs/architecture.md`: remote dataspace section documents the ownership rule, the
  reconnect rule, and the no-delivery-completeness non-claim.
- Tests: `src/remote/parts/dataspace/tests/m000/p003/body.rs`.

## Checks

Executed in `/home/brittonr/git/OnixResearch/aspen-w1`:

- `cargo fmt --all` then `cargo fmt --check`: clean.
- `cargo check --workspace --all-targets`: no errors.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p molten --lib`: 1465 passed.
- `cargo test -p molten --lib -- dataspace`: 23 passed.
- `cargo octet check --artifact-dir target/octet-remote-session-2`: 3617 findings,
  0 errors, `warning-only` — two below the pre-change baseline of 3619. The first
  revision added two findings (`excessive_file_length` and `no_unwrap` in the new
  part); splitting the part at the close/replay boundary and replacing the `expect`
  with a typed denial returned the family to baseline.
- Cairn: `validate --root . --strict` valid; `gate proposal|design|tasks` valid for
  this change.

Not run: Nix checks (see caveats).

## Traceability

- `r[molten.runtime_spine.remote_assertion_ownership]`:
  - ownership and evidence readability:
    `remote_assertion_is_session_owned_and_owner_is_readable_from_evidence`;
  - disconnect retraction and observer notification:
    `session_close_retracts_peer_facts_and_notifies_observers`;
  - late delivery and unknown-owner denial before staging:
    `late_delivery_to_closed_session_denies_before_staging` and
    `unknown_declared_owner_denies_before_staging`;
  - session identity is not reused after close:
    `session_identity_cannot_be_reused_after_close`;
  - replay does not resurrect a closed session's assertions:
    `replay_for_closed_session_stays_diagnostic_and_requires_reassertion`, with
    `open_session_replay_matches_observed_session_apply` as the positive replay
    control.

## Caveats

- The tests drive recorded local-gossip publish/deliver and recorded delivery logs.
  No live iroh transport session, reconnect handshake, or peer restart was exercised.
- Nix checks were not run; no dependency, lockfile, or flake input changed.

## Claim boundary

The evidence shows session ownership, denial, cleanup, and replay semantics over the
supplied recorded deliveries and runtime state. It does not prove delivery
completeness, live transport behavior, reconnect fact restoration, or release
readiness.
