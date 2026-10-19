# Preflight direct provenance binding references

## Problem

Normal provenance evaluation admits at most `MAX_PROVENANCE_REFS = 64` build-verification receipt inputs, and parsed provenance records admit at most 64 build-record refs. The public `evaluate_build_verification_binding` helper can instead be called directly with caller-constructed receipts or a forged public `Record`. Before this change it accepted an early passing receipt even among 65 receipts, or appended one or more diagnostics per failing receipt without that existing envelope. An oversized `build_record_refs` list also bypassed the parsed-record envelope during matching.

## Change

Use provenance's existing `ensure_ref_bound` on both receipt count and the record's build-record refs at helper entry. On an invalid count, return its current fail-closed `BuildVerificationBinding` form with one bounded-count diagnostic, without iterating or admitting any candidate. Exact-64 inputs retain normal matching semantics; no new reference cap is introduced.

## Verification

A canonical reproducible provenance record and real parsed build-verification receipt bind at the exact maximum. Both public +1 paths deny, including an early passing receipt and all-failing receipts whose diagnostic collection formerly scaled with the supplied slice.
