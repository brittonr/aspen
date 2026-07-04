# Change: cairn-policy-cross-reference-contracts

## Why

`cairn-policy/contracts.ncl` validates many policy fields individually, but several arrays still act as unchecked internal references. Unknown artifact dependencies, duplicate marker ids, stale replay case ids, or receipt contract entries that do not correspond to policy schemas can export into generated policy JSON before a later validation pass notices the mismatch.

## What

- Add whole-policy Nickel predicates that resolve internal Cairn policy references against declared ids.
- Reject duplicate marker ids/tokens, duplicate artifact ids, unknown artifact `requires` entries, stale determinism replay case/group refs, and receipt contract/schema command mismatches.
- Add focused fixtures for valid cross-references and representative malformed policies.

## Impact

Policy review catches structural mistakes at the contract boundary. Generated policy JSON becomes a tighter reflection of the reviewed source policy, reducing drift between policy source, validation gates, and release evidence.
