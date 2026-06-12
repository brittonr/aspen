# Design: replay-evidence-mcp-ux

## Overview

Add `search_replay_evidence` to the catalog MCP read-only allow-list and dispatch table. The tool builds on the existing catalog search core and the replay classifications added by `replay-receipt-catalog`.

## Tool arguments

The tool accepts normal catalog MCP search arguments plus replay-specific optional filters:

- `stage`: `verify` or `first-divergence`, mapped to `deterministic-replay:<stage>`.
- `decision`: mapped to `replay-decision:<decision>`.
- `divergence`: mapped to `replay-divergence:<kind>`.
- `actor-id`: mapped to `replay-actor:<actor>`.
- `handler-profile-ref`: mapped to `replay-handler-profile:<ref>`.
- `expected-report-ref`, `actual-report-ref`, `final-state-ref`.
- `expected-ref`, `actual-ref` for first-divergence evidence.
- fixture replay refs for output, effect-log, and expected/actual final states.

The dispatch layer also adds the broad `deterministic-replay:` text filter so the named tool is scoped to replay evidence.

## Evidence boundary

`search_replay_evidence` returns catalog query evidence and a catalog MCP receipt exactly like other read-only catalog tools. It does not import trust, mutate ledgers, admit source gates, authorize effects, or replace replay/gate verification.
