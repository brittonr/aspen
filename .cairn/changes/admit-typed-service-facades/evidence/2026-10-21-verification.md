# Admit typed service, state-machine, and task facades: verification

## What changed

The facade layer is a new library over the existing system-extension interface, in
`crates/molten-core/src/system_extension/facade/`:

- `admission.rs`: the facade manifest profiles. `FacadeProfile` is `service`, `state-machine`, or `task`. A state
  machine carries a phase table with per-phase legal events, postponable events, a postponement bound, and a state
  timeout with its fencing declaration. Admission rejects a phase table for a non-state-machine profile, an empty or
  oversized table, a duplicate or empty phase, a duplicated phase event, unbounded postponement, a postponement bound
  above the maximum, a state timeout without incarnation-and-phase fencing, a state timeout above the maximum, a task
  profile without a completion schema, a completion schema on another profile, a concurrency limit below one, and
  malformed or missing refs.
- `lowering.rs`: the admitted event, the runtime state, the application's model turn, and the lowered turn. The facade
  admits the event (phase legality, bounded postponement, the turn-sequence serialization check, and the completed-task
  check), admits a fired timer (a timer from an earlier phase or an earlier incarnation is stale), and lowers the model
  turn: timers bind the phase and incarnation, effects bind the executing generation and incarnation and must name a
  bound port, and the next phase must be declared. `lower_facade_outcome` lowers the turn into the callback outcome the
  interface already defines.
- `semantics.rs`: the profile rules. A completed task maps to the existing stopped transition and consumes no restart
  budget.
- `tests.rs`: the admission, lowering, and equivalence fixtures.

No core transition semantics changed. The facade composes with the admitted extension manifest and reuses the existing
canonical envelopes, the existing lifecycle transitions, and the existing incarnation fencing.

## Validation

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings` over the workspace: exit 0.
- `cargo test -p molten-core --lib system_extension::facade`: 11 passed, 0 failed. The fixtures cover a minimal
  admitted manifest per profile, the rejected admission shapes, an illegal event for a phase, a stale-state timer from an
  earlier phase and from an earlier incarnation, the postponed event with a stale timer, the bounded postponement that
  dead-letters, the parallel event that cannot interleave inside one turn, the completed task that stops without
  consuming restart budget, the state timeout fenced by phase and incarnation, and the equivalence between a
  facade-lowered turn and a hand-written turn.
- `cargo test --workspace`: exit 0. The molten library suite reported 1,652 passed, the core suite 492
  passed, and the native system-extension integration suite 9 passed.
- `cargo octet check --artifact-dir target/octet-precommit`: `Status: clean`, 0 findings, 0 warnings, 0
  errors, with `cargo-octet 0.1.0` on `nightly-2026-03-21`.
- `nix run "path:$CAIRN_ROOT#cairn" -- validate --root . --policy
  "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict`: exit 0 with no findings,
  `spec_findings: []` and `substance_findings: []`.
- `cargo nextest` cannot run in this checkout: `cargo-nextest` always calls
  `cargo metadata --all-features`, and that call panics in the pinned cargo on the optional `rad://` sources. The
  panic reproduces on the unmodified tree and is unrelated to this change, which touches no manifest. The documented
  Cargo fallback was used.

## Non-claims

- A facade adds no runtime, no I/O route, and no state store. It does not change failure isolation, authority, or
  durability guarantees; those remain owned by the existing contracts.
- Resemblance to `GenServer` or `gen_statem` is conceptual prior art, not API compatibility.
- The facade is opt-in. Nothing in this change turns validation, hashing, policy evaluation, or planning into actors,
  and no existing extension was retrofitted.
- The equivalence fixture compares the facade's lowered envelopes with a hand-written turn in the same fixture. It does
  not prove that any production extension behaves the same.
