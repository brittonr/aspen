# Readiness checker negative mutation evidence

V2 requires negative checker mutations proving non-zero exits for invalid readiness labels, incomplete exception metadata, forbidden transitive paths, and re-export leaks.

The checker output paths stay directly under `openspec/changes/prepare-crate-extraction/evidence/` while each mutation runs so the checker reads the real `verification.md` index instead of a temp-tree placeholder.

## blocked-readiness

Mutation: mark one candidate `publishable from monorepo`, which is blocked until license/publication policy exists.

```text
$ scripts/check-crate-extraction-readiness.rs --policy /tmp/tmp.z7SH5b4wcD/blocked-readiness/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family redb-raft-kv --output-json openspec/changes/prepare-crate-extraction/evidence/negative-blocked-readiness.json --output-markdown openspec/changes/prepare-crate-extraction/evidence/negative-blocked-readiness.md
Using saved setting for 'builders = ' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'connect-timeout = 30' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'download-attempts = 3' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-substituters = https://cache.nixos.org https://microvm.cachix.org' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= microvm.cachix.org-1:oXnBc6hRE3eX5rSYdRyMYXnfzcCxC7yKPTbZXALsqys=' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'fallback = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'keepOutputs = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-free = 10737418240' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-jobs = auto' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'min-free = 5368709120' from ~/.local/share/nix/trusted-settings.json.
Aspen development environment

Build caching:
  incremental compilation enabled

Common commands:
  cargo build                          Build project
  cargo nextest run                    Run all tests
  cargo nextest run -P quick           Quick tests (~2-5 min)
  cargo tigerstyle check               Run Tiger Style lints

Nix apps:
  nix run .#cluster                    3-node cluster
  nix run .#bench                      Run benchmarks
  nix run .#coverage [html|ci|update]  Code coverage
  nix run .#fuzz-quick                 Fuzzing smoke test

VM testing: aspen-vm-setup / aspen-vm-run <node-id>
  nix run .#dogfood-vm                 Dogfood in isolated VMs

CI worker setup (x86_64-linux only):
  Status: ENABLED (kernel/initrd paths set)
Error: crate extraction readiness check failed with 1 failure(s)
exit_code=1
```

### Generated failure report

```text
# Crate Extraction Readiness Report

- Candidate family: `redb-raft-kv`
- Passed: `false`
- Checked candidates: 6

## Failures

- aspen_kv_types: forbidden readiness state `publishable from monorepo`

## Warnings

- none
```

## empty-exception-owner

Mutation: blank the owner on `aspen-kv-types -> aspen-constants` exception metadata.

```text
$ scripts/check-crate-extraction-readiness.rs --policy /tmp/tmp.z7SH5b4wcD/empty-exception-owner/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family redb-raft-kv --output-json openspec/changes/prepare-crate-extraction/evidence/negative-empty-exception-owner.json --output-markdown openspec/changes/prepare-crate-extraction/evidence/negative-empty-exception-owner.md
Using saved setting for 'builders = ' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'connect-timeout = 30' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'download-attempts = 3' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-substituters = https://cache.nixos.org https://microvm.cachix.org' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= microvm.cachix.org-1:oXnBc6hRE3eX5rSYdRyMYXnfzcCxC7yKPTbZXALsqys=' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'fallback = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'keepOutputs = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-free = 10737418240' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-jobs = auto' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'min-free = 5368709120' from ~/.local/share/nix/trusted-settings.json.
Aspen development environment

Build caching:
  incremental compilation enabled

Common commands:
  cargo build                          Build project
  cargo nextest run                    Run all tests
  cargo nextest run -P quick           Quick tests (~2-5 min)
  cargo tigerstyle check               Run Tiger Style lints

Nix apps:
  nix run .#cluster                    3-node cluster
  nix run .#bench                      Run benchmarks
  nix run .#coverage [html|ci|update]  Code coverage
  nix run .#fuzz-quick                 Fuzzing smoke test

VM testing: aspen-vm-setup / aspen-vm-run <node-id>
  nix run .#dogfood-vm                 Dogfood in isolated VMs

CI worker setup (x86_64-linux only):
  Status: ENABLED (kernel/initrd paths set)
Error: crate extraction readiness check failed with 1 failure(s)
exit_code=1
```

### Generated failure report

```text
# Crate Extraction Readiness Report

- Candidate family: `redb-raft-kv`
- Passed: `false`
- Checked candidates: 6

## Failures

- aspen_kv_types: exception `aspen-kv-types -> aspen-constants` has unassigned owner

## Warnings

- none
```

## forbidden-transitive

Mutation: add transitive crate `serde_core` to `forbidden_by_default`, proving cargo-tree transitive scans fail non-zero when a forbidden transitive path is reachable.

```text
$ scripts/check-crate-extraction-readiness.rs --policy /tmp/tmp.z7SH5b4wcD/forbidden-transitive/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family redb-raft-kv --output-json openspec/changes/prepare-crate-extraction/evidence/negative-forbidden-transitive.json --output-markdown openspec/changes/prepare-crate-extraction/evidence/negative-forbidden-transitive.md
Using saved setting for 'builders = ' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'connect-timeout = 30' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'download-attempts = 3' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-substituters = https://cache.nixos.org https://microvm.cachix.org' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= microvm.cachix.org-1:oXnBc6hRE3eX5rSYdRyMYXnfzcCxC7yKPTbZXALsqys=' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'fallback = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'keepOutputs = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-free = 10737418240' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-jobs = auto' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'min-free = 5368709120' from ~/.local/share/nix/trusted-settings.json.
Aspen development environment

Build caching:
  incremental compilation enabled

Common commands:
  cargo build                          Build project
  cargo nextest run                    Run all tests
  cargo nextest run -P quick           Quick tests (~2-5 min)
  cargo tigerstyle check               Run Tiger Style lints

Nix apps:
  nix run .#cluster                    3-node cluster
  nix run .#bench                      Run benchmarks
  nix run .#coverage [html|ci|update]  Code coverage
  nix run .#fuzz-quick                 Fuzzing smoke test

VM testing: aspen-vm-setup / aspen-vm-run <node-id>
  nix run .#dogfood-vm                 Dogfood in isolated VMs

CI worker setup (x86_64-linux only):
  Status: ENABLED (kernel/initrd paths set)
Error: crate extraction readiness check failed with 4 failure(s)
exit_code=1
```

### Generated failure report

```text
# Crate Extraction Readiness Report

- Candidate family: `redb-raft-kv`
- Passed: `false`
- Checked candidates: 6

## Failures

- aspen_kv_types: transitive forbidden dependency `serde_core`
- aspen_raft_kv: transitive forbidden dependency `serde_core`
- aspen_raft_kv_types: transitive forbidden dependency `serde_core`
- aspen_redb_storage: transitive forbidden dependency `serde_core`

## Warnings

- none
```

## empty-reexporters

Mutation: erase `aspen-kv-types` re-exporters, proving re-export path metadata omissions fail non-zero.

```text
$ scripts/check-crate-extraction-readiness.rs --policy /tmp/tmp.z7SH5b4wcD/empty-reexporters/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family redb-raft-kv --output-json openspec/changes/prepare-crate-extraction/evidence/negative-empty-reexporters.json --output-markdown openspec/changes/prepare-crate-extraction/evidence/negative-empty-reexporters.md
Using saved setting for 'builders = ' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'connect-timeout = 30' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'download-attempts = 3' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-substituters = https://cache.nixos.org https://microvm.cachix.org' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'extra-trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= microvm.cachix.org-1:oXnBc6hRE3eX5rSYdRyMYXnfzcCxC7yKPTbZXALsqys=' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'fallback = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'keepOutputs = true' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-free = 10737418240' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'max-jobs = auto' from ~/.local/share/nix/trusted-settings.json.
Using saved setting for 'min-free = 5368709120' from ~/.local/share/nix/trusted-settings.json.
Aspen development environment

Build caching:
  incremental compilation enabled

Common commands:
  cargo build                          Build project
  cargo nextest run                    Run all tests
  cargo nextest run -P quick           Quick tests (~2-5 min)
  cargo tigerstyle check               Run Tiger Style lints

Nix apps:
  nix run .#cluster                    3-node cluster
  nix run .#bench                      Run benchmarks
  nix run .#coverage [html|ci|update]  Code coverage
  nix run .#fuzz-quick                 Fuzzing smoke test

VM testing: aspen-vm-setup / aspen-vm-run <node-id>
  nix run .#dogfood-vm                 Dogfood in isolated VMs

CI worker setup (x86_64-linux only):
  Status: ENABLED (kernel/initrd paths set)
Error: crate extraction readiness check failed with 1 failure(s)
exit_code=1
```

### Generated failure report

```text
# Crate Extraction Readiness Report

- Candidate family: `redb-raft-kv`
- Passed: `false`
- Checked candidates: 6

## Failures

- aspen_kv_types: no re-exporters recorded

## Warnings

- none
```

All negative mutations failed as expected.
