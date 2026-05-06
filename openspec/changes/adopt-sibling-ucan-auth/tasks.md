## Phase 1: Spec foundation

- [x] [serial] Create proposal, design, delta spec, and implementation task rail for switching Aspen auth to `../ucan`.

## Phase 2: Boundary inventory

- [x] [serial] Inventory current `aspen-auth-core`, `aspen-auth`, `aspen-token`, federation credential, and RPC admission call sites that issue, parse, verify, delegate, or authorize capability tokens. ✅ 2m 20s (started: 2026-05-06T23:21:13Z → completed: 2026-05-06T23:23:33Z) Evidence: `evidence/i1-aspen-auth-callsite-inventory.md`.
- [x] [serial] Inventory the sibling `../ucan` public APIs and decide which APIs belong in `aspen-auth-core` versus the runtime `aspen-auth` shell. ✅ 3m 12s (started: 2026-05-06T23:24:19Z → completed: 2026-05-06T23:27:31Z) Evidence: `evidence/i2-sibling-ucan-api-inventory.md`.
- [ ] [depends:boundary-inventory] Write the Aspen capability/operation to UCAN ability/resource mapping table, including unsupported or intentionally Aspen-local capabilities.

## Phase 3: Dependency wiring

- [ ] [depends:boundary-inventory] Add controlled Cargo/Nix wiring for `../ucan` / `../ucan/crates/ucan-core` with a documented local-development path and reproducible CI/release fallback or failure mode.
- [ ] [depends:dependency-wiring] Prove dependency boundaries with `cargo tree`/feature checks for `aspen-auth-core`, `aspen-auth`, and protected `aspen-core --no-default-features` paths.

## Phase 4: Adapter and migration implementation

- [ ] [depends:capability-mapping] Implement the UCAN-backed adapter that preserves Aspen-facing `Capability`, `Operation`, token CLI/RPC, and redacted receipt behavior.
- [ ] [depends:ucan-adapter] Add compatibility fixtures for existing Aspen token generation/inspection/delegation behavior or documented migration receipts for intentional format changes.
- [ ] [depends:ucan-adapter] Switch runtime verification/admission paths to the UCAN-backed verifier only after compatibility and negative evidence exists.

## Phase 5: Verification and docs

- [ ] [depends:runtime-switch] Add positive UCAN/Aspen round-trip tests and negative escalation, expiry, malformed proof, wrong audience, replay/revocation, and denied capability mapping tests.
- [ ] [depends:verification-tests] Update auth/federation/operator docs with the adapter boundary, sibling dependency policy, migration notes, and unsupported UCAN interoperability caveats.
- [ ] [depends:docs] Run targeted Rust tests, dependency graph checks, Nix/source-boundary checks, `openspec validate adopt-sibling-ucan-auth --strict`, helper verification, and `git diff --check`; archive only after all retained evidence tasks are complete.
