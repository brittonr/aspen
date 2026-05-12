# verus-proof-trust Specification

## Purpose
TBD - created by archiving change close-raft-batcher-apply-verus-proofs. Update Purpose after archive.
## Requirements
### Requirement: Raft Non-Crypto Verus Proof Gap Closure

Aspen MUST close remaining Raft `external_body` markers that represent operational arithmetic, byte accounting, or batch contiguity rather than cryptographic chain assumptions.

#### Scenario: Apply request arithmetic is verified
- GIVEN `apply_request_spec.rs` has trusted version/last-applied arithmetic facts
- WHEN the apply proof slice is implemented
- THEN the Raft Verus root MUST pass
- AND the markers for version increment and batch last-applied update MUST be removed or narrowed with a documented model limitation

#### Scenario: Write-batcher accounting is verified
- GIVEN `batcher_add_spec.rs` and `batcher_flush_spec.rs` have trusted byte-count and contiguity facts
- WHEN the batcher proof slice is implemented
- THEN the Raft Verus root MUST pass
- AND focused write-batcher tests MUST pass if executable helper semantics are touched

#### Scenario: Crypto chain assumptions remain out of scope
- GIVEN Raft chain hash/verify specs contain cryptographic assumptions
- WHEN this change is completed
- THEN those crypto assumptions MUST be left to the crypto/encoding boundary OpenSpec, not mixed with operational batcher/apply proof closure
