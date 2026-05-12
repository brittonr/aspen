## ADDED Requirements

### Requirement: Stock cluster VM checks are durable evidence rails [r[test-harness-runtime.stock-cluster-vm-checks]]

Aspen MUST keep the stock cluster VM checks runnable as focused operator evidence targets for repository-supported clustering behavior.

#### Scenario: MicroVM cluster check reaches product assertions [r[test-harness-runtime.stock-cluster-vm-checks.microvm-runs]]

- GIVEN an operator runs `nix build --impure .#checks.x86_64-linux.microvm-cluster-test --no-link -L` on a host with the required VM capabilities
- WHEN the check builds its closure successfully
- THEN it SHALL execute the VM test script instead of failing before VM execution due dependency or packaging drift
- AND it SHALL assert the intended multi-node cluster and AspenFs connection behavior

#### Scenario: Multi-node cluster check exposes a core cluster proof boundary [r[test-harness-runtime.stock-cluster-vm-checks.core-boundary]]

- GIVEN an operator runs `nix build --impure .#checks.x86_64-linux.multi-node-cluster-test --no-link -L`
- WHEN node startup, cluster initialization, learner addition, and voter promotion succeed
- THEN the check SHALL emit or record an explicit core-cluster proof boundary before optional plugin/forge assertions run
- AND a later optional plugin/forge failure SHALL NOT be reported as failure to boot or form the Raft/Iroh cluster

#### Scenario: Focused VM failures are classified by boundary [r[test-harness-runtime.stock-cluster-vm-checks.failure-classification]]

- GIVEN a stock cluster VM check fails
- WHEN evidence is reported or preserved
- THEN the failure SHALL be classified as one of build closure, host VM capability, VM boot/service readiness, cluster formation, optional plugin/forge fixture, or post-formation product assertion
- AND cluster tickets or equivalent secrets SHALL be redacted from committed evidence
