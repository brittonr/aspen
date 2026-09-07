# Closed node-host domain design

## Decision ownership

Molten owns the domain meaning and compatibility contract.
Octet owns exact marker admission and diagnostics; the Dylint driver supplies its existing library cfg.
A marker documents a reviewed closed domain. It grants no filesystem, source-gate, deployment, or runtime authority.
No configuration or receipt format is introduced.

## Exact scope

| Declaration | Consumer match sites at product ab3efbe97 |
|---|---|
| `local_store::LocalStoreKind` | `local_store/mod.rs:31` |
| `node_state::NodeStateNamespaceKind` | `node/state/authority.rs:40` |
| `node_state::NodeStateFileObservation` | `node/state/filesystem.rs:157`, `node/state/namespace.rs:105` |

Preserve public re-exports and every existing variant, payload, derive, and name.
Do not add `non_exhaustive`, change representation, or move decisions into a catch-all, helper table, or lookup default.

## Fixed mapping contract

Local-store pairs:
Artifact/artifact, Chunk/chunk, Retention/retention, Dataspace/dataspace,
Exchange/exchange, Ledger/ledger, Delivery/delivery, Durable/durable.

Node namespace pairs, in current `ALL` order:

| Kind | Relative directory |
|---|---|
| Identity | identity |
| Secrets | identity |
| Ledger | ledger |
| ControlInbox | control/inbox |
| ControlOutbox | control/outbox |
| ControlIngress | control/iroh-ingress |
| ControlIdempotency | control/idempotency |
| ControlService | control/service |
| Services | services |
| Receipts | receipts |
| Registry | registry |
| Chunks | chunks |
| Storage | storage |
| Ingress | control/iroh-ingress |

The two directory aliases are intentional. Do not require directory-string uniqueness.
`ALL` must retain its 14 distinct kind values and current order.
Directory aliasing must not make enum identities equal or authorize cross-namespace entries.
Expected-table tests describe this current inventory; they are not automatic discovery of future variants after arbitrary source changes.

## Observation contract

`Missing` remains distinct from `NonRegular(NodeStateEntryKind)` and `Regular(NodeStateFile)`.
Bounded reads deny Missing and NonRegular; only Regular consumes the already acquired file handle.
Mode observation returns None for Missing, denies NonRegular, and returns the recorded mode for Regular.
Retain exact error messages, no-follow behavior, observation identity, read limits, and effect order.
No fallback may classify a future observation as a current case.

## Proposed marker route

At the node-host crate root, conditionally enable `feature(register_tool)` and `register_tool(octet)` only when the driver supplies `dylint_lib = "octet"`.
Place `cfg_attr(dylint_lib = "octet", octet::sealed_enum)` on only the three reviewed declarations.
Register the expected cfg in the package manifest; do not allow unexpected-cfg diagnostics generally.

The repaired driver's source contains `--cfg=dylint_lib="{name}"` injection for active libraries.
Implementation must still prove this route with actual driver and package controls; source inspection alone is not evidence of activation.
Normal builds retain existing compiler requirements. A standalone unbootstrapped compiler control must prove that inactive registration attributes do not impose a new feature requirement.
This does not claim the whole existing dependency graph supports stable Rust.

## Verification design

1. Test every mapping and the exact namespace registry; test that aliases remain distinct enum identities and do not relax view authorization.
2. Retain and rerun real capability-backed missing/regular/directory/symlink tests for read and mode behavior. Do not replace them with scalar models.
3. In fresh isolated source copies, add a variant to each of the three actual enum declarations without changing consumers. Require E0004 at the relevant mapping/observation sites. Negative acceptance must identify those sites, not merely any compiler failure.
4. Prove marker activation with the retained Dylint driver. Wrong namespace, documentation text, and disabled marker cfg must not count as declarations. Preserve ordinary compiler exhaustiveness with and without active markers.
5. Run node-host tests and all-target Clippy on the existing focused toolchain, then freeze the implementation source and rerun the canonical source gate with the repaired diagnostic cohort.

Record source and tool identities, unchanged flags, source-after diff, process exit codes, and scoped results.
The four enum findings are expected to disappear; all other differences require explanation.
Do not report a clean workspace, startup approval, or runtime proof from that expected change.

## Lifecycle and reversibility

Keep this change separate from startup admission. Parent lifecycle tasks 4–5 remain open.
Proposal/design approval precedes production marker or feature edits.
If compatibility or negative controls fail, stop and retain the failure; do not add suppressions or change the canonical tool command.
Removing proposed analysis annotations later must leave runtime data and behavior unchanged.
