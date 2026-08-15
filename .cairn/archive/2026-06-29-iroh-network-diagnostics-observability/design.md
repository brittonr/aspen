## Overview

Use these n0 projects as references:

- `iroh-doctor`: local network report, accept/connect probes, port-map probes, relay latency, NAT classification, metrics dump.
- `net-tools`: `netwatch` for interface/route change monitoring and `portmapper` for UPnP/PCP/NAT-PMP mapping support.
- `iroh-metrics`: registry, counters/gauges/histograms, OpenMetrics text encoding, and metrics service patterns.
- `iroh-services`: optional cloud/service-backed endpoint metrics, relay presets, remote diagnostics capability, and client-host ALPN.
- `n0-watcher`: latest-value watcher semantics for state that changes frequently but should not grow unbounded queues.

Molten should not copy these projects as product semantics. The integration is a Molten evidence layer around diagnostics, metrics, and optional external telemetry.

## Functional core

Add pure deterministic core types:

- `NetworkDiagnosticsReport`: NAT class, UDP status, relay latency observations, direct-path status, port-map protocol availability, interface/route snapshot refs, diagnostics, and caveats.
- `ConnectivityProbeDecision`: pass/deny/degraded result for connect/accept, relay-only, direct, and timeout outcomes.
- `PortMappingDecision`: pass/deny/degraded result for probing or attempting UPnP, PCP, or NAT-PMP mappings.
- `NetworkWatcherSnapshot`: latest interface, address, default-route, relay, and endpoint-state summary.
- `MetricsSnapshot`: bounded metric groups, labels, counters/gauges/histograms, scrape/export refs, and redaction status.
- `ExternalDiagnosticsBridgeDecision`: pass/deny result for pushing metrics or granting remote diagnostics capability to an external iroh-services-style endpoint.

The core must normalize inputs, bound list sizes, classify observations, and decide whether evidence is pass, deny, or degraded. It must not perform network I/O, open ports, read environment variables, scrape metrics, or contact external services.

## Imperative shell

The shell owns live work:

- running STUN/NAT/UDP/relay probes,
- accepting or connecting to diagnostic peers,
- observing route/interface changes,
- starting or stopping metrics HTTP export,
- attempting port mappings,
- pushing to iroh-services when configured,
- writing receipts and diagnostic logs.

The shell must call pure validation before making network-mutating changes such as port mapping or external diagnostic capability grants.

## Port mapping policy

Port mapping is a network mutation and must be deny-by-default. A passing mapping attempt requires explicit requester, node identity, authority refs, policy refs, resource refs, port/protocol scope, duration bound, and operator evidence. A probe-only check may report availability without creating a mapping, but it still must bind diagnostics and caveats.

## Metrics and labels

Metrics are operational observations. Metrics labels must be bounded and redacted. Avoid labels that include raw peer ids, secret refs, full paths, ticket strings, or user-controlled high-cardinality values unless a visibility policy explicitly allows them. Metrics snapshots can support observability and resource-envelope receipts, but they do not replace canonical operation receipts.

## External diagnostics bridge

An iroh-services-style bridge may be useful for pilot operations, but it must be optional and explicit. The bridge profile should bind API-secret provenance without storing the secret in receipts, target service endpoint refs, allowed capability set, upload interval, redaction policy, and operator approval. Remote diagnostics requests must be admitted like any other inbound protocol.

## Receipts

Add canonical evidence families such as:

- `network-diagnostics-report-v1`,
- `network-connectivity-probe-receipt-v1`,
- `network-port-mapping-receipt-v1`,
- `network-watcher-snapshot-v1`,
- `metrics-snapshot-receipt-v1`,
- `external-diagnostics-bridge-receipt-v1`.

These receipts are diagnostics only. They must explicitly say they do not grant authority, policy, resource, provenance, source-gate, retention, transport correctness, or deterministic replay trust.

## Tests

Positive tests should cover local diagnostics report rendering, relay-latency summaries, probe pass/degraded/deny classification, metrics snapshot encoding, redacted labels, and watcher latest-state summaries.

Negative tests should cover missing policy for port mapping, malformed relay URLs, out-of-bounds metrics labels, secret leakage attempts, stale external capability grants, diagnostics with unrecorded live observations, and use of diagnostics receipts as mutation authority.

## Multi-node and soak integration

The NixOS multi-node VM check should bind network diagnostics and metrics snapshots into VM-level evidence when available. Production-soak receipts should include diagnostics refs for relay latency, route/interface state, resource pressure, and connectivity fault diagnostics. If host VM/network support is unavailable, the check must not mint pass evidence.

## Non-goals

- No default dependency on external iroh-services.
- No broad production WAN correctness claim from diagnostics alone.
- No automatic port mapping without explicit admission.
- No raw secret, path, ticket, or unredacted peer-data exposure in receipts.
