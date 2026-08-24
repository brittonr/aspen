## Design contract

- [x] [serial] Add the bounded design contract for durable-store-coordinated ownership. r[aspen.cas.contract]
- [x] [serial] Implement the pure CAS lease ownership decision. r[aspen.cas.decision]
- [x] [serial] Document replaceable nodes and the absence of a fixed membership list. r[aspen.cas.boundary]
- [x] [serial] Record WalTier's CAS-arbiter log as a related bounded, non-parity reference. r[aspen.cas.boundary]

## Verification

- [x] [parallel] Add positive cases for a matching-owner acquisition with an advanced epoch. r[aspen.cas.verification]
- [x] [parallel] Add negative and boundary cases for a mismatched owner, a stale epoch, a lost lease, and a fixed-membership assumption. r[aspen.cas.verification]
- [x] [serial] Run package, workspace, Clippy, Cairn, and Nix checks, then document non-claims. r[aspen.cas.verification]
