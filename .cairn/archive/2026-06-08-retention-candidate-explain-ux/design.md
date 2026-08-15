# retention-candidate-explain-ux Design

## Overview
`retention explain` is a read-only operator discovery workflow. Given an object ref and optional object-kind, retention-class, action, and subsystem filters, Molten scans the local retention store and emits canonical `retention-candidate-explain-v1` evidence summarizing what is already known about that candidate.

## Evidence Shape
The explain artifact includes:

- object ref and optional filters,
- pin refs,
- destructive evidence admission refs,
- remote clearance refs and remote clearance import refs,
- retention GC plan/apply/execute/audit refs,
- retention receipt refs and tombstone refs,
- diagnostics such as no known evidence or active pins,
- checks stating the artifact is read-only discovery and normal plan/apply/execute/admission/remote-clearance gates still apply.

The command writes the explain artifact only to the requested output/stdout and does not write new retention receipts, tombstones, plans, applies, executions, clearances, or admissions.

## Known Audit Storage
`retention gc-audit` already emits an audit artifact for operator review. This change stores the audit artifact under the retention root as known local evidence, so later explain calls can report audit refs alongside the execution gates they explain.

## Safety Boundaries
`retention-candidate-explain-v1` is discovery evidence only. It MUST NOT authorize deletion, substitute for plan/apply/execute gates, replace destructive retention admission, or act as policy, authority, resource, provenance, transport, execution, source-gate, remote-GC, or remote-clearance-import trust.
