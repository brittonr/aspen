## Phase 1: Audit the missing federation path

- [ ] Compare `crates/aspen-dogfood` federation subcommands against `scripts/deprecated/dogfood-federation.sh` and list the still-unported orchestration steps.
- [ ] Identify where the Rust flow should create or mirror repos, trigger sync, and hand off the build to bob.

## Phase 2: Implement federation orchestration parity

- [ ] Port the missing `federate` / `sync` / mirror-creation steps into the Rust binary.
- [ ] Ensure the build path in federation mode runs against the correctly mirrored repo on bob.
- [ ] Add regression tests for the pure orchestration helpers introduced by the port.

## Phase 3: Verification

- [ ] Run a federation-oriented verification flow that exercises the new orchestration path.
- [ ] Capture command output or VM/integration evidence showing the mirrored-repo path works end to end.
