## Why

Molten already stores immutable artifact refs, models names as metadata, fences system-extension generations, records retention pins, and admits effect manifests and handler profiles. Two gaps remain:

- normative uses resolve names to exact refs permanently, so there is no explicit one-snapshot late-bound service boundary for new units of work while old work stays pinned; and
- generation cleanup has subsystem-specific pins but no shared complete-root reachability report that can explain exactly why an old generation remains live or conservatively refuse retirement when observations are incomplete.

Effect requests also need to consume Kamacite's strengthened semantic operation identities so handler, replay, cache, remote-execution, and upgrade decisions fail loudly when operation behavior changes without an admitted compatibility artifact.

## What Changes

- Adopt a revision-pinned `artifact-binding-core` for pure successor-binding, snapshot-resolution, reachability, pin-path, and retirement-classification mechanics without transferring Molten authority or Preserves schema ownership.
- Add canonical Molten binding, resolution, root-inventory, retirement-report, and deploy-diagnostic artifacts around the shared pure outputs.
- Resolve late-bound service/system-extension names exactly once per admitted request, turn, callback pass, job, or protocol session; all work below that boundary pins the resolved artifact and dependency closure.
- Require complete registered roots and generation attribution before reporting a generation retired; uninstrumented or incomplete native execution remains `unknown`, never retired.
- Keep retirement evidence separate from retention and GC authority.
- Consume exact Kamacite semantic effect-operation identities in manifests, handler bindings, handles, requests, responses, logs, replay/cache identities, remote execution, and upgrade compatibility checks.
- Add deploy diagnostics for stale compare-and-swap state, incompatible binding targets, unreachable replacements, semantic handler mismatch, incomplete root inventories, and concrete generation pin paths.

## Impact

- **Files**: `cairn/changes/adopt-artifact-binding-and-semantic-effects/**`; later implementation affects `crates/molten-core`, runtime/system-extension shells, Preserves schemas, registry/ledger/retention/effects/replay/cache/remote-execution modules, CLI readback, docs, dependency pins, and fixtures.
- **Testing**: Cairn validation and gates now; implementation requires baseline and post-change focused core tests, shared-core compatibility fixtures, positive and negative binding/effect/retirement tests, cyclic and incomplete-root properties, system-extension and protocol-session integration, replay/cache checks, retention non-authority checks, Octet, lifecycle, and relevant Nix rails.
