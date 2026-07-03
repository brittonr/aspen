# Design: layered proof evidence contract

## Scope

This change names the expected proof layers and their boundaries. It does not create a new authority source and does not let a higher-layer summary override a lower-layer canonical denial.

## Layers

The initial layers are pure-core proof, gate proof, replay proof, release proof, and operator readback. A layer may aggregate lower layers only by canonical refs and explicit compatibility policy.

## Boundary checks

Each layer should record evidence-only caveats and supported subject scope. Cross-layer validation denies when the child ref is stale, the subject scope changes without an explicit bridge, the child decision is not acceptable, or a diagnostic/readback layer is used as pass evidence.

## Hegel RS properties

Generated layer graphs should cover stable topological sorting, stale child denial, cycle denial, wrong-scope denial, diagnostic-layer non-pass, and unchanged lower refs producing unchanged aggregate refs.

## Non-goals

- No automatic trust promotion.
- No replacement for requirement traceability.
