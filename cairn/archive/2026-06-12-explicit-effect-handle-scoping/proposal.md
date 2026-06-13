## Why

Molten already requires explicit policy, capability, authority, resource, and executor evidence before side effects. However, many effect requests are still identified primarily by effect kind and surrounding actor/session context. That is enough for simple cases, but it becomes ambiguous when one actor has access to multiple stores, dataspaces, clocks, blob stores, peer channels, replay logs, or adapter instances of the same kind.

Bluefin's value-level effect handles are useful prior art here: effect operations name the concrete handle they use, and the handle is scoped to the handler that introduced it. Molten should adapt that discipline at trust and effect boundaries without adopting Bluefin's Haskell API or implementation.

## What Changes

- Add canonical handler-binding and effect-handle evidence for admitted effect surfaces.
- First require executor hostcall requests to bind explicit handler/handle refs; extend the same `handle_ref` discipline to generic effect requests and adapters in later slices.
- Require evidence-bearing effect requests and hostcalls to bind an explicit `handle_ref` in addition to effect kind, operation, actor/session/run context, policy, capability, authority, and resource refs.
- Make handle identity deterministic and replayable, derived from canonical handler-binding evidence plus scope and sequence, not ambient runtime allocation.
- Validate handler-scoped lifetimes: handles must be introduced before use, used only within admitted actor/session/run/turn scope, and rejected after expiry or revocation.
- Disambiguate multiple same-kind effects by handle ref, not by type-level or stringly effect kind alone.
- Support compound handler profiles that expose several related handles while preserving per-handle authority/resource/policy evidence.
- Default remote or transferred handles to local-only denial unless explicit attenuation/delegation/remote-proxy evidence exists.

## Impact

This strengthens replay, hostcall validation, adapter isolation, and future multi-store/multi-peer execution. The first milestone can add schemas, validators, and harness tests around existing hostcall/effect envelopes without changing the pure runtime core.
