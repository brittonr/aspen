# Deferred decision record

This page records no-go decisions for issues whose own acceptance criteria make
"keep the current design" or "decide later" a valid outcome. A no-go decision is
not a claim that the idea can never work; it names the missing prerequisite and
the trigger for reopening.

## #2 Tree-clock evaluation

**Decision: no tree-clock implementation or dependency.**

**Prerequisite status (rechecked 2026-11-16).** The causal-turn prerequisite has
landed: the `record-turn-causality-in-traces` change is archived with every task
complete (`.cairn/archive/1970-01-01-record-turn-causality-in-traces/`), and
`r[molten.runtime_spine.turn_causality]` is an accepted requirement. Every
`RuntimeEvent::TurnCommitted` now carries `actor`, `cause` and `event_refs`
([`chronicle.rs`](../src/runtime/turn/chronicle.rs)); the cause is one of the
closed `RuntimeTurnCause` variants (prior turn, cleanup, linked-task release,
periodic activation, delay, external) in
[`causality.rs`](../src/runtime/turn/causality.rs). The earlier statement that
committed events lack a cause field is superseded.

**Still missing for a tree-clock evaluation:**

- No named recorded-causality query, workload, expected scale, time or memory
  budget, or acceptance threshold exists: a 2026-11-16 search of `docs/`,
  `.cairn/changes/`, `src/`, `crates/` and `tests/` for tree-clock,
  vector-clock or causal-query consumers found only this record and the issue
  triage page.
- No baseline measurement shows that an explicit-edge DAG or a vector clock is
  insufficient for any query.
- The trace is not yet a causal index. `TraceEvidence` still binds an ordered
  list of event refs and a replayability status
  ([`syndicate/p000/body.rs`](../src/runtime/dataspace/parts/syndicate/p000/body.rs)).
  Turns started through `PendingTurn::new()` or `apply_step` record a generic
  `External` cause (`runtime-turn`, `runtime-step`), so they carry no prior-turn
  edge, and the committed record names an actor string with no incarnation
  identity. A projection to clock entries would have to define those cases
  first.

Reopen when one concrete diagnostic query names its workload, bounds, baseline
result and acceptance threshold. Keep the explicit DAG or vector-clock baseline
authoritative until then; this record runs no experiment and makes no
equivalence or performance claim.

## #3 bbtree transient-index evaluation

**Decision: keep the current BTreeMap index; no bbtree dependency.**

The full product graph still rejects bbtree's exact August `vstd` alongside
`verified-logic`'s January `vstd`. That is an integration blocker, not a claim
that isolated compilation or measurement is impossible. On 2026-11-16 the
unmodified upstream revision `8adae97a0ed97e5c98fa2790312eb0aec9719846`
(bbtree 0.1.5, MIT OR Apache-2.0) built in an isolated pinned cohort.

The retained [observation](../evidence/prolly-transient-index-2026-11-16/observation.json)
binds exact current, actual unsync bbtree planner/diff, and sorted-vector/edit-overlay
source copies, resolved lock, fixtures, commands, full measurements and BLAKE3
inputs. Both plain and allocation-instrumented binaries passed all 26 semantic
cases. Seven repetitions across eight representative/maximum fixtures and ten
phases produced 3,360 rows in 42 processes. Canonical nodes/roots, staged blocks,
edit/error ordering, diffs and validation outcomes matched; this is sampled
differential evidence, not proof over the byte domain.

The predeclared gate required at least 10% paired end-to-end improvement in
every fixture, at least six of seven passing pairs, relative median absolute
deviation at most 5%, no semantic regression, and at most 5% peak/retained-memory
or RSS regression in every measured phase. Neither candidate passed.

| Fixture | bbtree/current median paired pipeline ratio | Sorted/current ratio |
| --- | ---: | ---: |
| Representative sequential | 0.9614 | 1.0101 |
| Representative random | 0.9813 | 1.0049 |
| Representative mixed | 0.9839 | 0.9592 |
| Representative common-prefix | 1.0152 | 1.0241 |
| Maximum sequential | 0.9504 | 0.9404 |
| Maximum random | 0.9670 | 0.9533 |
| Maximum mixed | 1.0063 | 1.0146 |
| Maximum common-prefix | 0.9785 | 1.0001 |

Peak/retained requested-live memory, phase RSS and process high-water RSS are
retained, not inferred from timing. Both process high-water gates passed; some
representative phase-memory/RSS gates failed. Pipeline retained requested-live
bytes were zero after output destruction. Diagnostic phases include validation,
decoding, indexing/conversion, cloning, iteration and canonical building; they
are not additive. The sorted control deliberately retains the current diff
algorithm. No cache indexes were changed or benchmarked: hot-set/scan, eviction
churn, hash-lookup baselines and disabled-cache equivalence remain prerequisites
if profiling ever justifies a cache experiment.

The generic x86-64 build permits applicable SSE tag paths and scalar fallbacks;
it does not qualify AVX512/native, AArch64/NEON, other targets, Miri, Verus,
concurrency, cold-cache or deployment behavior. Upstream proofs do not certify
the adapter. Molten-core Prolly maintainers own the retained baseline and any
future review. Reopen only for a materially changed compatible cohort or a
named workload with new predeclared gates. Canonical storage, profile identity,
roots, operation-scoped cache, publication and authority remain unchanged.

## #6 Live-frame hashing split

**Decision: keep the current single-step frame path.**

The 2026-11-16 optimized native loopback
[observation](../evidence/frame-owner-profile-2026-11-16/observation.json)
retains the timing-only patch, exact pre-instrumentation files, collector,
profile contract, commands, canonical ACK samples and BLAKE3 input identities.
Three warmups and twenty measured operations at each size completed: 46 real
Delivered exchanges, zero automatic retries, zero final inflight bytes and clean
listener cleanup. Actual ACK transition refs and complete transport state
matched the direct canonical reference; the route exposes a receipt ref, not
materialized receipt bytes.

| Selected fixture payload | Mean admission hash | Mean owner-held time | Hash/owner aggregate |
| --- | ---: | ---: | ---: |
| 1,024 bytes, assumed typical | 1.772 µs | 2.140 ms | 0.0828% |
| 4,096 bytes, admitted fixture maximum | 5.979 µs | 2.148 ms | 0.2784% |

The predeclared threshold was strictly more than 10% at **both** sizes on an
independently reviewed production-representative runtime-reuse/amortization
baseline. Neither diagnostic percentage reaches it, and that production
baseline is absent: this API still creates a worker thread and Tokio runtime
per frame. No runtime cost is subtracted to manufacture such a baseline. The
owner denominator measures complete external register and route calls,
including destructors and timing overhead; nested stages are not additive and
concurrent server hashing is not owner CPU time.

This is a selected 4-KiB fixture profile, not a universal transport maximum;
1 KiB is an explicit typical-size assumption, not a production traffic sample.
It qualifies no Internet, cross-host, deployed service, physical hardware or
release. Fabric-transport maintainers own the retained measurement capability.
Reopen after runtime reuse/amortization exists and a reviewed bounded profile
meets its predeclared gate. Canonical hashing, pending/ACK and uncertain-delivery
semantics remain unchanged; no begin/commit prototype or dependency is added.

## #9 TPM2-sealed node identity key

**Decision: supersede the earlier no-go with an explicit transport-only
`Tpm2Sealed` backend; the retained disposable-swtpm consumer rail passed.**

`ManagedSecret` does not front systemd-creds, TPM, or a secret manager. The
[reviewed issue comment](https://github.com/brittonr/aspen/issues/9#issuecomment-6768827011)
and [backend decision](fabric-cryptographic-identity/tpm2-sealed.md) distinguish
that eligibility label from an implemented loader and from PKCS11 signing.

The new loader checks the exact policy-only public object and measured tool,
unseals a PCR-bound transport record into memory through bounded execution,
and returns typed failures without file fallback. The existing rotation plan
admits exact `CapabilityFile`/`Tpm2Sealed` transitions in both directions.
The pinned disposable swtpm rail covers actual seal/load, selected-PCR extension
and denial, consumer-owned plaintext absence, and actual destination-key rotation
completion. The original parent rail execution reached both rotations, then
failed its plaintext scan with emulator state inside the consumer namespace.
The repaired rail separates provider device-state authority without excluding
any consumer file. Its retained consumer control subsequently passed with
`tpm2-tools` 5.7 and `swtpm` 0.10.1, including seal/load, both rotation directions,
consumer-file plaintext absence and denial after extending `sha256:16`; see the
[execution log](../.cairn/archive/2026-11-15-add-tpm2-sealed-identity/evidence/swtpm-2026-11-13/consumer.log)
and [cohort](../.cairn/archive/2026-11-15-add-tpm2-sealed-identity/evidence/swtpm-2026-11-13/cohort.txt).
This is software-TPM evidence, not physical-TPM qualification, encrypted provider
state, host-filesystem secrecy, startup integration, attestation or production admission.

## #10 Encrypted discovery provider

**Decision: do not add an Aspen discovery provider; keep address discovery and relay selection at the host layer until a concrete admitted-member consumer requires more.**

The issue is a design consideration, not an accepted requirement. There is no
agreed threat model for address hints, no membership-policy decision about which
already-admitted peers may learn a hint, and no provider or relay profile that
would carry the encrypted payload. Implementing a provider first would decide
those questions implicitly.

**Need and ownership assessment (2026-11-16).**

- *Deployments with changing member addresses: none recorded.* The repository
  has no deployment inventory. Its documented deployment surface is the bounded
  local [NixOS service module](nixos-service-module.md); the
  [external pilot tracker](external-pilot-tracker.md) has not selected a
  two-host workflow, and cross-host transport qualification (#15) is still
  unmet. Whether any operator runs members whose addresses change is unknown: a
  2026-11-16 search of `docs/`, `evidence/`, `pilots/`, `release/`,
  `.cairn/changes/` and the issue tracker found no report of one.
- *What the current code does with addresses.* Membership snapshots, views and
  node descriptors have no address field.
  `PolicyManagedProvider::replace_snapshot` accepts a newer membership view
  (same provider kind and profile, advancing epoch), which answers the issue's
  open question for membership only, not for locators
  ([adapters](../src/fabric_membership/adapters.rs)). Peer locators come from a
  `CrossProcessEndpointDescriptor` with a locator cohort and a validity window
  ([endpoint](../crates/molten-core/src/fabric_transport/cross_process/endpoint.rs)),
  the direct-LAN binding pins an authenticated endpoint id rather than an
  address hint ([direct](../crates/molten-core/src/fabric_transport/direct.rs)),
  and the Iroh endpoints are built with `RelayMode::Disabled`. A changed address
  therefore needs a newly issued descriptor today.
- *Host-layer trials reviewed.* The dots-onixos DNS/relay candidate declarations
  and recorded semantic boundary concern host transport, not Aspen membership,
  policy or encrypted group-hint admission. Reviewing them does not qualify
  changing-address discovery in Aspen; no private inventory, endpoint, identity
  or execution artifact is republished here.

No current Aspen consumer with changing member addresses is established. Adding
a second discovery protocol would introduce disclosure and key-distribution
policy without that demand. Keep this concern with the existing host-layer
trials rather than adding a provider or enabling a relay in Aspen.

Reopen when a concrete admitted-member workflow demonstrates that the host-layer
path is insufficient, and after its threat model, membership disclosure rule,
hint lifetime, group-key distribution/rotation and bootstrap path are agreed.
Address hints must remain non-authoritative.

## #12 Root-blocker explanations

**Decision: no explanation engine.**

No bounded world-workflow consumer asks for root-blocker explanations. The
existing operator workflows already report the first blocker and link receipts;
a general explanation engine would add projection, rule and budget surface
without a named query.

Reopen with a concrete workflow question, a bounded input snapshot, and a
comparison against the current first-blocker diagnostics.

## #52 Repository hygiene

### Epoch archive dates

**Cause.** The 88 `.cairn/archive/1970-01-01-*` entries come from Cairn's
`archive_date()` (`crates/cairn-cli/src/support/mutation.rs` in the Cairn
repository): it reads `CAIRN_ARCHIVE_DATE`, then `IRONSPEC_ARCHIVE_DATE`, and
falls back to the literal `1970-01-01` when neither is set. The development
shell sets neither, and `SOURCE_DATE_EPOCH` does not affect this path. The
fallback is a Cairn defect; Aspen does not patch it here.

**Decision: keep the existing names.** Renaming archives would break their
links and the archive-preservation rule in `AGENTS.md`. Each entry's real
archive commit is `git log --follow --oneline -- .cairn/archive/1970-01-01-<name>`.

**Prevention.** Archive with an explicit date,
`CAIRN_ARCHIVE_DATE=$(date -u +%F) cairn archive ... --execute`. The
`cairn-archive-date` pre-commit hook rejects any newly added
`.cairn/archive/1970-01-01-*` path, and the `epoch-archive-dates` flake check
enforces the same rule in CI: the 88 historical names are recorded in
`tools/repository-hygiene/epoch-archives.txt`, and any other epoch-dated
archive, or a missing recorded one, fails the check.

### Retained evidence and the orchestration archive

**Decision: keep `evidence/`, the archived change evidence and
`aspen-orchestration-batch-implementation.zip` unchanged.** `AGENTS.md`
preserves `.cairn/archive/`, `cairn/archive/` and historical evidence. The files
are already public in history, so redacting or deleting the current copies would
not remove them, while it would change the bytes that receipts and manifests
bind by hash. `docs/external-pilot-tracker.md` and
`pilots/orchestration/source.ncl` reference the zip.

A scan of the tracked evidence, archives, pilots and the zip on 2026-11-15
found no private-key blocks or GitHub, AWS, Slack, OpenAI, Anthropic, Google or
GitLab tokens. `evidence/cairn-source-remediation-2026-10-15/` (42.5 MB, 1,193
files) held 25,932 absolute `/home/<user>` paths, 1,068 local host-name
occurrences, and one captured `sudo -l` policy listing; those are
host-configuration disclosures, not credentials.

**Recurrence guard.** New and modified evidence must use the placeholders in
[CONTRIBUTING.md](../CONTRIBUTING.md#publishing-evidence). The
`evidence-publication-guard` pre-commit hook and the `evidence-publication`
flake check run `scripts/evidence-publication-guard.py`, which rejects home
paths, `/dev/shm` run paths, `.local` and observed host names, and RFC 1918 or
CGNAT/tailnet IPv4 addresses under `evidence/`, `release/` and change or archive
evidence (home and scratch paths also under `docs/`); the public GitHub-hosted
runner home is not a finding. The 1,068 files that held such material at
`fcbf60a3d` (636 in `evidence/`, 425 in `.cairn/archive/`, 4 in
`.cairn/changes/`, 3 in `docs/`) are grandfathered in
`tools/repository-hygiene/evidence-publication-baseline.tsv` by path and
BLAKE3, so an edit to any of them must redact it.

Reopen if a scan finds a credential (rotate it first; deleting the file does not
remove it from history) or if a release needs a redacted evidence export.

### Fragment layout

**Decision: keep the `parts/**/body.rs` + `#[path]` + `include!` layout for now.**

Measured costs at the reviewed commit: 1,262 `parts/**/body.rs` fragments (src
1,220, crates 4, tests 38); 549 `#[path = "..."]` remaps; 207 `include!` lists
with two or more fragments, 65 of them not in numeric order. These cost
navigation and long `super::` chains; they do not cause build or correctness
failures.

Reopen when a feature requires restructuring a fragmented module, when
diagnostics or review navigation become a measured bottleneck, or when a
fragment exceeds the 300-line bound. A reopening must preserve `// r[...]`
traceability and rerun the focused test rail.
