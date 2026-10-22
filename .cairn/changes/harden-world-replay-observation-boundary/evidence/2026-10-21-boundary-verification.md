# Replay observation boundary: baseline and verification

## Baseline before the change

Run before any edit (`cargo test -p molten-core --lib -- world_replay`,
`cargo test -p molten --lib -- world_replay`):

- Core: 9 passed. These cover stable two-step logical replay planning,
  earliest typed root divergence (first divergence by position, root kind,
  and field path), malformed field paths, parent and profile drift,
  reordered steps, stale schema, unsupported profile, exact opaque cohort and
  descriptor, missing and extra closure, and canonical member order.
- Shell: 11 passed. These cover logical replay through the ports with
  bounded receipts, stopping at the first divergence, a denial receipt before
  execution, exact opaque replay without logical fallback, canonical import
  and export, and the sealed reproduction and content-manifest boundaries.

Baseline gaps the change closes:

- No inventory existed. Time (`RootKind::Time`), entropy
  (`RootKind::Entropy`), the scheduler (`SchedulerReplayPolicy`), and effects
  (`EffectKind`, `RootKind::Effects`) were restored as roots. Hostcalls
  (`RuntimeHostcall`), asynchronous deliveries (`SchedulerChoiceKind`), and
  external reads had no declared replay treatment.
- `WorldReplayTransitionPort::execute_transition` received only the step.
  Nothing supplied recorded observations or prevented an adapter from
  performing the original effect again.

## After the change

- Core: 23 passed. The 9 baseline tests plus 14 boundary tests:
  - complete admission and a stable plan, with the boundary admitted before
    execution;
  - unknown source and omitted inventory row;
  - unsupported source;
  - missing result, extra value, and duplicate event;
  - result bound to another request;
  - earliest reordering divergence, with logical-position dispatch despite
    inverted arrivals;
  - profile, adapter, and secrecy drift;
  - tampered observation and ledger;
  - an external mutation declared replayable;
  - a detached opaque diagnostic that satisfies nothing, plus overclaim and
    cohort negatives;
  - a blocked boundary and an unbound ledger block the plan;
  - content identity over locators;
  - sharing only after identity, immutability, policy, and readback;
  - a bounded summary, and overclaim, dropped non-claim, and
    replay-after-effect negatives.
- Shell: 15 passed. The 11 baseline tests now run against sealed
  observations, plus 4 new tests:
  - the runtime catalog binds every hostcall, effect kind, and asynchronous
    kind once;
  - replay serves sealed observations in logical order and never repeats the
    send (`sealed_effects = 1`, `repeated_effect_attempts = 0`);
  - an adapter that attempts the original effect fails the replay;
  - unknown, reordered, misbound, or skipped demands fail the replay.

## Open

`[depends:add-native-process-record-replay-profile]`: ChaosControl has not
published that profile; there is no change of that name in the ChaosControl
repository. Generic detached opaque diagnostics are implemented and tested
(cohort-bound, detached-only, never satisfying an observation). Binding the
specific ChaosControl native-process descriptor format waits for its
contract.

## Non-claims

Passing replay shows that the selected Molten adapters reproduced the recorded
semantic observations and expected transitions for one profile, trace,
horizon, and cohort set. It does not prove arbitrary process determinism,
kernel or native-process replay, external effect completion, host security,
authority, or release eligibility.

## Gates run

- `cargo test --workspace --lib`: 1710 (`molten`) and 551 (`molten-core`)
  passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo octet
  check`: clean.
- `nix build .#checks.x86_64-linux.world-replay-octet-deny-all
  .#checks.x86_64-linux.world-promotion-octet-deny-all`: built. Both focused
  deny-all crates compile the core `world_replay` module.
- The canonical plan record gained `inventory-ref` and `ledger-ref` (arity 8
  to 10). The replay receipt gained `observation-boundary` (arity 18 to 19).
  The boundary schema artifacts are updated to match.
- Cairn strict validation: no spec or substance findings. Tracey guard: pass
  (2929 requirements, 995 referenced, 1934 inherited uncovered, 0 dangling).
