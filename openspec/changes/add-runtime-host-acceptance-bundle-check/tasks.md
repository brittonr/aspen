## Phase 1: Check Contract

- [x] [serial] Create the OpenSpec baseline for a runtime-host acceptance bundle check.
- [ ] [serial] Inventory the stable docs, manifest, inventory, marker, and non-proof anchors that the bundle should assert.

## Phase 2: Implementation

- [ ] [depends:inventory] Implement the deterministic acceptance-bundle check in the narrowest existing docs/harness test or Nix check surface.
- [ ] [depends:implementation] Add positive coverage for promoted product-path rows and negative coverage for build-only/fake-runner overclaims.

## Phase 3: Verification

- [ ] [depends:coverage] Run the new bundle check, `scripts/test-harness.sh check`, runtime-host docs guard tests, OpenSpec validation, and whitespace checks.
