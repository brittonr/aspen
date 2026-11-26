# World promotion current facts

`src/world_promotion/current/mod.rs` adds a signed current-facts adapter for one
local promotion transaction. It verifies operator-supplied authority and
intent-closure files before the transaction validator sees them; the store reads
the live branch head and grant consumption inside its write transaction.
Caller-provided booleans are not authority.

The adapter is composed into `world-promotion promote` with the local
transaction store and a content-addressed receipt store. The request document's
`authority_admitted` and `intent_closure_complete` fields are planning inputs
only; the signed files replace them before planning and transaction validation.

## Authority grant

```json
{
  "schema": "molten.world-promotion.authority-grant.v2",
  "action": "promote",
  "branch_id": "release",
  "authority_ref": "blake3:...",
  "policy_ref": "blake3:...",
  "generation": 1,
  "admitted": true,
  "candidate_head": "blake3:...",
  "nonce": "<64 lowercase hex characters>",
  "expires_at": 4102444800,
  "signer": "<64 hex characters>",
  "signature": "<128 hex characters>"
}
```

Every grant names one closed `action` and exactly that action's target fields:

| `action` | Target fields | Authorizes |
| --- | --- | --- |
| `promote` | `candidate_head` | promoting that candidate from `generation` to `generation + 1` |
| `deny-reservation` | `reservation_ref` | denying that reservation |
| `plan-retry` | `reservation_ref`, `attempt_ref` | planning one retry of that uncertain attempt |
| `abandon-uncertain` | `reservation_ref`, `attempt_ref` | abandoning that uncertain attempt and its reservation |

A missing, extra, or unknown target field, an unknown action, a malformed
nonce, or generation zero is denied. The JSON is only a carrier: the adapter
parses it to a typed `WorldPromotionGrant` and verifies the Ed25519 signature
over that grant's canonical packed Preserves record
(`canonical_promotion_grant`):

```text
<world-promotion-authority-grant "molten.world-promotion.authority-grant.v2"
  <branch-id S> <authority-ref S> <policy-ref S> <generation N> <admitted <true>>
  <target <promote <candidate-head S>>>
  <nonce S> <expires-at N>>
```

The target record's label is the action (`<deny-reservation <reservation-ref S>>`,
`<plan-retry <reservation-ref S> <attempt-ref S>>`,
`<abandon-uncertain <reservation-ref S> <attempt-ref S>>`). Preserves strings
are length-delimited, so no field value can shift into another field and two
distinct grants never share signing bytes. The signer must equal the trusted
operator key passed to the adapter. A tampered field, wrong key, malformed hex,
or unsupported schema (including the delimiter-signed `v1`) is denied before
the transaction.

A verified grant is still not current. Currentness is decided against the live
store when the grant is applied: the grant must be admitted, the observed
wall-clock time (`world_witness::live_clock`) must be before `expires_at`, its
`nonce` must not have been consumed, and its action and target must equal the
requested change exactly. A successful promotion or operator mutation records
the nonce in the store's `world_promotion_consumed_grants_v1` table (mapped to the
canonical grant bytes) in the same write transaction as its other writes, so a
grant authorizes one change **per store**. The grant does not name a store: the
same signed grant presented to another state root whose branch head matches is
consumed there independently.

## Intent closure

```json
{
  "schema": "molten.world-promotion.intent-closure.v2",
  "operation_ref": "blake3:...",
  "branch_id": "release",
  "candidate_head": "blake3:...",
  "expected_generation": 1,
  "intent_refs": ["blake3:..."],
  "complete": true,
  "signer": "<64 hex characters>",
  "signature": "<128 hex characters>"
}
```

The closure is signed over its canonical packed Preserves record
(`canonical_intent_closure`): `<world-promotion-intent-closure SCHEMA
<operation-ref S> <branch-id S> <candidate-head S> <expected-generation N>
<complete B> <intent-refs [S ...]>>`. The intent refs are a sequence, not a
comma-joined string. The adapter marks closure complete only when the file is
complete, unique, and exactly matches the plan's intent refs and identity
fields.

## Intent material

An intent reference is the BLAKE3 content reference of the intent's canonical
material record (`molten.world-effect-intent-material.v1`, arity 5): semantic
ref, handler ref, adapter ref, and release class. The schema artifact is
`schemas/preserves-boundaries/molten-world-effect-intent-material-v1.preserves`.
`LocalIntentMaterial` stores the bytes in the node storage namespace at
`world-promotion/intent-material/<blake3>.preserves` and reads them with a
64 KiB bound. An operator publishes material with:

```sh
molten world-promotion publish-intent \
  --state-root target/node \
  --intent intent.json
```

where `intent.json` holds `semantic_ref`, `handler_ref`, `adapter_ref`, and
`release_class`. The command prints `intent_ref=<blake3 ref>` and
`authority_granted=false`: publication only makes material available.
Republishing identical material is idempotent; a stored file whose bytes differ
from its address is reported and never overwritten.

## Observed facts

`read_observed_current` verifies the grant and closure, checks that both name the
requested branch, and returns an adapter whose `WorldPromotionTransactionFacts`
carry the verified grant and the observed wall-clock time. It does not read the
branch head: `LocalWorldPromotionStore::commit_promotion` reads the live head and
the grant's consumption inside its Redb write transaction and runs
`validate_promotion_transaction` against them there, so no observation can go
stale between validation and publication.

When the transaction is applied, the adapter reads the material of every plan
intent. Absent material, non-canonical bytes, bytes whose content reference
differs from the intent ref, or material whose semantic, handler, adapter, or
release class differs from the request's intent all make
`intent_material_available` false, and the transaction validator denies
`IntentMaterialUnavailable` before the head or any reservation changes.

## Command

```sh
molten world-promotion promote \
  --state-root target/node \
  --request promotion.json \
  --authority authority.json \
  --closure closure.json \
  --trusted-signer <64 hex characters>
```

Resubmitting the same request replays the stored operation (`replayed=true`)
without moving the head again; a different request under the same operation ref
is denied as an identity conflict.

## Operator authority

Deny, retry planning, and abandon each require the original request document
(`--request`) and a signed grant (`--authority`, `--trusted-signer`) in the
format above whose action and target name exactly that mutation: a
`deny-reservation` grant for the reservation, or a `plan-retry` or
`abandon-uncertain` grant for the reservation and attempt. The pure
`validate_operator_authority` decision requires the grant to be admitted,
unexpired and unconsumed, its generation and policy to equal the **live** branch
head's (read from the head table, not the reservation's historical generation),
and its branch and authority ref to match the immutable original plan. The store
then applies the mutation in one write transaction that rechecks the live head
it was decided against and the nonce, and consumes the nonce together with the
reservation and attempt writes. A missing, stale, revoked, expired, consumed,
foreign, or wrong-action grant is denied before any record changes; a grant
issued before a later transition advanced the branch is stale.

All three operator commands rebuild the original plan from the request document.
The document's `authority_admitted` and `intent_closure_complete` fields are not
authority and are ignored here: the plan is rebuilt with the facts its committed
transaction validated and accepted only when the store holds that exact committed
promotion identity for the operation. A request that changes any identity field,
or names an operation that never committed, is denied, so no replacement logical
identity can be planned.

Retry planning requires the selected attempt to be uncertain and explicit
duplicate-risk acknowledgement:

```sh
molten world-promotion retry-plan \
  --state-root target/node \
  --request promotion.json \
  --authority authority.json \
  --trusted-signer <64 hex characters> \
  --reservation-ref <ref> \
  --attempt-ref <ref> \
  --next-attempt-ref <ref> \
  --acknowledge-duplicate-risk
```

The command emits a bound retry plan for the same logical release; it does not
dispatch. A successful plan consumes its grant.

An operator can deny a committed or blocked reservation:

```sh
molten world-promotion deny \
  --state-root target/node \
  --request promotion.json \
  --reservation-ref <ref> \
  --authority authority.json \
  --trusted-signer <64 hex characters>
```

Deny applies only before an effect attempt: committed or blocked reservations
can be denied, and denying an already-denied reservation under a fresh grant is
idempotent. In-flight and uncertain releases require reconciliation or explicit
unknown-outcome acknowledgement; deny cannot relabel them as if no effect had
occurred.

An uncertain attempt can be abandoned only with the original request document,
a current grant, and explicit unknown-outcome acknowledgement. The attempt and
its uncertain reservation both become `abandoned` in one write transaction, so
`reconcile` no longer counts the reservation as unresolved:

```sh
molten world-promotion abandon \
  --state-root target/node \
  --request promotion.json \
  --authority authority.json \
  --trusted-signer <64 hex characters> \
  --attempt-ref <ref> \
  --acknowledge-unknown-outcome
```

Deny and abandon publish canonical receipts; neither proves external effect
completion. Effect dispatch remains a separate admitted step.

The promote command publishes its canonical receipt under
`<state-root>/world-promotion/receipts/<blake3>.preserves`. Promotion commits the
active head and reservation set atomically; effect dispatch remains a separate
admitted step.

## Non-claims

The signature binds the supplied operator facts; it does not prove the
underlying authority system or policy correctness, external effect completion,
or production admission. Intent material availability establishes that the
bytes named by the signed closure exist locally and match the plan; it does not
prove the effect handler or adapter will accept them. The adapter does not
perform promotion by itself.

Grant nonces are single use per store, not globally: consumption is recorded in
the local Redb store and is not replicated, so it does not prevent the same grant
from authorizing a matching change in a different state root.
