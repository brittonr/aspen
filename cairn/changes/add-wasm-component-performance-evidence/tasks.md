## Tasks

- [ ] [serial] Define typed Nickel fast/deep benchmark suites, representative component fixtures, phase markers, host requirements, measurement profiles, and BLAKE3 suite identities. r[molten.wasm_performance.suite] r[molten.wasm_performance.phases]
- [ ] [depends:wasm-performance-suite] Implement pure suite validation, run-compatibility checks, sample normalization, confidence/effect-size comparison, and stable regression classes. r[molten.wasm_performance.comparison] r[molten.wasm_performance.functional_core]
- [ ] [depends:wasm-performance-suite] Integrate Sightglass-backed compilation, instantiation, and execution runs with canonical Molten performance receipts and auxiliary human reports. r[molten.wasm_performance.phases] r[molten.wasm_performance.evidence]
- [ ] [serial] Add exact precompiled-component admission over Mantle/Valence identities and deny unknown, stale, cross-target, cross-configuration, or tampered `.cwasm` before deserialization. r[molten.wasm_performance.aot_admission]
- [ ] [depends:wasm-performance-comparison] Add named pooling, copy-on-write heap image, `InstancePre`, compilation-strategy, and bounded-concurrency profiles with deterministic conformance reruns. r[molten.wasm_performance.optimizations]
- [ ] [parallel] Add Wizer build fixtures with denied imports by default, deterministic virtual input profiles, repeated-output identity checks, and pre/post transform receipts. r[molten.wasm_performance.wizer]
- [ ] [parallel] Add positive baseline/improvement fixtures and negative incompatible-host, stale-suite, insufficient-sample, resource-exhaustion, AOT-tamper, Wizer-drift, and cross-runtime comparison fixtures. r[molten.wasm_performance.validation]
- [ ] [depends:wasm-performance-evidence] Export benchmark evidence as recorded-only, add operator trend readback, and preserve correctness/authority/release non-claims. r[molten.wasm_performance.evidence]
- [ ] [depends:wasm-performance-validation] Run fast and deep benchmark smoke checks, deterministic component conformance, focused tests, receipt validation, Octet checks, Cairn validation, and proposal/design/tasks gates. r[molten.wasm_performance.validation]
