# Design: Nickel/Basalt policy preflight

## Context

The harness policy frontend remains the small Preserves `<policy-v1 ...>` deny-rule syntax. That syntax is now treated as compatibility input to a static policy boundary rather than the final evidence artifact. Runtime execution must not start until the policy has been normalized and accepted by explicit preflight evidence.

## Static Nickel boundary

For each suite policy, the runner derives a deterministic Nickel source document with:

- `schema_version = "molten.harness.policy.nickel-static.v1"`
- the harness policy schema id
- the canonical Preserves policy ref
- ordered deny-rule records with optional actor/action/target/value fields
- Preserves value text plus canonical value refs for value-bearing rules

The source is evaluated/exported with `nickel-lang` using `eval_deep_for_export` and JSON export. The policy gate records both the Nickel source ref and the Nickel export ref. Validation re-runs the Nickel export and rejects mismatched or stale export evidence.

## Basalt boundary

The runner constructs a Basalt `ContractEnvelope` with backend `nickel`, contract id `molten.harness.admission-policy`, version `v1`, the Nickel source ref as normalized source hash, the admission request schema, the admission-decision receipt schema, and the Basalt preflight receipt schema. It calls Basalt envelope validation before emitting the gate.

The policy gate embeds a `<basalt-preflight ...>` receipt with decision, backend, contract id, envelope ref, policy ref, normalized source ref, and Basalt reason. Validation checks all bindings and recomputes the expected gate from the embedded suite policy.

## Steel predicates

Steel/dynamic predicates remain disallowed in local harness policy fixtures and policy gates unless future reviewed callable receipts are introduced. Non-empty Steel predicate lists fail closed.

## Gate receipts

Pass-evidence receipts include policy artifact refs for the canonical policy, policy gate, Nickel source, Nickel export, and Basalt preflight receipt. Checks include Nickel source/export normalization and Basalt receipt binding in addition to the existing policy-preflight checks.
