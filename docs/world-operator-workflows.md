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

## Independent world-head provider process

`world-witness-provider-pilot` is a narrow public client for the separately governed
`release-channel-index` process/SSH service. It is not a head-authority command.
The provider owner supplies `provider init ROOT CONFIG` and authenticated namespace
enrollment; absent or damaged provider state must never be silently recreated.
Use a provider root outside the entire consumer-store snapshot/rollback domain.

The explicit client configuration contains `namespace` (`channel`, `platform`,
`track`), `provider_key` (32 verification-key bytes), `writer_secret` (32 secret
bytes), `contract` (`provider_id`, `contract_id`, `source_revision`, `license`), and
`transport` (`program`, literal `args`, `timeout_ms`, at most 60000).
The independently installed contract uses the exact `provider_identity(key)`,
owner-approved `release-channel-index-v1`, the actual reviewed immutable source
revision, and `MIT`. This input does not attest which executable a host deployed.
The namespace track must equal the exact world branch ID. Use private mode-0700
directories and mode-0600 secret configurations (`umask 077` before provisioning).
For SSH supply the executable, exact host/command and pinned host-key policy;
there is no discovery, TOFU, shell interpolation or implicit enrollment.

After the integration owner builds the pinned cohort, these commands exercise
the real public process adapter (the configuration names the actual provider
process or SSH capability):

```sh
world-witness-provider-pilot /cap/world-provider.json query
# Reserve input: {"expected_revision":1,"claim":{"generation":1,
# "digest":[32 exact canonical claim identity bytes],"predecessor":null}}
world-witness-provider-pilot /cap/world-provider.json reserve /cap/reserve.json
# Persist the returned evidence reservation, not the outer result wrapper.
# After independent consumer-owned final CAS confirms success:
world-witness-provider-pilot /cap/world-provider.json commit /cap/reservation.json
# Only after proving that CAS did NOT commit:
world-witness-provider-pilot /cap/world-provider.json orphan /cap/reservation.json
# Only after explicit reconciliation/operator authorization:
world-witness-provider-pilot /cap/world-provider.json resolve-orphan /cap/reservation.json
```

The reserve-input comment describes the byte-array shape, not literal executable
JSON. Use the 32 bytes decoded from the authenticated canonical `blake3:` claim
reference, never a digest of debug output or a fabricated matching reference.
Every actual request uses a new OS-random challenge and authenticates the exact
request digest, namespace, provider pin and minimum retained revision. Mutation
transport failure emits `Uncertain` with the original reservation, not no-effect
failure. Retain that reservation and reconcile via a fresh query; do not blindly
repeat the append. The pilot's output is detached provider evidence, not durable
Molten publication and not proof that a consumer transaction occurred.

Reservations and orphans block committed currentness. Resolving an orphan clears
only pending status; it retains highest, so the old head remains inadmissible.
Recovery must reserve and commit a strictly higher external ordinal binding the
exact externally committed predecessor. Molten's `provider::ordinal::plan` derives
that ordinal from a fresh signed query (the resolved-orphan high-water plus one);
the Molten head generation still advances by exactly one inside the claim digest.
No reset or checkpoint erasure is safe recovery.

Complementary Sigsum inclusion is verified through `provider::inclusion`; its
evidence never supplies external latest-head currentness or branch authority.
The retained index is an authenticated single-provider high-water contract, not
a Merkle log, global consensus, or proof that the provider cannot roll back.
Same-OS processes demonstrate the protocol only under an explicitly selected
storage-failure boundary: they do not establish independent administrators,
physical hosts, backups, or production suitability. Source fixtures and commands
are not execution evidence; the integration owner runs the actual consumer-store
rollback pilot and conformance suite after all source/dependency edits settle.


### Operator-owned mutation authority

`world_witness::authority::CurrentAuthority` implements the real fresh-admission
boundary. The operator installs `world-head-trust.json` independently of world
bundles and claims. It names the exact Artifact Auth public key/generation and
head policy, issuer and holder verification keys, Basalt policy, trust interval,
revocation generation and revoked proof references. A claim-keyed credential
contains the actual canonical head-statement signature, signed UCAN token/proofs,
and holder signature binding `molten://world-head/BRANCH`, ability `mutate`, the
world-head mutation profile and exact canonical claim bytes.
It also pins `witness_policy_ref`, the canonical identity of the independently
installed witness profile, provider/source contract and quorum policy. Staging
and final-CAS rechecks must match this current operator pin; presented witness
evidence or caller-supplied policy does not install or update that trust.

`wire::install_trust` and `wire::install_credential` are explicit provisioning
helpers, not trust discovery. The operator separately opens the selected
`ledger::ReplayRedbStore` and `view::RevocationRedbStore` with `SyncMode::Immediate`,
then calls `provision_revocations` before moving both adapters into
`CurrentAuthority::open`. Admission never auto-creates a missing revocation view.
Every recheck reloads trust, verifies actual Artifact Auth and Basalt/UCAN
boundaries, checks time/current revocation snapshot, and reserves replay state
with the selected durable authority core/Redb CAS. Only the same complete
invocation scope and exact reserved claim owner can recheck idempotently;
nonce reuse for another request, terminal/uncertain state, unavailable storage
or changed trust is not a successful recheck. Local head CAS still prevents a
second publication. Neither witness evidence nor a source closure grants mutation
authority. Keep the authority roots outside consumer rollback snapshots and
retain their storage/key integrity assumptions explicitly. Same-OS fixture
operators are experimental private trust roots, not production administration
separation or global replay exclusion.

