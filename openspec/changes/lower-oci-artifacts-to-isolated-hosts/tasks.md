## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta spec for OCI artifact lowering.

## Phase 2: Runtime model update

- [ ] Replace or demote `RuntimeHostKind::OciContainer` so production runtime declarations use `RuntimeArtifact::OciImage` plus an isolated lowering target rather than a plain container host.
- [ ] Add portable lowering-plan model types for original OCI digest, selected target host, derived artifact identities, declared handles, and unsupported-image diagnostics.
- [ ] Add admission tests rejecting production OCI declarations without an isolated lowering target and accepting OCI-to-microVM lowering plans.

## Phase 3: Documentation and evidence

- [ ] Update `docs/runtime-applications.md` to describe OCI as artifact ingestion/lowering, not a production Podman-style host.
- [ ] Add docs/source-anchor tests that prevent reintroducing ordinary containers as the default production boundary.
- [ ] Run focused runtime-core tests, docs tests, strict OpenSpec validation, and whitespace checks.
