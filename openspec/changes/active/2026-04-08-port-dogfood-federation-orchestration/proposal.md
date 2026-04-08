## Why

The `aspen-dogfood` parity fix restored federation state, cookies, env wiring, and CI watch registration, but it intentionally did **not** port the deprecated federation pipeline's mid-stream orchestration. The old `scripts/deprecated/dogfood-federation.sh` still contains behavior the Rust binary does not perform yet: explicit federation setup steps, mirror/repo propagation work, and the end-to-end federation build handoff.

Without a dedicated follow-up, the parity fix change has to carry an open checkbox for work that is out of scope for that change.

## Scope

Port the missing federation orchestration steps from `scripts/deprecated/dogfood-federation.sh` into `crates/aspen-dogfood`, keeping the already-landed cookie/env/trust fixes intact.

## Expected outcome

After this follow-up:

- federation mode performs the missing `federate` / `sync` / mirror-creation steps in Rust
- bob can consume the mirrored Forge state through the intended federation flow
- `aspen-dogfood` federation mode reaches the same mid-pipeline behavior the deprecated shell script provided
- verification includes a federation-focused test or reproducible command transcript, not just unit coverage
