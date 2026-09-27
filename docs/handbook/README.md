# Molten workflow handbook

One hundred task-oriented documents for contributors, integrators, and operators. This collection complements the [technical companion](../technical/README.md), which explains implementation laws and boundaries. Use the [documentation map](../README.md) to distinguish governing documents from these source-linked explanations.

## Choose a document mode

| Mode | Use it when you need to… |
| --- | --- |
| Walkthrough | Follow one concrete fixture or implementation path from inputs to observations. |
| How-to | Make a bounded change or inspection with explicit prerequisites and stop conditions. |
| Reference | Look up the meaning and ownership of commands, fields, or artifacts. |
| Troubleshooting | Distinguish similar symptoms before selecting a safe next action. |
| Review checklist | Check whether a proposed change or operational claim has the right evidence. |

Each of the twenty topics below contains one document in each mode. A walkthrough may be a source walkthrough rather than an executable tutorial: the article states which kind it is. A checked-in fixture demonstrates its declared boundary, not production readiness.

## Execution and safety conventions

Commands are source-checked guidance, not reported executions. Follow the prerequisites in each article, use fresh isolated output locations, and retain failure artifacts. A source link proves where a spelling or behavior was inspected; it does not prove the command will succeed in your environment. Missing dependency transports, credentials, live adapters, current authority, or evidence are blockers to identify—not invitations to bypass gates.

Read decisions inside canonical receipts rather than treating successful parsing, command exit, or rendered output as equivalent to admission. Uncertain effects are not safe-to-retry failures. An old passing receipt is not current authority. Documentation review and link checks do not produce canonical behavioral evidence.

## Suggested routes

- **New contributor:** Contributing → CLI navigation → Harness → Replay workflows.
- **Data and artifact tooling:** Artifact workflows → Typed data → Catalog workflows → Upgrade workflows.
- **Distributed integration:** Job workflows → Remote workflows → Protocol workflows → Policy workflows.
- **Local operations:** Node workflows → Cluster workflows → Delivery workflows → Evidence workflows.
- **Runtime integration:** Extension workflows → World workflows → Retention workflows → Release workflows.

## Topics

### Contributing

- **Walkthrough:** [Checkout to first check](contributing/checkout-to-first-check.md)
- **How-to:** [Choosing a focused test](contributing/choosing-a-focused-test.md)
- **Reference:** [Workspace and toolchain reference](contributing/workspace-and-toolchain-reference.md)
- **Troubleshooting:** [Diagnosing development environment failures](contributing/diagnosing-development-environment-failures.md)
- **Review checklist:** [Preparing a reviewable change](contributing/preparing-a-reviewable-change.md)

### Cli navigation

- **Walkthrough:** [Tracing a command to its handler](cli-navigation/tracing-a-command-to-its-handler.md)
- **How-to:** [Choosing fixture versus live commands](cli-navigation/choosing-fixture-versus-live-commands.md)
- **Reference:** [Command families and artifact reference](cli-navigation/command-families-and-artifact-reference.md)
- **Troubleshooting:** [Diagnosing cli input and output failures](cli-navigation/diagnosing-cli-input-and-output-failures.md)
- **Review checklist:** [Reviewing a new cli surface](cli-navigation/reviewing-a-new-cli-surface.md)

### Harness

- **Walkthrough:** [Reading and running a checked in suite](harness/reading-and-running-a-checked-in-suite.md)
- **How-to:** [Building on an existing harness fixture](harness/building-on-an-existing-harness-fixture.md)
- **Reference:** [Suite input and report reference](harness/suite-input-and-report-reference.md)
- **Troubleshooting:** [Diagnosing suite admission failures](harness/diagnosing-suite-admission-failures.md)
- **Review checklist:** [Reviewing positive and negative scenarios](harness/reviewing-positive-and-negative-scenarios.md)

### Replay workflows

- **Walkthrough:** [Following a run into replay](replay-workflows/following-a-run-into-replay.md)
- **How-to:** [Comparing two recorded runs](replay-workflows/comparing-two-recorded-runs.md)
- **Reference:** [Replay input and divergence reference](replay-workflows/replay-input-and-divergence-reference.md)
- **Troubleshooting:** [Diagnosing effect log rejection](replay-workflows/diagnosing-effect-log-rejection.md)
- **Review checklist:** [Reviewing a counterexample for reuse](replay-workflows/reviewing-a-counterexample-for-reuse.md)

### Artifact workflows

- **Walkthrough:** [Following an artifact through the registry](artifact-workflows/following-an-artifact-through-the-registry.md)
- **How-to:** [Inspecting dependency closure before use](artifact-workflows/inspecting-dependency-closure-before-use.md)
- **Reference:** [Artifact and binding reference](artifact-workflows/artifact-and-binding-reference.md)
- **Troubleshooting:** [Diagnosing missing or incompatible artifacts](artifact-workflows/diagnosing-missing-or-incompatible-artifacts.md)
- **Review checklist:** [Reviewing artifact import boundaries](artifact-workflows/reviewing-artifact-import-boundaries.md)

### Typed data

- **Walkthrough:** [Following a typed value through storage](typed-data/following-a-typed-value-through-storage.md)
- **How-to:** [Checking schema compatibility before read](typed-data/checking-schema-compatibility-before-read.md)
- **Reference:** [Storage schema and cache reference](typed-data/storage-schema-and-cache-reference.md)
- **Troubleshooting:** [Diagnosing cache and schema disagreement](typed-data/diagnosing-cache-and-schema-disagreement.md)
- **Review checklist:** [Reviewing a data migration plan](typed-data/reviewing-a-data-migration-plan.md)

### Catalog workflows

- **Walkthrough:** [Following a catalog query](catalog-workflows/following-a-catalog-query.md)
- **How-to:** [Inspecting catalog results without executing artifacts](catalog-workflows/inspecting-catalog-results-without-executing-artifacts.md)
- **Reference:** [Catalog and mcp surface reference](catalog-workflows/catalog-and-mcp-surface-reference.md)
- **Troubleshooting:** [Diagnosing catalog identity and access failures](catalog-workflows/diagnosing-catalog-identity-and-access-failures.md)
- **Review checklist:** [Reviewing catalog exposure and redaction](catalog-workflows/reviewing-catalog-exposure-and-redaction.md)

### Upgrade workflows

- **Walkthrough:** [Following an upgrade plan](upgrade-workflows/following-an-upgrade-plan.md)
- **How-to:** [Inspecting a transcript and rewrite recipe](upgrade-workflows/inspecting-a-transcript-and-rewrite-recipe.md)
- **Reference:** [Upgrade task and compatibility reference](upgrade-workflows/upgrade-task-and-compatibility-reference.md)
- **Troubleshooting:** [Diagnosing blocked session drains](upgrade-workflows/diagnosing-blocked-session-drains.md)
- **Review checklist:** [Reviewing cutover and rollback evidence](upgrade-workflows/reviewing-cutover-and-rollback-evidence.md)

### Job workflows

- **Walkthrough:** [Following a ref backed job](job-workflows/following-a-ref-backed-job.md)
- **How-to:** [Preparing a recorded worker exchange](job-workflows/preparing-a-recorded-worker-exchange.md)
- **Reference:** [Job dag and result reference](job-workflows/job-dag-and-result-reference.md)
- **Troubleshooting:** [Diagnosing worker admission and result failures](job-workflows/diagnosing-worker-admission-and-result-failures.md)
- **Review checklist:** [Reviewing job effects and retry safety](job-workflows/reviewing-job-effects-and-retry-safety.md)

### Remote workflows

- **Walkthrough:** [Following an artifact exchange](remote-workflows/following-an-artifact-exchange.md)
- **How-to:** [Inspecting a remote dataspace session](remote-workflows/inspecting-a-remote-dataspace-session.md)
- **Reference:** [Exchange and federation reference](remote-workflows/exchange-and-federation-reference.md)
- **Troubleshooting:** [Diagnosing remote identity and closure failures](remote-workflows/diagnosing-remote-identity-and-closure-failures.md)
- **Review checklist:** [Reviewing a peer sharing boundary](remote-workflows/reviewing-a-peer-sharing-boundary.md)

### Protocol workflows

- **Walkthrough:** [Following a protocol session](protocol-workflows/following-a-protocol-session.md)
- **How-to:** [Inspecting control plane coordination](protocol-workflows/inspecting-control-plane-coordination.md)
- **Reference:** [Protocol and coordination reference](protocol-workflows/protocol-and-coordination-reference.md)
- **Troubleshooting:** [Diagnosing session transition denials](protocol-workflows/diagnosing-session-transition-denials.md)
- **Review checklist:** [Reviewing consensus scope and evidence](protocol-workflows/reviewing-consensus-scope-and-evidence.md)

### Policy workflows

- **Walkthrough:** [Following policy preflight](policy-workflows/following-policy-preflight.md)
- **How-to:** [Inspecting current authority before an effect](policy-workflows/inspecting-current-authority-before-an-effect.md)
- **Reference:** [Policy capability and resource input reference](policy-workflows/policy-capability-and-resource-input-reference.md)
- **Troubleshooting:** [Diagnosing denied authority contexts](policy-workflows/diagnosing-denied-authority-contexts.md)
- **Review checklist:** [Reviewing a policy change with negative cases](policy-workflows/reviewing-a-policy-change-with-negative-cases.md)

### Node workflows

- **Walkthrough:** [Following an isolated node lifecycle](node-workflows/following-an-isolated-node-lifecycle.md)
- **How-to:** [Inspecting node control evidence](node-workflows/inspecting-node-control-evidence.md)
- **Reference:** [Node artifact and state root reference](node-workflows/node-artifact-and-state-root-reference.md)
- **Troubleshooting:** [Diagnosing restart and lock inconsistency](node-workflows/diagnosing-restart-and-lock-inconsistency.md)
- **Review checklist:** [Reviewing node identity and secret handling](node-workflows/reviewing-node-identity-and-secret-handling.md)

### Cluster workflows

- **Walkthrough:** [Following the two node harness](cluster-workflows/following-the-two-node-harness.md)
- **How-to:** [Verifying an exported cluster run](cluster-workflows/verifying-an-exported-cluster-run.md)
- **Reference:** [Cluster run directory reference](cluster-workflows/cluster-run-directory-reference.md)
- **Troubleshooting:** [Diagnosing partial cluster failure](cluster-workflows/diagnosing-partial-cluster-failure.md)
- **Review checklist:** [Reviewing live versus simulated cluster evidence](cluster-workflows/reviewing-live-versus-simulated-cluster-evidence.md)

### Extension workflows

- **Walkthrough:** [Following the system extension fixture](extension-workflows/following-the-system-extension-fixture.md)
- **How-to:** [Inspecting a native callback journal](extension-workflows/inspecting-a-native-callback-journal.md)
- **Reference:** [Extension integration input reference](extension-workflows/extension-integration-input-reference.md)
- **Troubleshooting:** [Diagnosing callback and component denials](extension-workflows/diagnosing-callback-and-component-denials.md)
- **Review checklist:** [Reviewing a new extension integration](extension-workflows/reviewing-a-new-extension-integration.md)

### World workflows

- **Walkthrough:** [Following a world preview](world-workflows/following-a-world-preview.md)
- **How-to:** [Inspecting capture and restore plans](world-workflows/inspecting-capture-and-restore-plans.md)
- **Reference:** [World command and evidence reference](world-workflows/world-command-and-evidence-reference.md)
- **Troubleshooting:** [Diagnosing blocked promotion and merge](world-workflows/diagnosing-blocked-promotion-and-merge.md)
- **Review checklist:** [Reviewing a world mutation request](world-workflows/reviewing-a-world-mutation-request.md)

### Evidence workflows

- **Walkthrough:** [Following a receipt into the ledger](evidence-workflows/following-a-receipt-into-the-ledger.md)
- **How-to:** [Inspecting a provenance chain](evidence-workflows/inspecting-a-provenance-chain.md)
- **Reference:** [Receipt chain and trust state reference](evidence-workflows/receipt-chain-and-trust-state-reference.md)
- **Troubleshooting:** [Diagnosing stale and wrong subject evidence](evidence-workflows/diagnosing-stale-and-wrong-subject-evidence.md)
- **Review checklist:** [Reviewing an evidence bundle for handoff](evidence-workflows/reviewing-an-evidence-bundle-for-handoff.md)

### Retention workflows

- **Walkthrough:** [Following a retention diagnostic](retention-workflows/following-a-retention-diagnostic.md)
- **How-to:** [Inspecting pins before a gc plan](retention-workflows/inspecting-pins-before-a-gc-plan.md)
- **Reference:** [Retention root and deletion evidence reference](retention-workflows/retention-root-and-deletion-evidence-reference.md)
- **Troubleshooting:** [Diagnosing uncollectable and unproven objects](retention-workflows/diagnosing-uncollectable-and-unproven-objects.md)
- **Review checklist:** [Reviewing destructive operation preconditions](retention-workflows/reviewing-destructive-operation-preconditions.md)

### Delivery workflows

- **Walkthrough:** [Following a delivery diagnostic](delivery-workflows/following-a-delivery-diagnostic.md)
- **How-to:** [Inspecting dead letter and redrive evidence](delivery-workflows/inspecting-dead-letter-and-redrive-evidence.md)
- **Reference:** [Delivery operation and attempt reference](delivery-workflows/delivery-operation-and-attempt-reference.md)
- **Troubleshooting:** [Diagnosing uncertain delivery outcomes](delivery-workflows/diagnosing-uncertain-delivery-outcomes.md)
- **Review checklist:** [Reviewing retry and duplicate risk](delivery-workflows/reviewing-retry-and-duplicate-risk.md)

### Release workflows

- **Walkthrough:** [Following release dependency validation](release-workflows/following-release-dependency-validation.md)
- **How-to:** [Assembling candidate scoped review inputs](release-workflows/assembling-candidate-scoped-review-inputs.md)
- **Reference:** [Release rail and evidence reference](release-workflows/release-rail-and-evidence-reference.md)
- **Troubleshooting:** [Diagnosing source gate and release blockers](release-workflows/diagnosing-source-gate-and-release-blockers.md)
- **Review checklist:** [Reviewing performance and readiness claims](release-workflows/reviewing-performance-and-readiness-claims.md)

## Documentation scope

These articles describe existing behavior and review practices; they do not create requirements, repair source discrepancies, authorize effects, or certify deployment. Governing architecture, accepted Cairn requirements, and subsystem contracts retain their roles. Where prose and implementation differ, the article should identify both sources and limit its claim rather than silently resolving the difference.

This is a documentation-only expansion under the [proof workflow's documentation exemption](../proof-workflow.md#proof-checklist-for-cairn-changes). No runtime, schema, dependency, policy, or evidence artifact is changed by the handbook. Behavioral verification remains a separate obligation for implementation changes.
