# World operator workflows

Molten provides one `molten world` command family for composed world operations.
It does not provide a second runtime, daemon, or workflow engine.

## Ownership boundary

The world operator owns these items:

- typed workflow requests;
- deterministic operation ordering;
- stable preview identities;
- first-blocker selection;
- fresh apply admission;
- component receipt links;
- bounded summaries.

Each world component keeps its existing domain logic and receipt meaning.
A handler delegates one operation kind to that component's application service.
The workflow rejects a handler or receipt that crosses its declared component owner.

The owner map is closed:

- World Commit owns inspect and checkpoint.
- World Head owns branch creation.
- Fabric Simulation owns run and simulate.
- World Merge owns diff and conflicts.
- World Replay owns replay, verify, export, and import.
- World Promotion owns promote.
- World Distribution owns garbage-collection planning.

The workflow never converts a component receipt into authority or correctness proof.

The pure core performs no file, process, network, clock, credential, storage, or component operation.
The shell owns request loading, component adapters, mutable observations, record publication, and terminal output.

## Closed command surface

The command family includes these typed commands:

```text
molten world plan
molten world inspect
molten world checkpoint
molten world branch
molten world run
molten world diff
molten world conflicts
molten world replay
molten world simulate
molten world verify
molten world promote
molten world export
molten world import
molten world gc-plan
```

Each command reads an explicit JSON request with `--request`.
JSON is the native CLI input format.
Unknown fields fail parsing.
Therefore, raw command text cannot enter the workflow core.

`molten world plan` accepts a multi-operation graph.
Each other command requires exactly one matching operation.
Every command writes a canonical plan with `--plan-out` or `--out`.

## Explicit request facts

A request names these facts:

- request, world, branch, and expected-head identities;
- expected branch generation;
- policy and authority-observation identities;
- exact resource limits;
- profile identities, kinds, states, and status evidence;
- expected mutable observations;
- operation identities, subjects, profiles, and dependencies.

The planner rejects malformed references, duplicate operations, unknown dependencies, cycles, and excessive bounds.
Input order does not change the normalized operation order or plan identity.

## Preview and apply

Mutation commands are preview-first.
Without `--apply-plan-ref`, a mutation command only writes its canonical plan and summary.

Apply requires the exact preview identity.
The shell then obtains fresh facts immediately before each component-owned mutation.
The core compares the head, generation, policy, authority observation, and profile.
Any difference stops the workflow before that mutation.

The standalone CLI has no ambient live component handler registry.
Therefore, an apply request writes a denial receipt and fails closed.
An embedding must compose reviewed handlers and current-facts adapters explicitly.

Unknown component outcomes stop later operations.
The workflow links reconciliation evidence and never retries the component automatically.

## Profile states

Each profile has one closed state:

- `admitted`;
- `blocked`;
- `unsupported`;
- `unavailable`.

Witnessed-head and executable-extent profiles stay explicit.
They never fall back to a local-head or ordinary-artifact profile.

Opaque replay requires its exact opaque profile.
Opaque workflows cannot request semantic diff or conflict comparison.
No logical fallback is available.

## Aggregate records

The workflow emits four canonical Preserves records:

- `molten-world-workflow-request-v1`;
- `molten-world-workflow-plan-v1`;
- `molten-world-workflow-receipt-v1`;
- `molten-world-workflow-summary-v1`.

Receipt links keep the operation identity, component reference, evidence role, and completion state.
Links cannot claim authority or deletion authority.
Links also cannot declare sensitive material.

The aggregate stops at the first blocked or unknown operation.
It cannot report later operations as complete.

Human summaries contain only stable references, counts, closed states, and blocker codes.
They do not contain private keys, bearer tokens, environment values, state payloads, or host paths.

## Logical dogfood rail

The deterministic logical rail composes this sequence:

1. Inspect the bounded world.
2. Preview and capture a checkpoint.
3. Create an attenuated branch.
4. Run the simulation profile.
5. Capture deterministic successor work.
6. Produce a logical diff.
7. Inspect conflict state.
8. Replay exact transitions.
9. Verify closure and evidence.
10. Admit promotion reservation planning.
11. Export the complete capsule.
12. Import through the publication boundary.
13. Plan retention and garbage collection.

The rail uses one operation handler per component kind.
It verifies ordering, fresh mutation admission, receipt linkage, and first-blocker behavior.

## Opaque dogfood rail

The opaque rail restores and replays one exact opaque profile.
It does not request logical diff, merge, or semantic equivalence.
A semantic comparison request fails before a component handler runs.

## Negative coverage

Focused tests cover these failures:

- stale plan identity;
- changed branch generation;
- missing component handler;
- blocked or unavailable profile;
- denied authority observation;
- unresolved conflict;
- incomplete replay capsule;
- unknown component outcome;
- dependency cycle;
- raw command field;
- missing expected head;
- authority overclaim;
- deletion-authority overclaim;
- sensitive-material flag;
- opaque semantic comparison.

## Claim boundary

Workflow evidence proves only the checked request, ordering, links, and bounded observations.
It does not prove component correctness, external effect completion, runtime safety, or release eligibility.
A garbage-collection plan does not grant deletion authority.
A passing dogfood rail does not prove an arbitrary host or whole stack.
