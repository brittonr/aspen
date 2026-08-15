## Tasks

- [x] [serial] r[molten.node_runtime.iroh_alpn_registry_model] Inventory current and planned Molten Iroh ALPN/protocol identifiers and assign owner namespaces.
- [x] [serial] r[molten.node_runtime.iroh_alpn_registry_model] Define canonical registry entry fields for symbolic name, ALPN bytes, owner, handler profile, lifecycle state, limit refs, and required admission evidence.
- [x] [parallel] r[molten.node_runtime.iroh_alpn_registry_validation] Add pure registry validation for uniqueness, deterministic encoding, supported lifecycle, and duplicate or malformed entries.
- [x] [parallel] r[molten.node_runtime.iroh_alpn_handler_ownership] Gate router install, replacement, and removal through registry owner, generation, and handler-profile checks before live router mutation.
- [x] [parallel] r[molten.node_runtime.iroh_alpn_non_authority] Ensure ALPN selection, endpoint identity, and router receipts remain routing evidence only and cannot satisfy operation authority.
- [x] [serial] r[molten.testing.iroh_alpn_registry_negative_fixtures] Add positive fixtures for valid install/replace/remove and negative fixtures for duplicate ALPN, malformed encoding, wrong owner, stale generation, unsupported ALPN, handler-profile mismatch, and ALPN-as-authority overclaims.
- [x] [serial] r[molten.testing.iroh_alpn_registry_negative_fixtures] Update router/operator docs and run focused router tests plus Cairn validation.