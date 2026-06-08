## Overview

Introduce a shared destructive-retention evidence input shape used by subsystem destructive paths. The input carries requester identity, policy refs, authority refs, evidence refs, retained refs, remote refs, and reference-index completeness. Subsystems pass those values through to `retention::evaluate_retention` for every candidate before removing content or writing tombstones.

## Evidence requirements

Apply-mode destructive actions require all of the following before a candidate can pass:

- a requester ref identifying the operator or admitted maintenance actor,
- at least one policy ref,
- at least one authority ref,
- at least one supporting evidence ref,
- a complete reference-index proof,
- no active retention pins,
- no retained refs,
- no unresolved remote/cache refs.

Missing requester evidence is represented in the subsystem receipt diagnostics and evaluated with a deterministic placeholder ref so denial receipts remain canonical. Authority refs are evidence that delete authority was supplied; they do not grant authority by themselves.

## Dry-run behavior

Dry-run remains diagnostic/planning evidence. It still evaluates the supplied retention inputs and binds retention receipt refs, but it does not remove content or write subsystem tombstone receipts. Dry-run may be used to discover missing evidence without producing a pass gate for deletion.

## Receipt behavior

Subsystem receipts bind the retention receipt refs plus a retention-evidence summary including policy, authority, evidence, retained, remote, requester-present, and reference-index-complete fields. Retention receipts remain deletion-safety evidence and must not be treated as policy, authority, provenance, resource, transport, execution, or source-gate trust.

## CLI behavior

Ledger GC, chunk GC, and cache invalidation gain common flags for retention requester/policy/authority/evidence/retained/remote refs and an explicit `--retention-reference-index-complete` pass flag. Existing destructive invocations without these flags deny when candidates are selected for apply-mode deletion.
