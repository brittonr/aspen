# Design

## Profile and authority

A closed runtime-spine profile binds canonical component projections and the actual executable bytes, ABI, schema, handlers, message scheduler, deterministic clock/RNG and no-dispatch effect semantics. RuntimeState owns in-memory reconstruction; node-host typed ledger/durable roots own local durable storage. The destination descriptor is only an expected-cohort assertion. Unknown profiles remain denied.

An independently selected operator signer authorizes the destination identity, descriptor, exact runtime profile/cohort, generation, current policy/capability/revocation/resource/adapter references and expiration. A separate capability-rooted current-state document supplies those live values. Both observations are reread before activation, with equality fencing. Possession of the bundle and planning flags grants no authority. Local operator provisioning is the trust boundary; this is not a distributed revocation protocol.

## Reconstruction and publication

Verify all eleven bounded canonical content identities and decode their closed semantic projections before recreating runtime handles. A capability-derived typed ledger/durable root opens one Redb store; staged row keys isolate each descriptor without replacing an active world. History entries and complete unresolved request records are inline, not references to missing payloads. Stage every state projection and compare actual stored readback without running effects. The existing RuntimeState owner imports the canonical state and checks resumable counters, RNG, facet graph and scope invariants. Precompute canonical state/descriptor material, then reread exact current admission immediately before the activation transaction. A known pre-commit refusal remains inactive; a commit error is uncertain. A completed activation followed by receipt-store or output failure retains the completed activation fact. Partial reconstruction remains isolated and retained for reconciliation; cleanup never silently deletes evidence. The same descriptor/grant is idempotent, and a different active descriptor cannot be overwritten.

The selected profile is the smallest committed-turn runtime-spine owner, not the daemon, system-extension host, Wasm runtime or separate Fabric SchedulerRuntime. The pending-message scheduler, facets, logical clock, xorshift RNG and effect sequence are actual RuntimeState fields. The library returns that reconstructed owner, and a distinct readback process reconstructs it again from the activated typed store. No unresolved request is dispatched. The local signer/current-document channel is the explicit operator authority boundary, not a substitute for a distributed revocation oracle or for production source admission. Machine-loss and storage-medium durability remain unproven; execution on tmpfs cannot establish NVMe or power-loss claims.

## Evidence

Executable public-route fixture provisions an explicitly experimental signer and exercises separate restore and readback processes. Denials include corrupt/missing components, mismatched actual cohort, unsupported profile, revoked/generation-drift admission, malformed semantic state and receipt publication failure. No execution is claimed until the parent runs the focused rail.
