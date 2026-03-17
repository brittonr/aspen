## ADDED Requirements

### Requirement: Detect npins project type

The CI executor SHALL detect npins projects by checking for `npins/sources.json` in the project root. When detected and the `snix-eval` feature is enabled, the executor SHALL use in-process evaluation instead of the `nix eval` subprocess.

#### Scenario: npins project detected

- **WHEN** a `ci_nix_build` job is submitted with a project directory containing `npins/sources.json`
- **THEN** the executor SHALL attempt in-process evaluation via `NixEvaluator`

#### Scenario: flake project detected

- **WHEN** a `ci_nix_build` job is submitted with a project directory containing `flake.nix` but no `npins/sources.json`
- **THEN** the executor SHALL use the `nix eval --raw .drvPath` subprocess path

### Requirement: Evaluate npins project to drvPath in-process

The executor SHALL evaluate the npins project's attribute to a `.drvPath` string using `NixEvaluator::evaluate_with_store()`. The evaluation SHALL use snix-eval with snix-glue fetcher builtins. No subprocess SHALL be spawned for the evaluation step.

#### Scenario: Successful npins evaluation

- **WHEN** an npins project with `fetchTarball`-based pins is evaluated
- **THEN** the executor SHALL return a valid `/nix/store/*.drv` path
- **THEN** no `nix` subprocess SHALL have been spawned

#### Scenario: Evaluation fails due to unsupported builtin

- **WHEN** snix-eval encounters an unimplemented builtin (e.g., `fetchGit`)
- **THEN** the executor SHALL fall back to the `nix eval` subprocess path
- **THEN** the executor SHALL log which builtin was unsupported

### Requirement: Full native pipeline without subprocess

When npins detection and evaluation both succeed, the entire build pipeline SHALL execute in-process: eval → parse .drv → `LocalStoreBuildService` sandbox → ingest output → upload to PathInfoService. Zero external subprocesses.

#### Scenario: End-to-end npins native build

- **WHEN** an npins project with a trivial derivation is built
- **THEN** the job SHALL complete successfully
- **THEN** the output store path SHALL exist
- **THEN** no `nix` or `nix-daemon` subprocess SHALL have been spawned for eval or build
- **THEN** the output SHALL be uploaded to PathInfoService and served by the cache gateway
