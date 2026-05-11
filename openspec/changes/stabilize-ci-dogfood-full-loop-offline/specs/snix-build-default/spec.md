## ADDED Requirements

### Requirement: Offline deterministic CI dogfood full-loop fixture [r[snix-build-default.ci-dogfood-full-loop-offline]]

Dogfood VM tests that exercise full CI pipeline Nix jobs MUST use deterministic flake inputs that can be resolved without public registry, DNS, substituter, or lock-file update access from inside the guest.

#### Scenario: Full-loop fixture avoids public registry resolution [r[snix-build-default.ci-dogfood-full-loop-offline.no-registry]]

- GIVEN `ci-dogfood-full-loop-test` has pushed its sample repository into Forge
- WHEN the CI `syntax-check`, `build-and-test`, or `unit-tests` job runs `nix build` in the checked-out repository
- THEN the flake input graph SHALL resolve from a store-resident, copied, or otherwise test-local source
- AND the job log SHALL NOT contain attempts to fetch `https://channels.nixos.org/flake-registry.json`
- AND the job SHALL NOT require updating a missing `flake.lock` through public `nixpkgs` registry lookup

#### Scenario: Fixture input failure is classified separately from CI orchestration failure [r[snix-build-default.ci-dogfood-full-loop-offline.failure-classification]]

- GIVEN a full-loop dogfood VM run fails before any staged job can build the sample artifact
- WHEN the failure log contains registry, DNS, lock-update, or external input resolution errors
- THEN the result SHALL be reported as a fixture determinism failure rather than as accepted evidence about stage dependency ordering
- AND the remediation SHALL keep the pipeline proof narrow instead of broadening support claims for full self-hosting acceptance

### Requirement: CI dogfood full-loop stage proof remains intact [r[snix-build-default.ci-dogfood-full-loop-stage-proof]]

The deterministic fixture fix MUST preserve the full-loop acceptance intent: Forge push triggers a CI run; jobs are assigned to the expected stages; dependency ordering prevents later stages from running before earlier stages succeed; and the built artifact is retrieved and executed by the test.

#### Scenario: Three-stage pipeline succeeds with local inputs [r[snix-build-default.ci-dogfood-full-loop-stage-proof.success]]

- GIVEN the full-loop fixture uses only deterministic local/store inputs
- WHEN `ci-dogfood-full-loop-test` runs to completion
- THEN `format-check` and `syntax-check` SHALL complete successfully in the `check` stage
- AND `build-and-test` SHALL complete successfully only after the `check` stage succeeds
- AND `unit-tests` SHALL complete successfully only after the `build` stage succeeds
- AND the test SHALL verify the CI-built artifact or equivalent stage output without relying on external network resolution

#### Scenario: Feature-complete CI node is required [r[snix-build-default.ci-dogfood-full-loop-stage-proof.features]]

- GIVEN a VM test submits `type = 'nix` CI jobs through `aspen-node`
- WHEN the test's node package is selected in `flake.nix`
- THEN the package feature set SHALL include CI job execution support and the expected Nix build fallback/native path features (`ci`, `shell-worker`, `snix`, `snix-build`, and `nix-cli-fallback` where subprocess fallback is part of the acceptance path)
