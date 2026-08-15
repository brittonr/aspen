## Context

Molten already treats JUnit and rendered logs as non-normative views over canonical test-run receipts. The repository also has a Nix `nextest-config` check that preserves Nextest profile readbacks. The missing piece is semantic enforcement: profile names and README descriptions should correspond to actual test selection rules.

## Decisions

### Profiles have reviewed semantic rows

**Choice:** Define a small manifest or in-code table that records profile id, Nextest profile name, filter expression or test metadata contract, expected artifacts, retry policy, evidence scope, cost class, and platform availability.

**Rationale:** The profile matrix can be validated independently from any one command invocation and can feed canonical CI receipt generation.

### Deterministic profiles exclude non-replayable tests by default

**Choice:** `fast-core`, `harness`, `cli`, and `distributed-simulation` reject live-only, VM-only, exploratory, or diagnostic-only tests unless those tests are explicitly marked as excluded from deterministic pass evidence. `vm-platform` and `dogfood-soak` record platform/live caveats.

**Rationale:** Retry or live success cannot count as deterministic replay evidence.

### Config readback remains evidence-only

**Choice:** The Nix `nextest-config` check and CLI readback validate profile metadata, filters, JUnit paths, and retry behavior, then emit review evidence. They do not replace the actual test run receipt or subsystem gates.

**Rationale:** Configuration correctness is necessary but not sufficient for release evidence.

## Functional core / shell split

- Pure core: validate profile rows, filter presence, retry policy, artifact paths, evidence scope, platform flags, and non-replayable exclusions from in-memory records.
- Shell: read `.config/nextest.toml`, call `cargo nextest show-config`, discover test metadata, write readback artifacts, and render diagnostics.

## Validation strategy

- Add positive profile manifest/readback tests.
- Add negative fixtures for missing filter, wrong retry policy, deterministic profile containing live-only tests, JUnit-only evidence misuse, duplicate profile id, and missing expected artifact path.
- Run `nix build .#checks.$system.nextest-config --no-link`, focused testing-hardening tests, and Cairn validation/gates.

## Non-claims

Passing semantic profile validation does not prove that the underlying tests pass, that replay evidence is complete, or that release promotion should pass. It only proves the test profile configuration matches its documented evidence scope.
