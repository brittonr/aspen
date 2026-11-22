# External pilot tracker

This is the integration checklist for one bounded external Molten pilot. It does
not qualify a workflow by itself; each row names the evidence that must exist
before the pilot or release gate can cite it.

## Selected workflow boundary

The first candidate is the [hosted durable counter](addressable-actor-runtime.md#pinned-runnable-flow) on two separate x86-64 NixOS hosts. Host A submits an admitted keyed request over the registered direct-LAN Iroh service; host B runs the installed counter, returns the semantic result, checkpoints, stops its process, and wakes from the same capability-rooted local Redb state. Repeating the logical request must return the durable original result without another commit. A transport acknowledgement alone is not an actor result.

Use the locked default package (`molten-startup-source-binding`; no extra Cargo features), exact `bin/molten`, `molten-service-host-cohort` and `molten-addressable-counter` members, and independently selected source trust, placement, invocation and stream grants. Freeze the package, node profiles, authenticated peer IDs, admitted service, operation and generation, and all stream/resource bounds before the run. The local fixture already exercises the actor lifecycle. The cross-host entry path and its two admitted targets have not been qualified; do not substitute SSH, a same-host run, or an unauthenticated listener.

This profile promises **process-loss recovery on B's existing local store**, not disk/host loss, owner transfer, VM migration, portable snapshots, WAN/relay, or a dormant-agent consumer. The native scope of #19 is the runtime input; its later dormant-agent and quiescence gates remain separate. This pilot selection does not authorize execution on any machine or publish a release.

## Engineering dependencies

| Dependency | Required result | Current state |
| --- | --- | --- |
| #23 | Clean public source/build without SSH credentials or sibling overrides. | Clean hosted [run 43986122156](../.cairn/archive/2026-11-16-admit-anonymous-runtime-sources/evidence/hosted-ci-20261116/full-43986122156-observation.json) at `1e81020e3` passed locked Cargo, README Cargo fixture/cleanup, default Nix build, full flake checks and their required aggregate. Repeat for a later selected external candidate. |
| #16 | Positive signed VM integration with trust for every fresh scenario root. | The [signed fixture at `9905a10d6`](../evidence/signed-vm-9905a10d6/run.json) passed, retaining 132 original files. The separate unsigned check confirmed missing-trust denial. Neither establishes independently selected external operator trust. |
| #24 | Actual fault observations, not authored fixtures. | The [actual status-write denial](../.cairn/archive/2026-11-16-observe-real-vm-storage-denial/evidence/signed-complete-pass-2026-11-16.json) retained the typed pre-write EACCES attempt and matching independent pre/post observations; validators and export manifest passed. Other unsupported fault profiles remain unavailable. |
| #15 | Authenticated cross-host stream with wrong-peer/stale-generation denial and explicit uncertainty. | Blocked on two-host qualification. |
| #7 | Exposure/budget decision for the selected profile. | Loopback-only deferral recorded; reopen if the pilot uses a wider bind. |
| #19 | Hosted counter through the existing node/actor runtime. | [Signed native evidence](../.cairn/archive/2026-11-21-host-public-addressable-actor/tasks.md) covers same-host process recovery and denials under the experimental signer; exact external admission and the two-host entry path remain untested. |
| #20 | Cross-host ownership fencing, only if failover is promised. | Blocked on #15/#19. |
| Kiln #57 | Consumer persistence for a Kiln-hosted pilot, if selected. | External dependency; not started. |
| #22 | Capability report with implementation separate from recorded execution. | [Signed diagnostic run](../.cairn/archive/2026-11-21-qualify-exact-runtime-capabilities/evidence/2026-11-20-experimental-signed-consumer-run/run.json) recorded one bounded-observed harness row and six unsupported product rows. The selected external pilot needs its own bundle. |
| #26 | Supported NixOS module when NixOS is the advertised deployment path. | The [store-image/overlay fixture](../.cairn/archive/2026-11-16-observe-real-vm-storage-denial/evidence/module-store-image-overlay-pass-2026-11-16.json) passed startup, restart, bounded stop and all six trust/source denials. External package/profile and independent operator trust still require their own qualification. |
| #27 | Notices bound to selected dependency revisions. | Row-bound checker and `license-boundary` flake check implemented; rerun for the candidate closure. |
| #28 | Advisory and license audit dispositions. | The [fresh default Linux result](../evidence/dependency-audit-default-2026-11-21/observation.json) denied with 155 findings across 740 packages. The audit rail is delivered; no exception or release admission follows. |
| #29 | Source/documentation drift repaired for touched surfaces. | Prolly and source-sensitive map updated; `doc-links` and `readme-quickstart` flake checks added; rerun for the candidate. |
| #30 | Security and contribution entry points. | Added. Private reporting is enabled per a [retained read-only API observation](../.cairn/archive/2026-11-16-close-doc-and-tool-gaps/evidence/private-vulnerability-reporting-2026-11-16.json) (2026-11-16, owner account); report delivery and maintainer handling are not exercised. |

## Qualification and distribution

- [ ] Run the chosen workflow on two real hosts and retain success, authorization
  denial, interruption, and recovery cases.
- [ ] Bind retained artifacts to the candidate, dependency closure, runtime
  profile, and host environment.
- [ ] Provide the tested entry path and identity (#25) and, when advertised, the
  supported NixOS module (#26).
- [ ] Record audit results/dispositions (#28), notice consistency (#27), and
  documentation drift review (#29).
- [ ] Assemble release notes, exact source acquisition or archive, checksums,
  supported build/install commands, and retained evidence.
- [ ] Verify packaged material for the candidate. Preserve `aspen-orchestration-batch-implementation.zip` unchanged with its older [orchestration source binding](../pilots/orchestration/source.ncl); it is not a counter runtime input or a separate binary deliverable for this pilot. An exact Git source checkout still contains the historic archive.
- [ ] Exercise the public instructions in a fresh environment and record the
  observed prerequisites and limits.
- [ ] Select a new version/tag and prepare a reviewable release record; publish
  only through the existing authorized release workflow.

## Completion boundary

A passed checklist row is evidence for its named profile and environment only.
It is not general production certification, successful cluster truth, security
qualification, or a promise that unsupported profiles work. Preserve failed and
unsupported results.
