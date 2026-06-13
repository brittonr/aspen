## Why

Molten already leans toward a pure core, explicit effects, deterministic playback, typed artifacts, and property-driven testing. Haskell's ecosystem provides useful design patterns for making those ideas precise: pure core/effectful shell, capability-style effects, QuickCheck/Hedgehog property laws and shrinking, STM-style atomic transactions, typeclass-law conformance, newtype authority discipline, typed protocol state machines, golden tests, parser-combinator DSLs, optics for structured traversal, and strictness/resource guards.

These are patterns to adapt, not compatibility targets. Molten should not require GHC, Cabal, Stack, Haskell package APIs, laziness semantics, monad transformer libraries, or Haskell syntax. The value is the law-oriented engineering discipline: make effects explicit, make authority typed, make adapters lawful, make tests shrink and replay, and make resource behavior bounded.

## What Changes

- Define Haskell-inspired runtime laws as Molten design constraints without claiming Haskell compatibility.
- Strengthen the pure-core/effectful-shell split: core transitions are deterministic data transformations; IO, clocks, random, storage, network, policy, tracing, Wasm, Steel, and process effects occur only through admitted handlers.
- Treat effect manifests and handler profiles as mtl-style capability dictionaries: actors declare required capabilities/effects and runtime profiles bind handlers for local, record, replay, chaos, profiling, and production execution.
- Require Hegel-style property suites for implemented invariants; persisted seeds, shrink paths, and counterexamples must be canonical Preserves fixtures when suites export them.
- Make actor turns STM-like transactions: state deltas, assertions, messages, and effect intents are staged, validated, admitted, and committed atomically or rolled back.
- Add/recognize adapter law/conformance coverage for implemented storage, blob/chunk, policy, network, Wasm hostcall, Steel orchestration, scheduler, and replay boundaries; broader suites remain explicit future extensions.
- Add newtype/phantom-authority discipline for ids, refs, capabilities, schema refs, receipts, effect logs, secret refs, and runtime states so authority and identity cannot be confused.
- Use typed protocol/state-machine patterns with Trellis-backed gates for legal transitions and replayable diagnostics.
- Use golden canonical traces and Preserves fixtures rather than text-only golden output; review receipts for every golden update remain an extension unless a suite implements them.
- Use parser-combinator-style building blocks where deterministic parsers already cross Preserves/transcript/policy/oracle boundaries.
- Use optics/lens-inspired structured traversal for redaction, canonical diffs, evidence filtering, and safe report rendering.
- Add strictness/resource guards so deferred work, queues, traces, effects, and actor state remain bounded and testable.

## Impact

This change gives Molten a law-oriented implementation style. The archived milestone records the implemented discipline: pure transition functions and predicates, explicit effect manifests/profiles/requests/responses, typed wrapper refs and schema distinctions, transactional turn staging and rollback receipts, Hegel property coverage across implemented invariants, adapter law evidence for existing boundaries, structured redaction evidence, and bounded resource guards. General reserve/commit/abort adapter APIs, automatic shrunk-counterexample export for every suite, WIT-derived effect traits, and review receipts for every golden update remain future explicit extensions.
