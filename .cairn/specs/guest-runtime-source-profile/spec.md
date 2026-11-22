# Guest Runtime Source Profile Specification

## Purpose

Defines the `guest-runtime-source-profile` capability.

## Requirements

### Requirement: The guest profile is distinct

r[molten.startup_binding.guest_profile] Startup-binding admission MUST accept `molten-nix-guest-runtime-binding-v1` as a distinct producer profile and MUST continue to reject every unknown producer profile. Full profile behavior MUST remain unchanged.

#### Scenario: Guest profile is admitted
- GIVEN a binding uses the guest runtime profile and otherwise valid identities
- WHEN admission runs
- THEN it MUST accept the binding.

#### Scenario: Unknown profile is rejected
- GIVEN a binding uses any other producer profile
- WHEN admission runs
- THEN it MUST reject it as `UnsupportedProducer`.

### Requirement: The producer measures the runtime cohort

r[molten.startup_binding.guest_producer] The guest producer MUST build `build-closure` and all six closure graphs from the actual resolved runtime paths, record real NAR identities, emit an input-addressed output, and sign it with the selected operator key.

#### Scenario: Producer output is signed
- GIVEN a full signed output and an operator key
- WHEN the guest producer runs
- THEN the output MUST be input-addressed, signed by that key, and contain a guest-profile binding.

### Requirement: Guest admission proves only the recorded runtime

r[molten.startup_binding.guest_admission] Guest admission MUST authenticate the selected producer and measure the exact runtime cohort and executing binary. It MUST NOT rely on a node-root cache for output authentication, executable identity, or gate validation, or claim omitted build, compiler, or gate closure paths.

#### Scenario: Runtime cohort is retained
- GIVEN a guest output and trust
- WHEN the collaboration driver runs
- THEN it MUST retain the baseline observation and explicitly classify the evidence as runtime-only.

### Requirement: Guest campaign evidence is bounded

r[molten.startup_binding.guest_campaign] A KVM guest campaign MUST retain the output, trust, runtime closure, kernel, initrd, schedule, and observations, and MUST NOT promote runtime-only admission to full build provenance or production authority.

#### Scenario: Campaign is classified
- GIVEN a completed guest campaign
- WHEN evidence is retained
- THEN the receipt MUST include the runtime-only non-claims.
