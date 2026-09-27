# Molten technical companion

One hundred advanced reading notes on Molten's implementation boundaries. These articles complement—not replace—the [architecture](../architecture.md), [fabric ownership law](../distributed-system-fabric.md), [modularity boundaries](../modularity-boundaries.md), accepted Cairn requirements, and subsystem documentation linked in each article. They explain existing contracts; they do not introduce new requirements or certify production readiness.

Looking for a concrete workflow, command reference, or diagnostic procedure? Use the [workflow handbook](../handbook/README.md). The [documentation map](../README.md) explains how both collections relate to governing subsystem documents.

## How to read this collection

The intended reader is comfortable with Rust, state machines, content-addressed data, object capabilities, and distributed failure models. Start with **Foundations**, then follow the subsystem relevant to the change or incident you are investigating. Each article separates mechanics, invariants, failure reasoning, verification guidance, and non-claims, with links back to the owning sources.

Illustrative scenarios are reasoning aids, not execution records. Commands and test references are verification guidance unless explicitly identified as observed evidence. Neither this collection nor its link checks are canonical receipts. Where a source and an explanation disagree, consult the owning implementation and governing requirements; do not use explanatory prose to bypass admission.

### Suggested reading routes

- **Architecture review:** Foundations → Envelopes → Capabilities → Engineering.
- **Runtime implementation:** Dataspaces → Vats → Extensions → Execution → Time.
- **Distributed services:** Transport → Membership → Cryptography → Storage → Replication.
- **World lifecycle:** World state → World effects → Simulation.
- **Evidence and operations:** Configuration → Proof → Operations → Simulation.

## Topic index

### Foundations

Start here for ownership laws, identity, evidence, replay, and navigating the facade/core split.

- [Ownership and semantic boundaries](foundations/ownership-and-semantic-boundaries.md)
- [Deterministic playback contract](foundations/deterministic-playback-contract.md)
- [Canonical identity model](foundations/canonical-identity-model.md)
- [Evidence and authority separation](foundations/evidence-and-authority-separation.md)
- [Facade and core navigation](foundations/facade-and-core-navigation.md)

### Envelopes

Follow canonical values through envelope admission, schema boundaries, and reference domains.

- [Canonical preserves boundary](envelopes/canonical-preserves-boundary.md)
- [Envelope admission pipeline](envelopes/envelope-admission-pipeline.md)
- [Schema identity and evolution](envelopes/schema-identity-and-evolution.md)
- [Content references versus inline values](envelopes/content-references-versus-inline-values.md)
- [Typed reference domain separation](envelopes/typed-reference-domain-separation.md)

### Capabilities

Distinguish capability possession, delegated scope, policy admission, and current authority.

- [Capability context admission](capabilities/capability-context-admission.md)
- [Delegation and attenuation](capabilities/delegation-and-attenuation.md)
- [Policy preflight composition](capabilities/policy-preflight-composition.md)
- [Authority claims and subjects](capabilities/authority-claims-and-subjects.md)
- [Revocation and stale authority](capabilities/revocation-and-stale-authority.md)

### Dataspaces

Examine turn atomicity, assertion ownership, observation, and advisory caching.

- [Turn commit and rollback](dataspaces/turn-commit-and-rollback.md)
- [Assertion lifetimes and cleanup](dataspaces/assertion-lifetimes-and-cleanup.md)
- [Observation and routing](dataspaces/observation-and-routing.md)
- [Bounded access cache](dataspaces/bounded-access-cache.md)
- [Service dependency conversations](dataspaces/service-dependency-conversations.md)

### Vats

Study local object transactions and the asynchronous, authority-sensitive boundaries around them.

- [Transactional actormaps](vats/transactional-actormaps.md)
- [Near and far reference semantics](vats/near-and-far-reference-semantics.md)
- [Promise pipelining and bounds](vats/promise-pipelining-and-bounds.md)
- [Snapshot and restore authority](vats/snapshot-and-restore-authority.md)
- [Revocable proxies and rights](vats/revocable-proxies-and-rights.md)

### Extensions

Trace manifest admission, lifecycle fencing, callback effects, recovery, and migration.

- [Manifest and tier admission](extensions/manifest-and-tier-admission.md)
- [Generation fenced lifecycle](extensions/generation-fenced-lifecycle.md)
- [Callback validation and effect release](extensions/callback-validation-and-effect-release.md)
- [Native intent and recovery](extensions/native-intent-and-recovery.md)
- [Upgrade quarantine and migration](extensions/upgrade-quarantine-and-migration.md)

### Execution

Compare bounded processes, component hosts, and executable mappings without conflating their isolation guarantees.

- [Component import admission](execution/component-import-admission.md)
- [Bounded process execution](execution/bounded-process-execution.md)
- [Component resource accounting](execution/component-resource-accounting.md)
- [Executable extent trust boundary](execution/executable-extent-trust-boundary.md)
- [Execution performance evidence](execution/execution-performance-evidence.md)

### Time

Keep clock domains, scheduling, entropy, retry, and lease decisions explicit and bounded.

- [Clock domains and conversion](time/clock-domains-and-conversion.md)
- [Timer generation fencing](time/timer-generation-fencing.md)
- [Bounded runnable scheduling](time/bounded-runnable-scheduling.md)
- [Purpose bound entropy](time/purpose-bound-entropy.md)
- [Deadline retry and lease limits](time/deadline-retry-and-lease-limits.md)

### Transport

Separate protocol transitions and routing from admission, resource bounds, and live delivery observations.

- [Session admission and transitions](transport/session-admission-and-transitions.md)
- [Alpn routing and protocol identity](transport/alpn-routing-and-protocol-identity.md)
- [Backpressure and resource bounds](transport/backpressure-and-resource-bounds.md)
- [Sans io core and shell drain](transport/sans-io-core-and-shell-drain.md)
- [Transport failure and replay boundaries](transport/transport-failure-and-replay-boundaries.md)

### Membership

Separate connectivity, suspicion, placement, assignment, fencing, and consistency claims.

- [Connectivity versus membership](membership/connectivity-versus-membership.md)
- [Suspicion and source scoped views](membership/suspicion-and-source-scoped-views.md)
- [Placement versus committed assignment](membership/placement-versus-committed-assignment.md)
- [Epoch generation and token fencing](membership/epoch-generation-and-token-fencing.md)
- [Consistency and fastpath nonclaims](membership/consistency-and-fastpath-nonclaims.md)

### Cryptography

Analyze key purpose, signing domains, entropy, rotation, and the limits of verification.

- [Purpose scoped key handles](cryptography/purpose-scoped-key-handles.md)
- [Canonical signature domains](cryptography/canonical-signature-domains.md)
- [Entropy and key storage admission](cryptography/entropy-and-key-storage-admission.md)
- [Key rotation and generation fences](cryptography/key-rotation-and-generation-fences.md)
- [Verification without authority](cryptography/verification-without-authority.md)

### Storage

Follow capability-rooted effects, typed stores, canonical content, materialization, and disposable caches.

- [Capability rooted node state](storage/capability-rooted-node-state.md)
- [Typed local store boundaries](storage/typed-local-store-boundaries.md)
- [Content store profile equivalence](storage/content-store-profile-equivalence.md)
- [Filesystem materialization admission](storage/filesystem-materialization-admission.md)
- [Derived cache trust model](storage/derived-cache-trust-model.md)

### Replication

Study bounded transfer and delivery protocols, including recovery and addressable actor lifecycle.

- [Bounded dag closure exchange](replication/bounded-dag-closure-exchange.md)
- [Protected content replication](replication/protected-content-replication.md)
- [Delivery claims acks and retry](replication/delivery-claims-acks-and-retry.md)
- [Dead letter redrive and recovery](replication/dead-letter-redrive-and-recovery.md)
- [Addressable actor sleep wake drain](replication/addressable-actor-sleep-wake-drain.md)

### World state

Follow capture, detached heads, authority, semantic maps, and typed merge decisions.

- [Typed world roots and capture](world-state/typed-world-roots-and-capture.md)
- [Branch head claims and atomicity](world-state/branch-head-claims-and-atomicity.md)
- [Branch authority and linear transfer](world-state/branch-authority-and-linear-transfer.md)
- [History independent semantic maps](world-state/history-independent-semantic-maps.md)
- [Typed diff and merge conflicts](world-state/typed-diff-and-merge-conflicts.md)

### World effects

Distinguish publication, effect reservations, restore, distribution, replay, and operator composition.

- [Promotion and reservation atomicity](world-effects/promotion-and-reservation-atomicity.md)
- [Logical and opaque restore](world-effects/logical-and-opaque-restore.md)
- [Distribution retention and reachability](world-effects/distribution-retention-and-reachability.md)
- [Exact replay and bounded capsules](world-effects/exact-replay-and-bounded-capsules.md)
- [Preview first operator composition](world-effects/preview-first-operator-composition.md)

### Simulation

Interpret deterministic experiments, fault schedules, crash witnesses, VM observations, and structural metrics.

- [Deterministic simulation inputs](simulation/deterministic-simulation-inputs.md)
- [Fault schedules and observation](simulation/fault-schedules-and-observation.md)
- [Crash restart conformance](simulation/crash-restart-conformance.md)
- [Vm fault evidence boundary](simulation/vm-fault-evidence-boundary.md)
- [Structural benchmarks and nonclaims](simulation/structural-benchmarks-and-nonclaims.md)

### Proof

Read verification receipts, coverage, aggregate obligations, stack evidence, and release claims at their actual scope.

- [Verification run receipt contract](proof/verification-run-receipt-contract.md)
- [Positive negative traceability](proof/positive-negative-traceability.md)
- [Aggregate proof obligations](proof/aggregate-proof-obligations.md)
- [Stack evidence composition](proof/stack-evidence-composition.md)
- [Release readiness and proof scope](proof/release-readiness-and-proof-scope.md)

### Operations

Use lifecycle, health, cluster, profile, and replication evidence without promoting diagnostics to authority.

- [Node lifecycle and recovery](operations/node-lifecycle-and-recovery.md)
- [Bounded observability and health](operations/bounded-observability-and-health.md)
- [Receipt first cluster diagnosis](operations/receipt-first-cluster-diagnosis.md)
- [Production profile admission](operations/production-profile-admission.md)
- [Replication pilot operational limits](operations/replication-pilot-operational-limits.md)

### Configuration

Understand Nickel contracts, effective values, context expansion, effect profiles, and negative fixtures.

- [Nickel evaluation and contract boundary](configuration/nickel-evaluation-and-contract-boundary.md)
- [Effective config and source traces](configuration/effective-config-and-source-traces.md)
- [Context profile expansion](configuration/context-profile-expansion.md)
- [Effect manifest and handler profiles](configuration/effect-manifest-and-handler-profiles.md)
- [Negative contract fixture design](configuration/negative-contract-fixture-design.md)

### Engineering

Maintain dependency cohorts, purity boundaries, test authority, static audits, and profiling discipline.

- [Dependency cohorts and reproducible builds](engineering/dependency-cohorts-and-reproducible-builds.md)
- [Purity and adapter boundary enforcement](engineering/purity-and-adapter-boundary-enforcement.md)
- [Test workspace lifetime and authority](engineering/test-workspace-lifetime-and-authority.md)
- [Static authority audit method](engineering/static-authority-audit-method.md)
- [Profiling without semantic authority](engineering/profiling-without-semantic-authority.md)

## Reading source discrepancies

Some articles identify differences between governing prose and the inspected implementation. These are source-review observations, not reproduced bug reports, new policy exceptions, or fixes. Follow the linked sources before depending on the disputed guarantee. In particular:

- [Envelope admission](envelopes/envelope-admission-pipeline.md) distinguishes constructor validation from deserialization and later revalidation.
- [Proof composition](proof/aggregate-proof-obligations.md) separates aggregate extraction and layered diagnostics from acceptance of every child.
- [Promotion atomicity](world-effects/promotion-and-reservation-atomicity.md) distinguishes transactional publication from the timing of supplied authority observations.
- [Typed merge](world-state/typed-diff-and-merge-conflicts.md) identifies the equal-root shortcut's admission precedence.
- [DAG exchange](replication/bounded-dag-closure-exchange.md) scopes observation refresh relative to the transfer loop.
- [Transport replay](transport/transport-failure-and-replay-boundaries.md) distinguishes the generic registered shell from the cross-process wrapper.
- [Context expansion](configuration/context-profile-expansion.md) identifies category-specific override handling.
- [Local startup](operations/node-lifecycle-and-recovery.md) separates synthetic source-gate evidence from production requirements.
- [Static audits](engineering/static-authority-audit-method.md) distinguishes rule files, default profiles, and metadata identity.

## Documentation-only scope

This expansion changes explanatory Markdown and documentation navigation only. It does not change runtime behavior, schemas, policy, dependencies, accepted requirements, or evidence artifacts. Behavioral proof generation is therefore exempt for this documentation-only change under the [proof workflow](../proof-workflow.md#proof-checklist-for-cairn-changes). Source links and document structure are review aids, not proof of the distributed system's correctness. Existing subsystem verification obligations remain unchanged.
