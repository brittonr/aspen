## Phase 1: Receipt and replay

- [x] [serial] I1 Implement deterministic operational receipt construction and validation. r[molten.artifact_auth_operational_receipt.identity]
- [x] [depends:molten.artifact_auth_operational_receipt.identity] I2 Add capability-rooted receipt persistence and restart replay against actual file-adapter status. r[molten.artifact_auth_operational_receipt.persistence]
- [x] [depends:molten.artifact_auth_operational_receipt.persistence] I3 Add positive and negative restart, rotation, revocation, malformed, tamper, wrong-namespace, and false-parity tests. r[molten.artifact_auth_operational_receipt.replay]
- [x] [depends:molten.artifact_auth_operational_receipt.replay] I4 Update cryptographic-identity operator documentation and non-claims. r[molten.artifact_auth_operational_receipt.authority]

## Phase 2: Validation

- [x] [parallel] V1 Run focused Cargo, rustfmt, strict Clippy, Octet, and Cairn gates. r[molten.artifact_auth_operational_receipt.replay]
- [x] [serial] V2 Run full workspace and Nix gates; sync and archive requirements. r[molten.artifact_auth_operational_receipt.authority]
