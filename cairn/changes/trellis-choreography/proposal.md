## Why

Molten needs a way to describe multi-party runtime protocols without scattering send/receive logic across actors, dataspace handlers, policy gates, and transport adapters. ChoRus is useful prior art, but adopting it as the normative protocol layer would introduce a second choreography calculus next to Trellis and would not directly reuse Molten's verified-logic primitives.

Trellis already provides finite global choreography syntax, local endpoint syntax, projectability checks, endpoint projection, and one-step global/local semantics. Molten should build its choreography surface on those primitives and keep the runtime interpreter, dataspace transport, policy checks, and evidence emission as Molten-specific layers.

## What Changes

- Treat Trellis choreography primitives as the normative choreography semantics for Molten.
- Add a Molten protocol manifest/DSL layer that lowers named roles, labels, payload schemas, and protocol metadata into Trellis `GlobalChoreo` values.
- Validate global choreographies with Trellis projectability before installing or running a protocol.
- Project each admitted global choreography to Trellis `LocalChoreo` endpoints for the local role.
- Interpret projected local endpoints over the Molten dataspace by publishing and consuming protocol-message envelopes.
- Carry protocol id, session id, role ids, labels, payload tags, sequence/effect ids, payload references, and admission evidence in runtime envelopes.
- Gate protocol installation, sends, receives, branch choices, and external effects through Nickel/Basalt/Cairn/Trellis policy and receipt boundaries before dataspace side effects occur.
- Use ChoRus only as non-normative API inspiration if useful; do not make `chorus_lib` part of the runtime contract.

## Impact

This adds a verified choreography control-plane target for Molten's runtime spine. The initial implementation can stay finite and in-process: define manifests, lower them to Trellis, project endpoints, and interpret send/receive/choice/offer over the local dataspace. Remote Iroh transport, Wasmtime actors, Steel orchestration, and durable stores can reuse the same protocol-message envelope after the local interpreter exists.
