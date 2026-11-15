# Public addressable-actor consumer

## Why

Issue #19 requires a real public-node consumer of the accepted actor profile, not a simulation or another service core. Existing actor state transitions and Redb storage are reusable, but a native callback result is not durable semantic completion.

## Change

Compose the existing actor shell with native hosting, current placement and operator authority, resource observations, and coordination delivery. Publish a public request/result boundary that distinguishes durable completion from lifecycle progress, denial, stale state, duplicate replay, and uncertainty. Preserve exact manifest/profile/source bindings and explicit survival classes.

## Boundaries

No second mailbox, scheduler, assignment ledger, journal, authority engine, or semantic store. No startup-source bypass, synthetic admission booleans, live fallback during replay, or universal exactly-once claim. Missing current-owner adapters block live qualification rather than becoming simulated passing evidence.
