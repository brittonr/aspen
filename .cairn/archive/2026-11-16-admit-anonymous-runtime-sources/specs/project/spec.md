# Reproducible Sources Delta

## MODIFIED Requirements

### Requirement: Git dependencies use immutable release pins

r[molten.project.reproducible_dependencies.exact_pins] Every Git dependency in the Molten release closure MUST be bound to an immutable revision in the repository-owned dependency source of truth, and manifest, lockfile, and Nix source identities MUST agree.

#### Scenario: Floating Git source fails release validation

- GIVEN a release dependency is bound only to a branch, tag, moving reference, or unpinned SSH URL
- WHEN release dependency validation runs
- THEN validation MUST fail before the dependency contributes to release evidence.

#### Scenario: Lock drift fails release validation

- GIVEN the manifest or Nix source pin names one immutable revision and the Cargo lockfile resolves another
- WHEN release dependency validation runs
- THEN validation MUST fail with a revision-drift diagnostic naming both identities.

#### Scenario: A published anonymous coordinate preserves the implementation pin

- GIVEN a provider publishes the selected exact commit through anonymous HTTPS Git
- WHEN the consumer changes its transport coordinate
- THEN manifests, source policy, Cargo/Nix locks and generated source/build plans MUST agree on that coordinate and unchanged implementation revision
- AND source availability alone MUST NOT qualify a clean build, runtime, authority or release claim.

#### Scenario: An unavailable provider remains explicit

- GIVEN a required source cannot be acquired through the selected public path
- WHEN a public build is assessed
- THEN it MUST remain blocked rather than using hidden owner credentials, sibling source overrides or a fabricated clean result.

#### Scenario: A nested tool dependency preserves its exact source identity

- GIVEN a pinned tool input still selects an SSH transport and the same exact commit is anonymously available over reviewed HTTPS
- WHEN its root-owned nested input override is migrated
- THEN the selected implementation revision and metadata-free NAR MUST remain unchanged, with no provider-source or frozen verifier edits
- AND remaining unavailable optional sources MUST stay explicit rather than becoming an inferred clean Nix build.

#### Scenario: An unavailable private provider ships as an owner-approved exact snapshot

- GIVEN a required source has no anonymous route and its owner approves publishing exactly the build-required files
- WHEN the consumer vendors them
- THEN every shipped upstream file MUST match the exact revision by Git blob and BLAKE3 identity, and extra, missing, symlinked or undeclared files MUST fail the check
- AND Aspen-owned derived files MUST be declared beside the unchanged originals, and the snapshot MUST NOT be treated as feature, runtime, authority or release qualification.

#### Scenario: A pinned tool's public SSH-named source is fetched over HTTPS

- GIVEN a pinned tool declares a public GitHub dependency by SSH URL that the consumer cannot override
- WHEN builds rewrite only that transport to anonymous HTTPS
- THEN the tool revision and fetched commit MUST be unchanged
- AND the rewrite MUST be scoped to the build environment and documented with its removal trigger.
