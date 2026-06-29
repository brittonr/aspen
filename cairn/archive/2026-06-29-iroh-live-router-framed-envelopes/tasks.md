## Phase 1: Core router model

- [x] [serial] r[molten.node_runtime.iroh_protocol_router] Add pure router registry types and validation for ALPN install, replace, remove, unsupported-ALPN denial, generation advancement, and shutdown evidence binding.
- [x] [serial] r[molten.node_runtime.iroh_protocol_router_receipts] Emit canonical router receipts for install, replace, remove, shutdown, and denial decisions.

## Phase 2: Framed envelope core

- [x] [serial] r[molten.node_runtime.iroh_framed_envelope_stream] Add pure framed-envelope validation for canonical Preserves bytes, declared envelope refs, sequence, peer, ALPN, and bounded frame limits.
- [x] [serial] r[molten.node_runtime.iroh_framed_envelope_stream] Add positive and negative unit tests for valid frames, malformed frames, oversized frames, mismatched refs, and unsupported ALPNs.

## Phase 3: Live Iroh shell

- [x] [serial] r[molten.node_runtime.iroh_protocol_router] Add the thin Iroh endpoint/router shell that applies admitted registry decisions and dispatches accepted connections to handlers.
- [x] [serial] r[molten.node_runtime.iroh_framed_envelope_stream] Add the thin bidirectional stream shell that length-delimits canonical envelope frames and calls the pure frame validator before delivery.

## Phase 4: Service-session patterns

- [x] [serial] r[molten.node_runtime.iroh_service_session_streaming] Add pure service-session descriptors for unary request/response, server streaming, client streaming, and bidirectional streaming over admitted framed envelope streams.
- [x] [serial] r[molten.node_runtime.iroh_service_session_streaming] Add tests that local and remote service-session records share the same canonical admission model while remote frames remain bounded Preserves frames.

## Phase 5: CLI and node-control evidence

- [x] [serial] r[molten.node_runtime.iroh_protocol_router_receipts] Add CLI/node commands or fixtures that install, replace, remove, and show live Iroh protocol registrations with receipt output.
- [x] [serial] r[molten.node_runtime.iroh_framed_envelope_stream] Bind framed-stream receipts into a node-control live workflow fixture without changing existing command semantics.

## Phase 6: Multi-node validation

- [x] [serial] r[molten.testing.nixos_vm_multinode.framed_stream] Extend the NixOS multi-node VM check to exercise one framed direct stream between nodes and bind its child receipts into the VM test run evidence.
- [x] [serial] r[molten.testing.nixos_vm_multinode.framed_stream] Add VM or harness denial coverage for unsupported ALPN and malformed/oversized frame attempts.

## Phase 7: Documentation and gates

- [x] [serial] r[molten.node_runtime.iroh_protocol_router_receipts] Document the evidence-only trust boundary and the `iroh-examples` reference patterns in README or architecture docs.
- [x] [serial] r[molten.node_runtime.iroh_protocol_router_receipts] Run focused Rust tests, Cairn validation, and the smallest relevant Nix/VM gate available in the current environment.
