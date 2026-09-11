# Runtime spine: service demand lifetime delta

## ADDED Requirements

### Requirement: Service demand is retractable state
r[molten.runtime_spine.demand_assertion_lifetime] Molten MUST represent service demand as a retractable assertion owned by the demanding scope, MUST deduplicate demand per owner and service, MUST keep dependency-gated and force-run demand distinct, MUST make shutdown eligible only when the last live demand is withdrawn, and MUST keep a restart request a message.

#### Scenario: Withdrawing demand allows shutdown
- GIVEN a running service that one owner demands
- WHEN that demand assertion retracts
- THEN shutdown becomes eligible and still passes the normal shutdown admission gates.

#### Scenario: Another demander keeps the service alive
- GIVEN a running service demanded by two owners
- WHEN one owner withdraws its demand
- THEN the service stays running.

#### Scenario: Force-run demand is explicit
- GIVEN a demand record that declares force-run
- WHEN dependency readiness is unmet
- THEN startup proceeds only because the record declared force-run, and the declaration appears in the decision evidence.

#### Scenario: Restart is not demand
- GIVEN a restart request delivered as a message
- WHEN the runtime processes it
- THEN demand state is unchanged and the restart path uses its own retry and restart records.
