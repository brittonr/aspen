## Why

Molten now has canonical artifacts, typed storage, schema identity, upgrade receipts, and a local evaluation cache, but examples and bug reports still need a first-class way to become repeatable evidence. Ad hoc markdown, shell snippets, and manual CLI sessions can drift from real behavior and can accidentally depend on paths, current working directories, environment variables, or wall-clock time.

A local executable transcript slice gives Molten a deterministic, content-addressed documentation and regression format. Transcripts should run from fresh local state by default, compare canonical Preserves outputs and receipts, and optionally reuse the evaluation cache when all determinism inputs are represented in the key.

## What Changes

- Add local canonical transcript artifacts with markdown source, parsed stanza records, dependency refs, handler profile refs, policy/capability/revocation refs, seed/config refs, and expected output refs.
- Define an initial, deliberately small stanza subset: `molten-cli`, `preserves`, `artifact`, `policy`, `expect`, and `comment`/markdown prose preservation.
- Add stanza modifiers for expected error, known bug, hidden rendered output, skip, required feature, deterministic seed, and handler profile override.
- Implement a fresh-state local transcript runner that creates isolated registry/cache/storage/ledger roots, runs supported stanzas deterministically, and records canonical stanza outcomes.
- Emit canonical transcript receipts for run start, stanza outcomes, expected failures, known bugs, denied effects, cache hits/misses, and final run result.
- Integrate deterministic transcript runs with `eval-cache` using keys that bind transcript artifact refs, dependency closure, handler profile, policy/capability/revocation refs, runner/tool version, and seed/config refs.
- Add local CLI commands for creating/running/showing/rendering transcripts without treating paths or mutable names as transcript identity.

## Impact

Molten documentation, tutorials, and incident reports can become reproducible tests. Upgrade sessions can later require transcript-gate evidence backed either by a fresh deterministic run or by a valid cache hit receipt. This also creates a small, user-facing integration point across the artifact registry, evaluation cache, typed storage, receipts, and existing test harness.
