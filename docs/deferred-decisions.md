# Deferred decision record

This page records no-go decisions for issues whose own acceptance criteria make
"keep the current design" or "decide later" a valid outcome. A no-go decision is
not a claim that the idea can never work; it names the missing prerequisite and
the trigger for reopening.

## #2 Tree-clock evaluation

**Decision: no tree-clock implementation or dependency.**

The prerequisite causal-turn validation has not produced a named recorded-
causality query, expected scale, time or memory budget, or evidence that an
explicit DAG or vector-clock baseline is insufficient. The inspected runtime
records ordered events without a cause field, so a tree-clock projection would
start from a representation the prerequisite has not validated.

Reopen when the prerequisite lands and one concrete diagnostic query names its
workload, bounds, baseline result, and acceptance threshold. Keep the DAG or
vector-clock baseline authoritative until then.

## #3 bbtree transient-index evaluation

**Decision: keep the current BTreeMap index; no bbtree dependency.**

The published `bbtree` versions inspected (0.1.5 through 0.2.1) pin
`vstd = "=0.0.0-2026-08-30-0159"`. The workspace's `verified-logic` dependency
pins `vstd = "=0.0.0-2026-01-25-0057"`. Cargo rejects the two exact pre-release
versions in one graph, so a benchmark cannot compare bbtree against the current
planner without a separate workspace, a different Verus cohort, or a patch that
would change the evaluated source.

Reopen with a bbtree release or pinned revision that is compatible with the
workspace's verified-logic cohort, or with an isolated benchmark workspace that
retains the exact source/toolchain evidence. Until then, the canonical edit
path, profile identity, roots, and storage behavior stay unchanged.

## #6 Live-frame hashing split

**Decision: keep the current single-step frame path.**

No profile output shows that `frame_ref` is a material fraction of owner-held
time. The current path also creates a thread and Tokio runtime per exchange,
which is a larger suspected cost. Splitting hashing into a `Send` begin step and
an owner-held commit step before measuring that path would add an API and stale-
pending state without evidence.

Reopen after runtime reuse or amortization lands and a profile at typical and
maximum frame sizes names a predeclared threshold. The canonical hash, pending,
acknowledgement, and uncertain-delivery semantics must remain unchanged.

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

**Decision: no provider implementation.**

The issue is a design consideration, not an accepted requirement. There is no
agreed threat model for address hints, no membership-policy decision about which
already-admitted peers may learn a hint, and no provider or relay profile that
would carry the encrypted payload. Implementing a provider first would decide
those questions implicitly.

Reopen after a threat model, membership disclosure rule, hint lifetime, and
public bootstrap path are agreed. Address hints must remain non-authoritative.

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
`.cairn/archive/1970-01-01-*` path.

### Retained evidence and the orchestration archive

**Decision: keep `evidence/` and `aspen-orchestration-batch-implementation.zip`
unchanged.** Both are already public in history, so redacting or deleting the
current copies would not remove them, while it would change the bytes that
receipts and manifests bind by hash. `docs/external-pilot-tracker.md` and
`pilots/orchestration/source.ncl` reference the zip.

A scan of the tracked evidence, archives, pilots and the zip on 2026-11-15
found no private-key blocks or GitHub, AWS, Slack, OpenAI, Anthropic, Google or
GitLab tokens. `evidence/cairn-source-remediation-2026-10-15/` (42.5 MB, 1,193
files) holds 25,932 absolute `/home/<user>` paths, 1,068 occurrences of the
host name `aspen1.local`, and one captured `sudo -l` policy listing; those are
host-configuration disclosures, not credentials. New evidence follows
`SECURITY.md` and records home directories and host names as placeholders.

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
