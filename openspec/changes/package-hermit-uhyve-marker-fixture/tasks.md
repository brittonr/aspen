## Phase 1: Spec Foundation

- [x] [serial] Define the reproducible Hermit marker fixture package contract and anti-overclaiming boundary.

## Phase 2: Package Fixture

- [ ] [serial] Add a source-built `.#hermit-uhyve-marker` package that emits a valid `x86_64-unknown-hermit` image printing `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED` plus provenance metadata.
- [ ] [depends:package] Add a cheap package/contract check proving the marker image path exists, is executable/readable, and its metadata pins source revision, output path, and expected marker.

## Phase 3: Product-Path Reproduction

- [ ] [depends:package] Update Hermit/Uhyve harness/docs to use the packaged marker image in the gated proof command while preserving explicit opt-in KVM/Uhyve prerequisites.
- [ ] [depends:package] Rerun the ignored real Hermit/Uhyve product-path proof with `.#uhyve` and `.#hermit-uhyve-marker`, then save receipt evidence under this change.
