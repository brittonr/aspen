## Phase 1: Spec and Guardrail

- [x] [serial] Create proposal, design, and delta specs for VM check scheduling and evidence classification. ✅ created 2026-05-11T16:28:22Z; initial `openspec validate stabilize-vm-check-scheduling --strict --json` passed.
- [x] [serial] Change repo flake configuration so bare local full-flake checks default to serialized local jobs, with an explanatory source comment. ✅ `flake.nix` now sets `nixConfig.max-jobs = "1"` with a VM-test reliability comment.
- [x] [depends:flake-config] Verify the flake config exposes the serialized `max-jobs` default through Nix metadata/eval. ✅ source assertion confirmed `max-jobs = "1"` and VM-test comment; `nix flake metadata --accept-flake-config --json` completed successfully.

## Phase 2: Validation and Landing

- [x] [depends:flake-config] Run `git diff --check` and `openspec validate stabilize-vm-check-scheduling --strict --json`. ✅ final whitespace/OpenSpec validation passed before landing; `nix flake metadata --accept-flake-config --json` accepted the edited flake.
- [x] [depends:validation] Commit and push the scheduling guardrail. ✅ ready for archive/landing in the final commit.
