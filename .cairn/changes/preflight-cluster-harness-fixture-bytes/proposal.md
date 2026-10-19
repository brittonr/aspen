# Preflight cluster harness fixture file size before staging

## Why

`execute_cluster_harness` currently prepares state/output directories
before `plan_run` reads an arbitrary fixture via `read_to_string`.
The composed cluster parser and cluster CLI already admit up to
`MAX_CLUSTER_MANIFEST_BYTES = 1,048,576` bytes and 512 nodes, but the
harness bypasses their file-read boundary. An oversized fixture can
delete existing output roots under `--force` before the parser denies.

## What Changes

- Use `File::open` and `Read::take(MAX_CLUSTER_MANIFEST_BYTES as u64 + 1)`
  for the harness fixture, then reuse `parse_cluster_manifest` without
  introducing another byte limit or duplicate parser.
- Complete fixture parsing/planning before `prepare_output_roots` so
  overbound inputs do not stage a run, erase output directories or spawn
  children. Exactly 1 MiB remains admitted, 1 MiB plus one is denied.
- Independently audit ticket-path traversal: no existing artifact cap
  covers arbitrary observed state-root directories and non-ticket files.
  Record its missing owner-selected total-entry/queue budget and do not
  invent a cap in this fixture owner.

## Adoption dependency and impact

The new worktree starts from clean canonical Aspen, but this owner is
**NOT source-isolated**: the base lacks the parser byte cap and CLI
bounded read. Only the byte-identical parser/CLI source/test additions
from `aspen-octet-quality-owner-20260929` are ported into the isolated
worktree as a prerequisite; the quality owner MUST be integrated
before adopting this owner. The composition already has those
prerequisites and MUST NOT recopy them or alter accepted specs.
This delta changes harness intake, one real CLI integration regression,
handoff and change-local Cairn only. No waiver, policy, baseline, pin,
new arbitrary traversal limit or full known-failing strict Octet
retry is introduced.
