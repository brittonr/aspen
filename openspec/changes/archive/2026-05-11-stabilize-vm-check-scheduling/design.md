## Context

Aspen's flake exposes many checks, including heavyweight NixOS VM tests. A default parallel `nix flake check -L` run recently produced VM host contention symptoms in `multi-node-kv`, while the focused `multi-node-kv-test` and serialized full rail passed. That makes the default local rail less useful as operator evidence: it can create false product-failure signals on a single development host.

## Goals / Non-Goals

**Goals:**
- Make the default local full-flake rail deterministic for VM-heavy checks.
- Keep focused check targets available for fast triage.
- Record a clear evidence classification rule for future drains.

**Non-Goals:**
- Build a new scheduler for NixOS VM tests.
- Remove or weaken any VM test.
- Claim that serialized full-flake proof replaces dedicated CI capacity planning.

## Decisions

### 1. Serialize local jobs by default in flake config

**Choice:** Change the repository `nixConfig.max-jobs` default from `auto` to `1` and add a source comment explaining that Aspen's VM-heavy checks should be serialized locally unless explicitly overridden.

**Rationale:** This is the smallest guard that changes actual operator behavior for `nix flake check -L`; documentation alone would not prevent repeating the ambiguous parallel run.

**Alternative:** Add only a wrapper script. Rejected for this slice because bare `nix flake check` would still silently use the risky default.

**Implementation:** Edit `flake.nix` near `nixConfig`, validate with `nix flake metadata --json` or `nix eval` where possible, and run OpenSpec/whitespace checks.

## Risks / Trade-offs

**Slower non-VM local full checks** → Operators can still explicitly override with `--max-jobs auto` or a chosen value for non-VM-focused workflows.

**Config trust prompt on first use** → Existing `nixConfig` already requires flake config acceptance for repo defaults; this change only adjusts the value.
