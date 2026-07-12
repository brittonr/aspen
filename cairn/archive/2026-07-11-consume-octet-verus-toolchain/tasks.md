## Dependency migration

- [x] [serial] Add a revision-pinned Octet toolchain input distinct from the existing Rust source input r[molten.node_replication_pilot.octet_toolchain_consumption]
- [x] [serial] Consume the Octet profile and production verifier packages and remove pilot-owned Verus and Rust packaging r[molten.node_replication_pilot.octet_toolchain_consumption]

## Evidence and validation

- [x] [parallel] Add positive central-profile validation and a negative mismatched-profile check r[molten.node_replication_pilot.octet_toolchain_consumption]
- [x] [serial] Bind the Octet revision, profile digest, and package output into the deterministic decision r[molten.node_replication_pilot.octet_toolchain_consumption]
- [x] [parallel] Update command metadata, documentation, and saved blocked evidence without adding a runtime dependency r[molten.node_replication_pilot.octet_toolchain_consumption]
- [x] [serial] Run the pilot check and Cairn gates; sync and archive the change r[molten.node_replication_pilot.octet_toolchain_consumption]
