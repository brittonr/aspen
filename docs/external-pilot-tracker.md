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
| #23 | Clean public source/build without SSH credentials or sibling overrides. | Implemented source graph; clean public-runner evidence still required. |
| #16 | Positive signed VM integration with trust for every fresh scenario root. | Sender-root provisioning fixed; signed rail not executed. |
| #24 | Actual fault observations, not authored fixtures. | Storage denial explicitly unavailable; executed denial still required. |
| #15 | Authenticated cross-host stream with wrong-peer/stale-generation denial and explicit uncertainty. | Blocked on two-host qualification. |
| #7 | Exposure/budget decision for the selected profile. | Loopback-only deferral recorded; reopen if the pilot uses a wider bind. |
| #17 / #18 / #19 | Selected runtime composition: promotion, restore, or hosted actor. | Blocked on concrete current-authority, runtime, or public-node adapters. |
| #20 | Cross-host ownership fencing, only if failover is promised. | Blocked on #15/#19. |
| Kiln #57 | Consumer persistence for a Kiln-hosted pilot, if selected. | External dependency; not started. |
| #22 | Capability report with implementation separate from recorded execution. | Implemented tool; pilot bundle still required. |
| #26 | Supported NixOS module when NixOS is the advertised deployment path. | Module/eval implemented; opt-in VM rail not executed. |
| #27 | Notices bound to selected dependency revisions. | Implemented checker; rerun for the candidate closure. |
| #28 | Advisory and license audit dispositions. | Implemented tool; candidate snapshot and dispositions still required. |
| #29 | Source/documentation drift repaired for touched surfaces. | Prolly and source-sensitive map updated; rerun for the candidate. |
| #30 | Security and contribution entry points. | Added; private reporting channel still needs maintainer verification. |

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
