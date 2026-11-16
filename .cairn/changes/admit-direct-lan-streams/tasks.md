# Tasks

- [x] [serial] Implement bounded deterministic receive gate and private GatedInbound with compile-fail coverage. r[molten.fabric_transport.receive_gate]
- [x] [serial] Add explicit direct profile and exact actual-peer/protocol/role/generation/operation admission. r[molten.fabric_transport.direct_lan]
- [x] [serial] Implement fixed-memory protected-socket streaming with lifecycle/uncertainty and no replay. r[molten.fabric_transport.direct_stream_bounds]
- [x] [serial] Author real same-host separate-process denial/fault/cleanup qualification tests. r[molten.fabric_transport.direct_lan] r[molten.fabric_transport.receive_gate] r[molten.fabric_transport.direct_stream_bounds]
- [x] [serial] Document selected consumer/runtime handoff and external qualification prerequisites. r[molten.fabric_transport.direct_lan] r[molten.fabric_transport.direct_stream_bounds]
- [x] [serial] Parent runs focused tests, source gates and strict lifecycle validation after integration. r[molten.fabric_transport.direct_lan] r[molten.fabric_transport.receive_gate] r[molten.fabric_transport.direct_stream_bounds]
- [ ] [serial] Execute actual two-host and exact Kiln/Cloud Hypervisor qualification when target/cohort is supplied. r[molten.fabric_transport.direct_stream_bounds]

The repaired parent selection passed 15 direct-stream cases and nine workspace controls. The [earlier complete workspace execution](../../../evidence/integrated-cutover-2026-11-16/earlier-checks.json) and [final ordered source checks](../../../evidence/integrated-cutover-2026-11-16/checks.json) retain their distinct stages, including the earlier failed and later repaired Clippy result. No two-host/Kiln/Cloud Hypervisor execution is asserted; that final task stays open.
