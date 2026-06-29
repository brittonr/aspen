## Why

Molten's live Iroh and multi-node VM work needs stronger operator evidence for the network environment around a node. The `iroh-doctor`, `net-tools`, `iroh-metrics`, `iroh-services`, and `n0-watcher` projects cover useful pieces: NAT and UDP diagnostics, relay latency, port-mapping probes, interface/route change monitoring, latest-state watchers, metrics export, and optional service-backed diagnostics.

Molten should integrate these ideas as evidence-producing diagnostics, not as ambient trust. Network observations can explain connectivity, resource pressure, and production-soak caveats, but they must not grant authority, policy admission, provenance trust, source-gate acceptance, retention clearance, or deterministic replay status.

## What Changes

- Add a local network diagnostics evidence surface for NAT classification, UDP reachability, relay latency, direct/relay path choice, interface/route snapshots, and port-mapping protocol availability.
- Add explicit, opt-in port-mapping attempts with authority, policy, resource, and operator evidence; deny by default when evidence is missing.
- Add route/interface watcher state for node health and live transport diagnostics, recording latest-state summaries without unbounded event buffers.
- Add bounded OpenMetrics-style export and snapshot receipts for node-control, live transport, queue depth, delivery idempotency, chunk/artifact sync, and resource pressure counters.
- Add an optional iroh-services bridge profile for pushing metrics or allowing remote diagnostics only when an operator supplies explicit external-service config and capability evidence.
- Bind diagnostics and metrics refs into production-soak and NixOS multi-node VM receipts as diagnostic evidence, with non-replayable live observations clearly marked.

## Impact

This gives operators and CI stronger visibility into why live Iroh paths pass, degrade, or deny. It should be especially useful for multi-node VM gates, production-soak pilots, and future real-network pilot runs. The change is evidence-only by design: diagnostics may guide decisions, but every side effect still requires the existing Molten gates.
