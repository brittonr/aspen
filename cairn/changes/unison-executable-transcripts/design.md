## Context

Molten already plans docs/transcripts as artifacts, but the runner semantics deserve an explicit design. A transcript should be a human-readable markdown file and a machine-checkable artifact that describes runtime interactions from a fresh state. It should be able to demonstrate envelope routing, policy denial, choreography projection, storage migration, remote sync, and upgrade sessions.

Unison transcripts are prior art: fenced code blocks define code and UCM commands, expected failures can be marked, noisy output can be hidden, and runs use fresh ephemeral codebases by default. Molten should adapt these ideas around Preserves envelopes, policies, handler profiles, and receipts.

## Goals

- Make docs, examples, tutorials, and bug reproductions executable and reproducible.
- Run transcripts against fresh local deterministic state by default.
- Compare canonical outputs, trace records, and receipts rather than fragile terminal text only.
- Support expected errors and known bugs without stopping the whole transcript.
- Store transcripts and their outputs as content-addressed artifacts.
- Let CI and local CLI run selected transcript suites.

## Non-Goals

- Do not adopt UCM or Unison transcript syntax exactly.
- Do not allow transcripts to perform production side effects unless explicitly admitted.
- Do not treat hidden output as hidden evidence; hidden only affects rendered docs.
- Do not make transcripts a replacement for property tests or formal Trellis predicates.
- Do not allow transcript runs to depend on ambient local state by default.

## Transcript structure

A transcript is markdown with fenced stanzas. Initial stanza kinds may include:

- `molten-config`: Nickel or canonical config fixture.
- `molten-cli`: CLI command to run against the transcript runtime.
- `preserves`: envelope, schema, or value fixture.
- `policy`: Nickel/Basalt/Steel policy fixture.
- `artifact`: inline artifact metadata or reference.
- `expect`: canonical expected result, trace pattern, receipt pattern, or diagnostic.
- `shell` (optional, restricted): host shell command only in explicitly admitted developer/CI contexts.

Modifiers may include `:error`, `:bug`, `:hide`, `:skip`, `:requires`, `:seed`, and `:profile`.

## Execution modes

- `fresh`: default; creates a new local registry/runtime/store and deletes it after success.
- `save`: creates a fresh state and saves resulting registry/runtime snapshot for inspection.
- `fork`: copies an existing named fixture state and writes output to a new state.
- `in-place`: mutates an existing state; denied by default and intended only for controlled maintenance.

All modes must record the initial state hash, transcript artifact id, handler profile, policy refs, and resulting output hash.

## Expected output

Terminal output is useful for humans but should not be the sole oracle. Expected stanzas should support:

- exact canonical Preserves values,
- pattern matches over trace records,
- expected receipt kinds and decisions,
- expected diagnostics and error classes,
- expected artifact ids or dependency closure hashes,
- expected absence of side effects.

Text rendering may be regenerated from canonical records.

## State and reproducibility

Transcript runs should pin:

- transcript artifact id,
- dependency closure hash,
- runner version,
- handler profile and deterministic seed/config,
- initial registry/runtime/store state hash,
- policy artifacts,
- expected output artifact ids.

A transcript can be cached only if its handler profile is deterministic and all dependencies are represented in the cache key.

## Policy and evidence

Running a transcript may require capabilities if it installs artifacts, starts actors, writes storage, or performs adapter effects. Production network/filesystem effects are denied unless the transcript declares them and policy admits them. Each run emits a transcript-run receipt with stanza outcomes, trace refs, cache refs, and output artifact refs.

## Open Questions

- Should transcript expected-output files be separate artifacts or embedded in the markdown artifact?
- Which CLI subset is stable enough for the first transcript runner?
- Should transcript rendering prefer text diffs, Preserves diffs, or both?
- How should `:bug` stanzas interact with CI pass/fail policy?
