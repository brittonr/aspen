# Tasks

- [x] [serial] Implement bounded deterministic receive gate and private GatedInbound with compile-fail coverage. r[molten.fabric_transport.receive_gate]
- [x] [serial] Add explicit direct profile and exact actual-peer/protocol/role/generation/operation admission. r[molten.fabric_transport.direct_lan]
- [x] [serial] Implement fixed-memory protected-socket streaming with lifecycle/uncertainty and no replay. r[molten.fabric_transport.direct_stream_bounds]
- [x] [serial] Author real same-host separate-process denial/fault/cleanup qualification tests. r[molten.fabric_transport.direct_lan] r[molten.fabric_transport.receive_gate] r[molten.fabric_transport.direct_stream_bounds]
- [x] [serial] Document selected consumer/runtime handoff and external qualification prerequisites. r[molten.fabric_transport.direct_lan] r[molten.fabric_transport.direct_stream_bounds]
- [x] [serial] Parent runs focused tests, source gates and strict lifecycle validation after integration. r[molten.fabric_transport.direct_lan] r[molten.fabric_transport.receive_gate] r[molten.fabric_transport.direct_stream_bounds]
- [x] [serial] Qualify cancellation, deadline and dropped-future boundaries with independently observed protected effects, cleanup and replay fencing; repair any incorrect non-delivery classification. r[molten.fabric_transport.direct_stream_bounds]
- [ ] [serial] Execute actual two-host and exact Kiln/Cloud Hypervisor qualification when target/cohort is supplied. r[molten.fabric_transport.direct_stream_bounds]

The repaired parent selection passed 15 direct-stream cases and nine workspace controls. The [earlier complete workspace execution](../../../evidence/integrated-cutover-2026-11-16/earlier-checks.json) and [final ordered source checks](../../../evidence/integrated-cutover-2026-11-16/checks.json) retain their distinct stages, including the earlier failed and later repaired Clippy result. No two-host/Kiln/Cloud Hypervisor execution is asserted; that final task stays open.

The cancellation review on issue #15 reproduced a false `NotDelivered` after a protected open wrote to its peer and was then cancelled ([before-fix observation](evidence/cancellation-20261117/reproduction.json)). The operation now marks entry into `open` before awaiting it. The 2026-11-17 parent run passed 44 direct-stream tests, 25 transport library tests including the registered-request drop test, the protected-port doc test, Clippy and a clean Octet workspace check; its [retained barrier observations](evidence/cancellation-20261117/checks.json) are same-host loopback evidence only.
