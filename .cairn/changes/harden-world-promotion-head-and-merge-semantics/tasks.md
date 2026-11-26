# Tasks

- [x] [serial] Redesign `SignedPromotionAuthority` and closure signing (`src/world_promotion/current.rs`, `crates/molten-core/src/world_promotion/`) to bind action, target, nonce/expiry over canonical packed Preserves, and validate live branch head in `src/world_promotion/operator/mod.rs`. r[molten.world_commit.promotion_authority_action_and_canonical_binding]
- [x] [serial] Make reservation+attempt transitions atomic in `src/world_promotion/service.rs` and `store/outbox.rs`, transition reservation state on `abandon_uncertain_for_attempt`, assert immutable fields in `update_reservation`, move `validate_promotion_transaction` inside `begin_write()`, and fix `read_back_promotion` partial-commit and replay handling in `store/transaction.rs`. r[molten.world_commit.promotion_atomic_dispatch_and_readback]
- [ ] [serial] Fix `execute_world_head_transition` (`src/world_head/parts/service/p000/body.rs`) to emit truthful `applied`/`stale`/`uncertain` receipts and fix `p001/body.rs` to derive `role` and `authority_admitted` from signed statements and authority ports. r[molten.world_commit.world_head_truthful_receipts_and_verified_signers]
- [ ] [parallel] Fix `world_merge::reduce_root` equal-root shortcut, enforce `profile.input_schema` in `planning/application.rs`, fix creation/deletion classification in `diff.rs`, and reject unreachable/detached cycles in `worldcommit/restore.rs`. r[molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
- [ ] [serial] Add positive/negative tests for every fixed defect, remove resolved discrepancy notes from `docs/technical/world-effects/promotion-and-reservation-atomicity.md` and `docs/technical/world-state/typed-diff-and-merge-conflicts.md`, and run focused world suites, Clippy, Octet, and strict Cairn validation. r[molten.world_commit.promotion_authority_action_and_canonical_binding] r[molten.world_commit.promotion_atomic_dispatch_and_readback] r[molten.world_commit.world_head_truthful_receipts_and_verified_signers] r[molten.world_commit.merge_conservative_precedence_and_full_graph_closure]

## Verification (2026-11-26, world promotion tasks 1–2)

Audit claims confirmed against the source before changing it: the grant signed only
`(branch, authority, policy, generation, admitted)` as newline-joined text and the closure
comma-joined its intent refs; operator grants were compared with the reservation's historical
generation, never the live head; dispatch and recovery wrote reservation and attempt in separate
Redb transactions; abandon left the reservation `uncertain`; `update_reservation` checked only the
state transition; `validate_promotion_transaction` ran before `begin_write()` against a head read
earlier; `read_back_promotion` returned `Prior` for a partial successor and `Corrupt` once any
reservation left `committed`.

Grants are now `molten.world-promotion.authority-grant.v2` (closed action, exact target, nonce
single use per store — consumed in the applying write transaction — and wall-clock `expires_at`)
signed over `canonical_promotion_grant`; closures are `intent-closure.v2` signed over
`canonical_intent_closure`. Recovery also settles pre-change pairs (abandoned attempt with an
`uncertain`/`attempting` reservation) to `abandoned`. Docs updated:
`docs/world-promotion-current-facts.md` (including the per-store single-use non-claim),
`docs/technical/world-effects/promotion-and-reservation-atomicity.md` (resolved source-boundary
note removed). The typed-diff doc named in the last task belongs to the world-merge slice.

Commands (run on leviathan via `aremote`/`anix`, formatting locally); final run after review fixes:

- `cargo fmt --all` — clean.
- `cargo test -p molten-core --lib world_promotion` — 23 passed, 0 failed.
- `cargo test -p molten --lib world_promotion` — 39 passed, 0 failed.
- `cargo test -p molten --test worldpromotioncli --test worldpromotiondenials` — 5 + 3 passed, 0 failed.
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings` — no warnings or errors.
- `cargo octet check --artifact-dir target/octet` — Status: clean, 0 findings, 0 errors.
- `anix build -L .#checks.x86_64-linux.world-promotion-octet-deny-all` — Status: clean, 0 findings
  (first run found two `excessive_file_length` findings in the core test files; fixed by splitting
  out `tests/grant.rs` and moving one non-claims test into `tests/observation.rs`).
- `cairn validate --root . --policy …/cairn-policy.json --strict` — valid.
