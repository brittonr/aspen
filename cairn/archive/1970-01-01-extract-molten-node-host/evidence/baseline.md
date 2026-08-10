# Baseline evidence

- Published source: `ee3998eca2fc8a1d119407e3d58cc501212a1be3`
- Canonical product branch: `origin/molten`
- Workspace command: `nix develop path:$PWD -c env CARGO_TARGET_DIR=/home/brittonr/.cargo-target/molten-node-host cargo test --workspace --locked`
- Workspace result: pass
  - `molten` library: 1,252 passed
  - `molten` binary: 51 passed
  - `cliharness`: 57 passed
  - `fabric_simulation_boundary`: 3 passed
  - `molten-core`: 173 passed
  - `molten-release-policy`: 5 passed
  - total: 1,541 passed, 0 failed, 0 ignored
- Focused node-state command: `cargo test -p molten node_state::tests --locked`
- Focused node-state result: 10 passed, 0 failed
- Focused local-store command: `cargo test -p molten local_store::tests --locked`
- Focused local-store result: 6 passed, 0 failed
- Pinned Cairn revision: `3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`
- Cairn layout: legacy `cairn/`, as admitted by the repository-pinned lifecycle tool
- Proposal gate receipt: `26912d8c10421d80b9f2463e36771f9bdb87d7aa8a091a128fab33c8aa1444a4`
- Design gate receipt: `14883a4e2016e1e7d8a460ec7ea0b5fd8edf2cd3c59ab78b1eb8bb700e044e71`
- Tasks gate receipt: `05f4049ee7a1362ebb2d432f39a108bbc4620d46d3b05cd77f5ab011ce9630bd`

The current workspace Cairn tool rejects the legacy layout and requests migration. This change uses the repository-pinned Cairn revision. It does not migrate unrelated lifecycle state.
