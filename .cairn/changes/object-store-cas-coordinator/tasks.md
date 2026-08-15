## Design contract

- [ ] [serial] Add the bounded design contract for durable-store-coordinated ownership. r[aspen.cas.contract]
- [ ] [serial] Implement the pure CAS lease ownership decision. r[aspen.cas.decision]
- [ ] [serial] Document replaceable nodes and the absence of a fixed membership list. r[aspen.cas.boundary]

## Verification

- [ ] [parallel] Add positive cases for a matching-owner acquisition with an advanced epoch. r[aspen.cas.verification]
- [ ] [parallel] Add negative and boundary cases for a mismatched owner, a stale epoch, a lost lease, and a fixed-membership assumption. r[aspen.cas.verification]
- [ ] [serial] Run package, workspace, Clippy, Cairn, and Nix checks, then document non-claims. r[aspen.cas.verification]
