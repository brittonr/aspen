## Context

Molten already has broad `unison-executable-transcripts` requirements. This change is the first local implementation slice: it adopts the useful pattern of executable documents while using Molten's canonical Preserves envelopes, artifact refs, receipts, effect handles, and evaluation cache. Unison transcripts are prior art only; Molten does not adopt UCM syntax, Unison codebase semantics, hash formats, or typechecker behavior.

The local runner should start with a narrow, deterministic CLI-like subset rather than a general shell. The goal is to make examples and bug repros executable without introducing ambient authority or non-deterministic cache keys.

## Goals

- Represent transcript documents as canonical artifacts with stable refs.
- Parse markdown into bounded, typed stanza records.
- Run transcripts against fresh local state by default.
- Compare canonical Preserves outputs, diagnostics, receipt kinds/decisions, and artifact/cache refs rather than terminal text only.
- Emit transcript receipts that can be imported into the local evidence ledger and referenced by upgrade gates.
- Integrate deterministic runs with the local evaluation cache.
- Render human-readable docs from canonical run evidence while preserving hidden/noisy stanzas in evidence.
- Keep production effects denied unless an explicit handler profile and policy admit them.

## Non-Goals

- Do not implement a general shell transcript runner in this slice.
- Do not allow transcripts to depend on cwd, host env, local paths, mtimes, wall-clock time, or mutable names as identity.
- Do not treat hidden output as hidden evidence; it only affects rendered docs.
- Do not claim transcript output is proof unless it references validated evidence receipts.
- Do not implement distributed transcript sharing or remote cache publication in this slice.
- Do not wire durable choreography protocol-drain sessions yet; expose receipt shapes that can be consumed later.

## Transcript artifact model

Introduce a canonical transcript artifact record:

```preserves
<transcript-artifact-v1 "molten.transcript.artifact.v1"
  <source <markdown-source-ref>>
  <stanzas [<transcript-stanza-v1 ...> ...]>
  <dependencies <closure-hash> [<artifact-or-schema-or-policy-ref> ...]>
  <handler-profile <none> | <some <handler-profile-ref>>>
  <policy [<policy-ref> ...]>
  <capability [<capability-ref> ...]>
  <revocation [<revocation-ref> ...]>
  <seed <seed-ref-or-none>>
  <expected [<expected-output-ref> ...]>
  <checks [<check "no-ambient-identity" "pass"> ...]>>
```

The transcript artifact ref is the canonical hash of this record. The markdown source path used to load a transcript is not identity; only canonical source bytes and parsed stanza values are identity inputs.

## Stanza model

Each fenced block becomes a canonical stanza:

```preserves
<transcript-stanza-v1 "molten.transcript.stanza.v1"
  <index 0>
  <kind "molten-cli" | "preserves" | "artifact" | "policy" | "expect" | "comment">
  <modifiers [<modifier "error"> <modifier "bug"> <modifier "hide"> ...]>
  <input <inline <value-ref>> | <content-ref <manifest-ref>>>
  <refs [<declared-ref> ...]>
  <checks [<check "bounded-stanza" "pass"> ...]>>
```

Initial supported stanza kinds:

- `molten-cli`: a restricted Molten test command expressed as arguments, not shell text.
- `preserves`: canonical value fixture made available by ref.
- `artifact`: inline artifact install or artifact-ref declaration.
- `policy`: policy/capability/revocation fixture refs for later stanzas.
- `expect`: expected canonical value, receipt decision/kind, diagnostic class, artifact ref, cache ref, or trace pattern.
- `comment`: preserved prose anchor for rendering, not execution authority.

Initial modifiers:

- `error`: stanza is expected to fail with a declared error kind/diagnostic.
- `bug`: failure is known and recorded but may be policy-gated out of CI pass decisions.
- `hide`: omit rendered output from human docs while keeping canonical evidence.
- `skip`: record skipped outcome with reason and do not execute.
- `requires`: feature gate that must be satisfied by runner capability refs.
- `seed`: deterministic seed/config override represented by a seed ref.
- `profile`: handler profile override for the stanza.

## Runner modes

The first implementation should support:

- `fresh`: default; create isolated local registry, ledger, typed-storage, cache, and scratch roots; delete scratch after success unless `--save-state` is requested.
- `save`: same as fresh but materializes final roots for inspection and records state refs.

Later modes may include `fork` and `in-place`, but `in-place` must be denied by default with a receipt until explicit maintenance policy exists.

The runner must not use host cwd/env/path/mtime/time as identity. If a local file is loaded, its canonical bytes and parsed value refs are recorded; the path is diagnostic text only.

## Stanza execution

`molten-cli` stanzas should run through an internal command dispatcher or a constrained argument-vector runner. The first admitted commands should be read/write-local-test surfaces that already produce canonical refs, for example:

- `test artifact install/list/view/closure/impact`,
- `test schema identity/compat`,
- `test storage put/get/verify/recipe/migrate`,
- `test cache put/get/status/list/show/invalidate`,
- `test report validate/gate` over transcript-local files.

Unsupported commands deny with a transcript receipt rather than falling back to an ambient shell.

## Expected output and comparisons

Transcript expectations must compare canonical records where possible:

- exact Preserves value refs,
- artifact/cache/storage/schema refs,
- receipt kind and decision,
- diagnostic kind/message substring,
- output presence/absence,
- trace pattern refs,
- expected denied side effects.

Text rendering can be included for human readability, but text alone is not sufficient evidence for pass decisions.

## Receipts

Introduce canonical transcript receipts:

```preserves
<transcript-run-receipt-v1 "molten.transcript.run-receipt.v1"
  <operation "run" | "stanza" | "render" | "cache-hit" | "cache-miss" | "deny">
  <decision "pass" | "deny" | "error" | "skip" | "known-bug">
  <transcript <transcript-artifact-ref>>
  <stanza <none> | <some <stanza-ref>>>
  <mode "fresh" | "save" | "fork" | "in-place-denied">
  <refs [<dependency-ref> <policy-ref> <cache-ref> <receipt-ref> ...]>
  <diagnostics ["..."]>
  <checks [<check "fresh-state" "pass"> ...]>>
```

The final run receipt should bind all stanza outcome refs, initial state refs, final state refs, cache receipts, output refs, and policy/capability/revocation refs.

## Evaluation cache integration

Deterministic transcript runs should use `eval-cache` operation `transcript-run`. The cache key must bind:

- transcript artifact ref,
- dependency closure hash and refs,
- handler profile ref,
- policy/capability/revocation refs,
- runner/tool ref and version,
- seed/config refs,
- expectation refs.

Cache hits are admissible only for deterministic tiers. `production-effectful-trace-only` entries may record observed traces but must not satisfy semantic transcript output expectations.

## Policy and effects

The default local runner profile is deterministic and deny-by-default for production effects. Transcript stanzas that request storage, artifact registry, blob/chunk, network/Iroh, clock/random, or adapter effects must reference handler profiles and policy/capability evidence. Unsupported or undeclared effects emit denial receipts and fail expectations unless explicitly marked `:error`.

## CLI

Add `molten test transcript` commands:

- `parse <markdown> --out transcript.preserves`,
- `run <transcript-or-markdown> --state fresh|save --cache <path> --registry <path> --receipt-out <path> --out <path>`,
- `show <transcript-ref-or-file>`,
- `render <transcript-or-run-receipt> --out <markdown>`.

All commands should print full refs. Paths identify local input/output locations only; they never define transcript or cache identity.

## Tests and properties

Required tests:

- parsing preserves stanza order and stable refs,
- fresh runs are deterministic across different temp roots,
- expected canonical value/ref comparisons pass and mismatches fail closed,
- expected error stanzas pass only for the declared error kind,
- known bug stanzas are recorded distinctly from pass/fail,
- hidden output is omitted from rendering but remains in receipts,
- unsupported shell/production effects are denied by default,
- eval-cache hit is used only when key refs match all determinism inputs,
- Hegel properties for stanza ordering, stable transcript identity, and denied ambient identity.

## Open Questions

- Which exact restricted CLI subcommands should be admitted in the first runner implementation?
- Should expected outputs live embedded in the markdown artifact or as separate artifact refs by default?
- How should known-bug stanzas interact with CI profiles and pass gates?
- When durable choreography sessions exist, should transcript receipts become the primary protocol-drain evidence format?
