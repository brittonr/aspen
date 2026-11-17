# External pilot tracker

This is the integration checklist for one bounded external Molten pilot. It does
not qualify a workflow by itself; each row names the evidence that must exist
before the pilot or release gate can cite it.

## Selected workflow boundary

Use one named workflow, for example an admitted infrastructure job on two NixOS
hosts with bounded failure/recovery and inspectable receipts. The exact packages,
features, adapters, authority/trust inputs, supported platform, persistence
boundary, and exclusions must be recorded before any run. Executable promotion
(#17) or a hosted actor (#19) may supply the workload; restore (#18) is required
only if the workflow promises snapshot restoration.

## Engineering dependencies

| Dependency | Required result | Current state |
| --- | --- | --- |
| #23 | Clean public source/build without SSH credentials or sibling overrides. | Clean hosted [run 43986122156](../.cairn/archive/2026-11-16-admit-anonymous-runtime-sources/evidence/hosted-ci-20261116/full-43986122156-observation.json) at `1e81020e3` passed locked Cargo, README Cargo fixture/cleanup, default Nix build, full flake checks and their required aggregate. Repeat for a later selected external candidate. |
| #16 | Positive signed VM integration with trust for every fresh scenario root. | The [complete signed fixture](../.cairn/archive/2026-11-16-observe-real-vm-storage-denial/evidence/signed-complete-pass-2026-11-16.json) passed with all 132 original exported files retained. Its VM-local signer/source/profile does not establish independently selected external operator trust. |
| #24 | Actual fault observations, not authored fixtures. | The [actual status-write denial](../.cairn/archive/2026-11-16-observe-real-vm-storage-denial/evidence/signed-complete-pass-2026-11-16.json) retained the typed pre-write EACCES attempt and matching independent pre/post observations; validators and export manifest passed. Other unsupported fault profiles remain unavailable. |
| #15 | Authenticated cross-host stream with wrong-peer/stale-generation denial and explicit uncertainty. | Blocked on two-host qualification. |
| #7 | Exposure/budget decision for the selected profile. | Loopback-only deferral recorded; reopen if the pilot uses a wider bind. |
| #17 / #18 / #19 | Selected runtime composition: promotion, restore, or hosted actor. | Blocked on concrete current-authority, runtime, or public-node adapters. |
| #20 | Cross-host ownership fencing, only if failover is promised. | Blocked on #15/#19. |
| Kiln #57 | Consumer persistence for a Kiln-hosted pilot, if selected. | External dependency; not started. |
| #22 | Capability report with implementation separate from recorded execution. | Implemented tool; pilot bundle still required. |
| #26 | Supported NixOS module when NixOS is the advertised deployment path. | The [store-image/overlay fixture](../.cairn/archive/2026-11-16-observe-real-vm-storage-denial/evidence/module-store-image-overlay-pass-2026-11-16.json) passed startup, restart, bounded stop and all six trust/source denials. External package/profile and independent operator trust still require their own qualification. |
| #27 | Notices bound to selected dependency revisions. | Row-bound checker and `license-boundary` flake check implemented; rerun for the candidate closure. |
| #28 | Advisory and license audit dispositions. | Implemented tool; candidate snapshot and dispositions still required. |
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
- [ ] Inventory packaged material, including the historical
  `aspen-orchestration-batch-implementation.zip`; retain historical archives and
  required fixtures.
- [ ] Exercise the public instructions in a fresh environment and record the
  observed prerequisites and limits.
- [ ] Select a new version/tag and prepare a reviewable release record; publish
  only through the existing authorized release workflow.

## Completion boundary

A passed checklist row is evidence for its named profile and environment only.
It is not general production certification, successful cluster truth, security
qualification, or a promise that unsupported profiles work. Preserve failed and
unsupported results.
