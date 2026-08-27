## Context

Molten uses UCAN, Basalt, capability contexts, effect handles, runtime profiles, and durable authority observations. These mechanisms decide current use of external capabilities.

World branching creates a new question. The runtime must decide which authority can accompany a new branch and under which obligations.

A stored commit identity cannot answer this question. Branch authority requires current policy and stateful enforcement.

## Decisions

### Decision: Basalt owns portable branch-policy meaning

**Choice:** Consume a pinned Basalt policy cohort with closed branch modes and deterministic decisions. Molten maps its native capability facts into that contract.

Molten retains parsing, token handling, currentness, derivation, transfer, adapters, persistence, and effects.

**Rationale:** Policy review belongs in Basalt. Runtime enforcement belongs with the product that controls effects.

### Decision: A world commit stores observations, not live capabilities

**Choice:** World commits may reference metadata-only authority observations. They exclude bearer tokens, keys, credentials, capability paths, private policy bodies, and secret entropy.

Activation always obtains and validates current authority outside the commit.

**Rationale:** Content-addressed cloning must not duplicate or disclose authority.

### Decision: Use closed branch modes

**Choice:** Molten maps Basalt decisions into these obligations:

- `copyable` requires an explicit independently valid grant for the destination.
- `attenuated` requires a verifiable narrower derivation.
- `linear` requires fenced transfer and source deactivation.
- `simulation_only` binds a deterministic non-live adapter.
- `promotion_gated` records an intent that cannot dispatch before admitted promotion.
- `replace_before_activation` requires a new current grant.
- `non_branchable` denies branch activation.

Unknown modes deny.

**Rationale:** A Boolean copy flag cannot express exclusivity, attenuation, simulation, or promotion rules.

### Decision: Plan in the core and enforce in the shell

**Choice:** The pure core validates supplied normalized facts and returns a derivation plan with obligations. It performs no token minting, storage, clock, network, secret, or adapter effects.

The shell realizes obligations through explicit ports and returns observations for final activation admission.

**Rationale:** Policy decisions and planned effects do not prove that authority changed.

### Decision: Fence linear transfers durably

**Choice:** A linear capability transfer binds source branch, destination branch, capability identity, expected durable generation, and one operation identity.

The mutation boundary marks the source unavailable before destination activation. Unknown outcomes require observation-first reconciliation.

**Rationale:** Copying a linear authority would violate exclusivity. Blind retry can create ambiguous ownership.

### Decision: Simulation adapters cannot fall back to live effects

**Choice:** A simulation-only plan binds an exact deterministic adapter profile. Missing simulation support denies activation.

No live adapter fallback is allowed during replay, exploration, or branch inspection.

**Rationale:** Simulation must not leak branch effects into production systems.

### Decision: Recheck at activation and promotion

**Choice:** Branch creation can record a plan, but branch activation and promotion each recheck policy, capability, revocation, replay, scope, adapter, and durable ownership facts.

A stale passing receipt cannot authorize later use.

**Rationale:** Authority changes independently from immutable world state.

## Rollout

1. Pin the Basalt policy and add pure mapping fixtures.
2. Support simulation-only and non-branchable modes first.
3. Add attenuated and replace-before-activation paths.
4. Add linear transfer only after durable unknown-outcome tests pass.
5. Connect promotion-gated decisions to the effect-release change.

## Risks / Trade-offs

- Linear transfer failure can temporarily block both branches. Explicit recovery is safer than duplicate activation.
- Consumer mappings can lose native meaning. Lossy mappings must deny.
- Simulation adapters can drift from live behavior. Their receipts must state bounded conformance only.
- Policy acceptance does not prove token derivation or runtime enforcement.
