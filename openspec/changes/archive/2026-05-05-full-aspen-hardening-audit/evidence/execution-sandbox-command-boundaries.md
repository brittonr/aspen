# Execution sandbox and command-boundary audit

Generated: 2026-05-05T12:10:48Z

## Scope

Audited representative execution boundaries for jobs, CI executors, native Nix/SNIX build paths, and fallback subprocess gates. The slice focused on command spawning, working-directory confinement, shell wrapping, timeout/cancellation behavior, and subprocess fallback feature gates.

No live credentials, tokens, tickets, or secret-bearing files were read. This evidence uses source inspection and synthetic filesystem fixtures only.

## Source handles

- `crates/aspen-ci/src/orchestrator/pipeline/executor.rs:148-318` — CI job configs become worker `JobSpec` payloads; shell payloads separate command/args unless a full shell expression is explicitly configured; deploy jobs are not dispatched to the worker queue.
- `crates/aspen-ci/src/orchestrator/pipeline/executor.rs:320-388` — Nix payloads preserve flake fields, timeout, artifact/cache metadata, sandbox flag, and source hash for VM workers.
- `crates/aspen-ci/src/orchestrator/pipeline/executor.rs:390-435` — VM payloads pass command/args/env/timeout/source hash to isolated Cloud Hypervisor workers and do not pass host checkout paths.
- `crates/aspen-ci/src/agent/executor/validation.rs:10-45` — the split CI agent canonicalizes working directories before workspace-prefix checks, covering symlink/traversal escapes.
- `crates/aspen-ci-executor-shell/src/agent/executor.rs:122-185` — shell executor validates working directory before process spawn and returns bounded result metadata.
- `crates/aspen-ci-executor-shell/src/agent/executor.rs:210-249` — remediated shell executor validation now requires absolute, existing, canonical workspace or tmpfs-fallback paths before command execution.
- `crates/aspen-ci-executor-shell/src/agent/executor.rs:252-275` — command execution uses `Command::new` with explicit args, clears inherited environment, installs bounded default PATH only when absent, and spawns a process group.
- `crates/aspen-ci-executor-shell/src/agent/executor.rs:286-420` — stdout/stderr line length is bounded, timeout/cancellation terminates the process group, and heartbeat lifecycle is bounded.
- `crates/aspen-ci-executor-nix/src/executor.rs:94-106` — native SNIX build path is preferred; subprocess `nix build` fallback is retained behind `nix-cli-fallback`.
- `crates/aspen-ci-executor-nix/src/executor.rs:182-253` — `nix eval --raw` fallback is passed as positional installable args, not interpolated through a shell.
- `crates/aspen-ci-executor-nix/src/executor.rs:255-325` — native build attempts in-process evaluation/build and only falls back to subprocess when the feature gate is enabled and native eval cannot satisfy the request.

## Finding and remediation

The legacy `aspen-ci-executor-shell` working-directory guard checked lexical prefixes with `Path::starts_with` before process spawn. That rejected obvious paths outside `/workspace`, but it did not canonicalize the requested working directory or the configured workspace root, so a path under the workspace that was a symlink to an outside directory could pass the prefix check and escape when `current_dir` followed the symlink.

Remediation in this slice:

- Added canonical working-directory validation to `crates/aspen-ci-executor-shell/src/agent/executor.rs`.
- Requires request paths to be absolute and existing before execution.
- Canonicalizes both the request path and workspace root before prefix comparison.
- Preserves the existing `/tmp/ci-workspace-*` tmpfs fallback only when both the request spelling and canonical target remain under that bounded fallback prefix.
- Added regressions for accepted canonical workspace children and rejected symlink escapes.

## Verified behavior

- Shell executor no longer accepts a symlink located under the configured workspace root when the symlink target canonicalizes outside that root.
- Existing outside-workspace, root, relative-path, and nonexistent-path rejection behavior remains covered.
- The split `aspen-ci` agent already had canonical validation; this slice aligned the standalone shell executor with that hardened boundary.
- Nix/SNIX fallback subprocesses observed in this slice pass command/arguments through `Command::new`/`.arg()` without shell interpolation and are feature-gated behind `nix-cli-fallback` where applicable.

## Verification commands

```sh
rustfmt crates/aspen-ci-executor-shell/src/agent/executor.rs
cargo test -p aspen-ci-executor-shell validate_working_dir -- --nocapture
cargo check -p aspen-ci-executor-shell
scripts/tigerstyle-check.sh
openspec validate full-aspen-hardening-audit --strict --json
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify full-aspen-hardening-audit --json || true
git diff --check
python -m json.tool openspec/changes/full-aspen-hardening-audit/evidence/execution-sandbox-command-boundaries.json >/dev/null
```

## Evidence scan note

Credential-like terms in this artifact are contextual security vocabulary only. No credential values are present.
