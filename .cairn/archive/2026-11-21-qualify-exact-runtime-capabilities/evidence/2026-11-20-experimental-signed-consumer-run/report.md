# Capability qualification report

Report: `blake3:689a5cbf88b37b48c242d19f3025c59467ebc9e58372c4c2c19f7cb89ca10d72`

Source: `blake3:fff8b53eb12833687d82447569c7fc49cffc219e35aab1d08859ef05a3ee86f6`; build: `blake3:35fbf48e179093913ce918d4f03d7710f302296eebeb9de246aa287db91e9344`; profile: `harness-simulation-v1`; platform: `x86_64-linux`; environment: `Simulation`

| Capability | Implementation | Execution | Missing families |
|---|---|---|---|
| control-plane-failover | 0 matrix entries / 2 requirements | **unsupported** | 0 |
| cross-host-transport | 0 matrix entries / 2 requirements | **unsupported** | 0 |
| cross-process-transport | 0 matrix entries / 2 requirements | **unsupported** | 0 |
| durable-state-recovery | 0 matrix entries / 1 requirements | **unsupported** | 0 |
| effect-uncertainty | 2 matrix entries / 2 requirements | **unsupported** | 0 |
| harness-runtime-capability-binding | 2 matrix entries / 1 requirements | **bounded-observed** | 0 |
| world-replay-promotion-restore | 0 matrix entries / 3 requirements | **unsupported** | 0 |

- control-plane-failover: selected profile/environment is outside the reviewed capability mapping

- cross-host-transport: selected profile/environment is outside the reviewed capability mapping

- cross-process-transport: selected profile/environment is outside the reviewed capability mapping

- durable-state-recovery: selected profile/environment is outside the reviewed capability mapping

- effect-uncertainty: selected profile/environment is outside the reviewed capability mapping

- harness-runtime-capability-binding / harness-simulation / `blake3:c0401b866e3cda3b83a573c4a79c0f43b2be800429019ff461816841714c17c0`: **pass** (native expected-deny), validated=true, exact-subject=true; scenario: denied-send
  - finite native harness simulation only; not live transport, durability, cross-host or release qualification

- harness-runtime-capability-binding / harness-simulation / `blake3:89a97f931ad17d5f6dc6cbc0a7078b6a007302e42b97c459733e3e0e329264ac`: **pass** (native pass), validated=true, exact-subject=true; scenario: granted-send
  - finite native harness simulation only; not live transport, durability, cross-host or release qualification

- world-replay-promotion-restore: selected profile/environment is outside the reviewed capability mapping

_diagnostic only; no operation authority, release admission, production readiness or live-scope promotion_
