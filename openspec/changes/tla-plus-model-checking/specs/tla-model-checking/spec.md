## ADDED Requirements

### Requirement: Safety invariants for trust protocol

TLA+ specs MUST define and model-check safety invariants that hold in ALL reachable states of the trust protocol.

#### Scenario: Share consistency

- **WHEN** a node has committed a configuration at epoch E
- **THEN** the invariant `CommittedNodesHaveShares` holds: the node has a share for epoch E in its persistent state

#### Scenario: Epoch monotonicity

- **WHEN** a node accepts a new configuration at epoch E
- **THEN** E is strictly greater than any previously committed epoch on that node

#### Scenario: Expungement permanence

- **WHEN** a node has been marked as expunged
- **THEN** it remains expunged in all subsequent states (no transition out of expunged state)

#### Scenario: Configuration consistency

- **WHEN** two nodes have configurations for the same epoch
- **THEN** the configurations are identical (same members, same coordinator, same threshold)

### Requirement: Liveness properties

TLA+ specs MUST define liveness properties that eventually hold under weak fairness assumptions.

#### Scenario: Reconfiguration completes

- **WHEN** a reconfiguration is initiated and a quorum of nodes is available
- **THEN** eventually all nodes in the new configuration have committed and hold shares for the new epoch

#### Scenario: Expunged node notified

- **WHEN** a node is removed from the configuration and a quorum is available
- **THEN** eventually the removed node receives an Expunged message (directly or via peer enforcement)

### Requirement: Crash recovery modeled

TLA+ specs MUST model node crashes and restarts, verifying that the protocol recovers correctly.

#### Scenario: Leader crash during share collection

- **WHEN** the coordinator crashes after collecting some but not all shares
- **THEN** a new coordinator can restart the reconfiguration from scratch

#### Scenario: Node crash after prepare but before commit

- **WHEN** a node crashes after receiving a Prepare but before the commit
- **THEN** on restart it can recover by collecting shares from peers or receiving a CommitAdvance

### Requirement: CI integration

TLC model checking MUST run as part of `nix flake check` and `nix run .#check-tla`.

#### Scenario: CI catches invariant violation

- **WHEN** a code change introduces a protocol bug that violates a TLA+ invariant
- **THEN** `nix flake check` fails with the TLC counterexample trace

#### Scenario: Hermetic execution

- **WHEN** `nix run .#check-tla` is executed
- **THEN** it runs TLC without requiring a pre-installed Java runtime (Nix provides it)

### Requirement: Spec-to-code mapping documented

Each TLA+ variable and invariant MUST have a documented correspondence to its Rust implementation.

#### Scenario: Developer reads a TLA+ invariant

- **WHEN** a developer reads `CommittedNodesHaveShares` in the TLA+ spec
- **THEN** a comment identifies the corresponding Rust assertion or test in the trust protocol code
