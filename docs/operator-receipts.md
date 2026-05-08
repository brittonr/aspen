# Operator Receipts

## Auth/UCAN dependency receipt policy

When operator receipts mention auth admission, use the boundary in
[`auth-ucan-adapter.md`](auth-ucan-adapter.md): Aspen tokens remain the
user-facing wire format in this slice, while runtime verification projects
capabilities through the sibling UCAN validator. Redact token bodies, signatures,
private keys, bearer values, and local patch paths that reveal private checkout
locations; record only commit hashes, test commands, and pass/fail status.

Aspen has two operator-facing receipt surfaces for self-hosting evidence:

1. **Dogfood run receipts** from `nix run .#dogfood-local -- full`, which prove the local self-hosting loop reached Forge push, native CI build, deploy, verification, receipt publish, and cleanup.
2. **Native CI run receipts** from `aspen-cli ci receipt <run-id>`, which summarize one CI pipeline run with schema-versioned stage/job/artifact metadata.

Receipts are evidence summaries, not secret stores. Do not paste cluster tickets, bearer tokens, cookies, private keys, or connection strings into receipts or incident notes; redact them as `[REDACTED]`. Operator-visible receipt commands, including human and JSON output from dogfood receipt views and `aspen-cli ci receipt`, redact configured secret markers, `aspen://` ticket material, token/cookie/password/private-key fields, and unsafe connection strings while preserving non-secret run IDs, stage/job names, statuses, bounded failure categories, artifact hashes, and Nix store paths needed for diagnosis.

## Dogfood run receipts

A full local dogfood run exercises Aspen hosting itself:

```bash
nix run .#dogfood-local -- full
```

On success the command writes a local JSON receipt under the configured cluster directory's receipt sibling. With the default cluster directory this is:

```bash
/tmp/aspen-dogfood-receipts/<run-id>.json
```

The run also publishes the final validated receipt into Aspen KV before stopping the cluster:

```text
dogfood/receipts/<run-id>.json
```

For live cluster-backed readback, keep the cluster running:

```bash
nix run .#dogfood-local -- full --leave-running
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts cluster-show <run-id> --json
```

For local readback after the cluster has stopped:

```bash
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts list
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id>
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id> --json
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts diagnose <run-id>
```

### Dogfood receipt fields to check

Dogfood receipts use schema `aspen.dogfood.run-receipt.v1`. The top-level receipt records:

- `run_id` — stable run identifier, normally `dogfood-<timestamp>`.
- `command` — dogfood command that produced the receipt.
- `created_at` — wall-clock creation time.
- `mode` — whether federation or VM CI mode was enabled.
- `project_dir` and `cluster_dir` — local paths used by the run.
- `stages` — ordered stage receipts.

Each stage uses the field name `stage` and records:

- `status` — `succeeded`, `failed`, or another stage status.
- `started_at` / `finished_at` — wall-clock stage timestamps.
- `elapsed_ms` — monotonic stage duration evidence when available.
- `failure` — structured first-response summary for failed stages.
- `artifacts` — operator-safe local artifact references.

Use `jq` to inspect the status and duration evidence:

```bash
jq '{run_id, command, mode, stages: [.stages[] | {stage, status, elapsed_ms}]}' \
  /tmp/aspen-dogfood-receipts/<run-id>.json
```

For `full` mode, the normal success path includes these stages in order:

1. `start`
2. `push`
3. `build`
4. `deploy`
5. `verify`
6. `publish_receipt`
7. `stop`

Treat the run as operator-trusted only when every required stage succeeded and the final command output reported deploy and verification success. A receipt intentionally does not expose a top-level `status`; derive outcome from the stage list and command result.

### Failure triage

Start with the built-in diagnosis helper:

```bash
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts diagnose <run-id>
```

Then inspect the first failed stage:

```bash
jq '.stages[] | select(.status != "succeeded") | {stage, failure, artifacts}' \
  /tmp/aspen-dogfood-receipts/<run-id>.json
```

High-signal patterns:

- `push` failed: check Forge remote setup and whether the commit intended for CI was committed before running dogfood. Dogfood pushes git `HEAD`; uncommitted local fixes are not included in the CI source.
- `build` failed: inspect CI logs and native CI receipts for the pipeline run when available.
- `deploy` failed: inspect deploy executor output and node health.
- `verify` failed: inspect service health and the verification command output.
- `publish_receipt` failed: the local JSON receipt is still useful, but the run did not prove cluster-backed receipt readback.

## Native CI run receipts

Native CI receipts are returned over Aspen's iroh client RPC path by `CiGetRunReceipt` and exposed through `aspen-cli`:

```bash
aspen-cli ci receipt <run-id>
aspen-cli --json ci receipt <run-id>
```

The human output is a compact summary:

```text
CI receipt: <run-id>
Schema: aspen.ci.run-receipt.v1
Pipeline: <pipeline-name>
Repository: <repo-id>
Ref: <ref-name>
Commit: <commit-hash>
Status: <status>
Stages: <succeeded>/<total> succeeded
Jobs: <total> (<with-log-handles> with log handles)
Artifacts: <artifact-count>
```

The JSON output wraps the schema-versioned receipt:

```bash
aspen-cli --json ci receipt <run-id> | jq '.receipt | {schema, run_id, status, stages}'
```

### CI receipt fields to check

CI receipts use schema `aspen.ci.run-receipt.v1`. The receipt records:

- `run_id`, `pipeline_name`, `repo_id`, `ref_name`, and `commit_hash`.
- `status` — stable lowercase pipeline status.
- `created_at_ms`, `started_at_ms`, `completed_at_ms` — Unix millisecond timestamps.
- `error` — pipeline-level failure text when initialization or checkout failed.
- `stages` — pipeline stages in pipeline order.

Each stage records:

- `name` and stable lowercase `status`.
- `started_at_ms` and `completed_at_ms`.
- `jobs` — jobs in deterministic name order.

Each job records:

- `name`, optional `job_id`, stable lowercase `status`, timestamps, and `error`.
- `artifacts` — operator-safe `CiArtifactInfo` metadata produced by that job.

Artifact entries are metadata only. They include names, blob hashes, sizes, media/content metadata, and producer context needed to locate output; they do not embed artifact bytes or secrets. Receipt artifact lists are sorted deterministically by artifact name and then blob hash.

Useful `jq` views:

```bash
# Stage and job outcomes.
aspen-cli --json ci receipt <run-id> \
  | jq '.receipt.stages[] | {stage: .name, status, jobs: [.jobs[] | {name, job_id, status, artifact_count: (.artifacts | length)}]}'

# Artifact metadata by producing job.
aspen-cli --json ci receipt <run-id> \
  | jq '.receipt.stages[].jobs[] | select((.artifacts | length) > 0) | {job: .name, job_id, artifacts}'

# Log follow-up handles for failed jobs.
aspen-cli --json ci receipt <run-id> \
  | jq '.receipt.stages[].jobs[] | select(.status != "success") | {name, job_id, status, error}'
```

Use each `job_id` with the CI log/output commands when deeper diagnosis is needed.

## Acceptance evidence trail

The operator-receipt hardening slice is intentionally backed by both focused guardrails and full dogfood acceptance evidence:

- `789f099fd` (`Test CI receipt artifact evidence`) proves `CiGetRunReceipt` reads artifact metadata from KV and attaches it to the producing job.
- `10bb6fb38` (`Document operator receipt evidence`) adds this operator guide and the documentation/schema anchor guardrail.
- `b2ff3e75e` (`Fix operator receipt doc guardrail in Nix`) keeps the docs guardrail compatible with Nix cleaned Cargo sources.
- `9549b37dc` (`Guard dogfood receipt operator output`) pins dogfood receipt summary/diagnose output, including `elapsed_ms`, artifact metadata, failure display, and secret redaction.
- `ead4bd0a7` (`Guard CI receipt artifact output`) pins native `aspen-cli ci receipt` artifact display in human output.
- `e92606b12` (`Fix Nix clippy vendored dependency gate`) restores the Nix clippy gate after the vendored netlink/core macro seam blocked dogfood CI.
- `32806ccf9` (`Fix dogfood app vendored dependency builds`) makes dogfood app/package derivations compile dependency sources instead of reusing dummy local-path artifacts.
- `07cdb8610` (`Fix dogfood CI build artifact reuse`) extends that source-compilation path to dogfood CI build/test checks.

The latest full self-hosting acceptance run for the current pushed `main` is:

```text
run_id: dogfood-20260506T220958Z
local receipt: /tmp/aspen-dogfood-receipts/dogfood-20260506T220958Z.json
cluster key: dogfood/receipts/dogfood-20260506T220958Z.json
commit: a3f2cad78a6760f3782302bf68d15104db948123
result: format-check, clippy, build-cli, build-node, and nextest-quick passed; deploy completed; node 1 healthy; verification passed; publish_receipt and cleanup succeeded; 7/7 stages succeeded
receipt diagnosis: no failed stage found (7/7) stages succeeded
```

Earlier full dogfood acceptance for the operator-output guardrails:

```text
run_id: dogfood-20260505T202756Z
local receipt: /tmp/aspen-dogfood-receipts/dogfood-20260505T202756Z.json
cluster key: dogfood/receipts/dogfood-20260505T202756Z.json
commit: ead4bd0a7
result: deploy completed; node 1 healthy; verification passed; all stages succeeded
ci_run_artifact: 497775a3-9bb7-461b-8d17-d0147b956e18
```

A later current-head gated run captured durable failure evidence for the then-current synced `main`; it is superseded by the successful `dogfood-20260506T220958Z` run above:

```text
run_id: dogfood-20260506T191239Z
local receipt: /tmp/aspen-dogfood-receipts/dogfood-20260506T191239Z.json
commit: 2f55a92e17b3abecb71c5fa2f96eca087281fb1a
result: start succeeded; Forge push succeeded; native CI build gated at check/clippy; deploy/verify/publish_receipt did not run
triage: local `nix build .#checks.x86_64-linux.clippy --no-link -L --show-trace` reproduced unresolved `netlink-packet-route` imports against the Nix vendored `netlink-packet-core` surface
redaction: saved OpenSpec log redacts the cluster ticket and `aspen://...` remote URL; receipt/readback artifacts contain no secrets
```

Treat this section as a historical trail, not a live status endpoint. For current evidence, rerun `nix run .#dogfood-local -- full` at the commit you want to cite and record the new receipt path/key. A failure receipt is useful triage evidence, but it is not acceptance evidence unless every required stage succeeds.

## Operator checklist

Before citing a dogfood or CI receipt as evidence:

- Confirm the schema: `aspen.dogfood.run-receipt.v1` or `aspen.ci.run-receipt.v1`.
- Confirm the receipt refers to the expected repo/ref/commit or dogfood project directory.
- Confirm every required dogfood stage or CI stage/job has a success status.
- Confirm duration/timestamp fields are present where expected.
- Confirm artifact metadata is present for jobs expected to produce artifacts.
- Confirm copied receipt output contains redaction placeholders rather than raw tokens, tickets, cookies, private keys, connection strings, or synthetic secret markers.
- Prefer cluster-backed dogfood readback (`cluster-show`) when proving Aspen stored its own final success receipt.
- Redact secrets from copied logs and incident notes.
