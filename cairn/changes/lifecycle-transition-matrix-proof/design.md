# Design: lifecycle transition matrix proof

## Scope

This change proves the finite lifecycle transition gate by enumerating the state graph and action-target relation. It covers `State`, `Action`, `allowed_transition`, `action_matches_target`, and `transition_receipt` decisions for lifecycle transitions.

## Proof checklist

- **Proof claim**: a lifecycle transition receipt passes exactly when the `(from_state, to_state)` edge is allowed and the action is valid for the target state, including the explicit supervisor-decision escape hatch.
- **Out of scope**: adapter process behavior, service supervision policy, and distributed runtime scheduling.
- **Trusted assumptions**: the state and action enum variant lists are finite and fully exposed to the test matrix.
- **Positive evidence**: every allowed edge with a matching action produces a passing receipt.
- **Negative evidence**: every unlisted edge or mismatched action produces a denying receipt.
- **Canonical refs**: receipt refs remain BLAKE3 canonical refs produced by existing lifecycle receipt rendering.
- **Regeneration command**: `cargo test lifecycle`.

## Functional core

The proof should prefer pure helpers that expose lifecycle states, actions, allowed edges, and action-target compatibility. Tests should not construct adapters or runtime shells to prove the decision law.

## Non-goals

- No compatibility claim for BEAM, OTP, or Lunatic lifecycle semantics.
- No new lifecycle states or actions unless a later change explicitly extends the graph.
