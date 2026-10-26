# Proposal: Pilot the corrected extracted-runtime consumer boundary

## Why

The historical Molten Wasm pilot lives outside current repository conventions and targets an incomplete published extraction. A direct call to an old module helper is not evidence that a current consumer can supply explicit policy, resource, epoch, world, import/export, and materialization inputs to the corrected Component Model API.

This bounded change supplies a current-source standalone Molten actor pilot, without pretending that product runtime migration has occurred.

## What Changes

- Add an isolated `pilots/hwr-consumer` Cargo workspace and real Component Model actor guest with a Molten-owned `actor-turn` contract. r[molten.hwr_consumer_pilot.actor]
- Pass consumer-owned policy, bounds, exact byte identity, world/export facts, and an actively operated epoch service through the corrected producer API. r[molten.hwr_consumer_pilot.boundary]
- Keep dependencies externally revision-pinned and document the temporary historical pin replacement prerequisite. r[molten.hwr_consumer_pilot.pin]
- Provide exact bounded-output and denial assertions, an operator command, ownership, and explicit limits on the resulting evidence. r[molten.hwr_consumer_pilot.evidence]

## Impact

- **Files:** `pilots/hwr-consumer/`, the existing governing `docs/wasm-component-runtime.md`, and this native Cairn change only.
- **Owner:** Molten Wasm component runtime maintainers (`brittonr/aspen`).
- **Immediate outcome:** complete pilot source; final corrected immutable producer pin and execution evidence pending.
- **Durable contribution:** repeatable external-consumer Component Model boundary exercise with product semantics outside the producer.
- **Verification:** after replacing the historical pin, run `cargo run --manifest-path pilots/hwr-consumer/Cargo.toml`; record exact source identities, output, fuel observation, and denials. No execution is claimed by this proposal.

## Non-goals

No production runtime, pure core, root dependencies/lock/build plans, historical archive, or banned legacy crate changes. No production adoption, parity, policy migration, trusted Mantle materialization, canonical Preserves ABI, host effects, fallback core-module execution, ambient WASI, release authority, or whole-system correctness claim. The separate shared-runtime adoption change remains open on its own terms.
