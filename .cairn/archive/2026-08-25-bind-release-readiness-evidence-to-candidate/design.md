# Design

## Context

`release_candidate_gate_value` checks that each required evidence category has at least one valid content reference. It cannot determine which source revision produced each referenced artifact.

## Architecture

The functional core adds a narrow `CandidateEvidenceBinding` value. Each value pairs one artifact content reference with one candidate source content reference. The release gate validates both references, requires non-empty evidence for passing decisions, and requires every bound source to equal the gate's reviewed `source_ref`.

The core records candidate evidence bindings in the canonical receipt. The receipt moves to schema and record version 2 because its evidence fields change shape.

The CLI shell accepts one repeatable `ARTIFACT_REF@SOURCE_REF` binding option for each evidence category. It parses the pair, owns the strings, and converts them to borrowed core values. The core remains independent from Clap and external I/O.

## Invariants

- A passing gate has at least one binding for every required evidence category.
- Every artifact and source value is a canonical content reference.
- Every evidence source equals the gate's reviewed candidate source.
- The receipt preserves each artifact-to-source association.
- The source comparison occurs before the core emits a passing receipt.

## Boundaries and Non-Claims

- The core performs no file, process, environment, clock, or network access.
- The CLI owns argument parsing, output, and process exit behavior.
- A binding states which candidate an artifact claims to evaluate.
- The gate does not open the referenced artifact or prove its internal claims.
- A passing gate does not grant deployment, workload, policy, or release authority.

## Validation

Run focused release-candidate and CLI tests before and after the core change. Run formatting, Clippy, strict Cairn gates, the focused Nix rail, and the broad Nix rail before integration.
