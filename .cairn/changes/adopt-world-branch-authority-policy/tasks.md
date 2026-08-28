## Phase 1: Policy adoption and pure planning

- [x] [depends:add-world-branch-authority-policy] Pin Cargo and Nix to the same reviewed Basalt branch-authority policy revision and record exact source and license evidence. r[molten.world_branch_authority.adoption]
- [x] [serial] Define Molten normalized capability facts, branch actions, policy decisions, derivation obligations, realization observations, activation decisions, and diagnostics. r[molten.world_branch_authority.derivation]
- [x] [depends:world-branch-authority-dtos] Implement pure Basalt mapping, decision validation, obligation planning, unknown-mode denial, and lossy-mapping rejection. r[molten.world_branch_authority.adoption] r[molten.world_branch_authority.derivation]
- [x] [parallel] Add canonical Preserves plan, observation, activation, transfer, and metadata-only receipt schemas. r[molten.world_branch_authority.evidence]

## Phase 2: Runtime enforcement

- [x] [depends:world-branch-authority-core] Add narrow current policy, UCAN, revocation, replay, scope, durable ownership, derivation, transfer, adapter, and activation ports. r[molten.world_branch_authority.activation]
- [x] [depends:world-branch-authority-ports] Implement simulation-only, non-branchable, copyable, attenuated, and replace-before-activation realization paths. r[molten.world_branch_authority.derivation] r[molten.world_branch_authority.simulation]
- [x] [depends:world-branch-authority-ports] Implement generation-fenced linear transfer with source deactivation, destination activation, and observation-first recovery. r[molten.world_branch_authority.linear]
- [ ] [depends:bind-world-promotion-to-effect-release] Bind promotion-gated decisions to release reservation admission without authorizing dispatch from the policy receipt. r[molten.world_branch_authority.activation]
- [x] [depends:world-branch-authority-realization] Add branch plan, authority-inspect, activate, transfer, simulate, and recovery commands with safe bounded output. r[molten.world_branch_authority.evidence]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive fixtures for every supported branch mode, exact attenuation, safe simulation, linear transfer, and current activation. r[molten.world_branch_authority.verification]
- [x] [parallel] Add negative unknown mode, authority widening, missing derivation, linear copy, double activation, stale generation, unknown transfer outcome, stale policy, revoked UCAN, replayed proof, simulation fallback, promotion bypass, lossy mapping, bearer disclosure, and receipt-as-authority fixtures. r[molten.world_branch_authority.verification]
- [x] [serial] Document capability non-cloning, policy and enforcement ownership, linear recovery, simulation limits, and activation currentness. r[molten.world_branch_authority.evidence]
- [x] [depends:world-branch-authority-verification] Run focused tests, Basalt and Durable Authority State compatibility fixtures, Octet, Clippy with warnings denied, Cairn gates, lifecycle checks, and relevant Nix checks. r[molten.world_branch_authority.verification]
