## Phase 1: Implementation

- [x] [serial] Inventory hint producers and consumers and separate local bind configuration from exchanged hints. r[molten.fabric_transport.ip_hint_domain]
- [x] [serial] Add the shared admission; make producers omit and consumers reject hints outside the domain. r[molten.fabric_transport.ip_hint_domain]
- [x] [serial] Cover each address class, malformed scope text, the two-host interface map and the round-trip property; document the domain and compatibility; run focused tests, formatting, Clippy and source/lifecycle gates before review, sync and archive. r[molten.fabric_transport.ip_hint_domain]

## Verification (2026-11-16)

Integration tree at `ce12ae5d9` plus this change: `cargo fmt --check`; `cargo test -p molten --lib ip_hint` (4 passed: `exchanged_hints_admit_only_unscoped_unicast_addresses`, `malformed_and_scoped_hint_text_is_rejected`, `a_senders_interface_index_never_selects_a_receiver_interface`, and the `admitted_hints_roundtrip_every_semantic_field` Hegel property with 200 cases); `cargo test -p molten --lib fabric_transport` (24 passed); `cargo test -p molten --lib daemon` (71 passed, including the existing loopback live-ticket export and import tests); `cargo clippy -p molten --lib --bins --tests -- -D warnings`. No two-host network run was performed; the interface-map case is a fixture, and cross-host qualification stays with #15.
