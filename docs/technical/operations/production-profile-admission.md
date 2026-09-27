# Production profile admission

A production profile is a reviewed description of deployment inputs, not executable authority. This article assumes familiarity with content refs and the [production operator runbooks](../../production-operator-runbooks.md). It separates Nickel export validation, node configuration resolution, and release evidence review, because passing one boundary does not establish the others. The [Technical companion](../README.md) provides surrounding context.

## Configuration admission has several subjects

The reusable [Nickel contract module](../../production-profile-contracts.ncl) defines schema metadata, state-layout predicates, resource relationships, and finite operational vocabularies. An exported profile carries a candidate source ref, schema ID and version, source language, profile identity, and a profile body. Its contract ties profile identity to the profile name and requires the source-gate input list to equal the singleton candidate ref.

That validation answers a shape-and-relationship question. It does not demonstrate that the referenced policy was approved, that adapters started, or that a store can survive its declared load. The runbooks explicitly separate development, pilot, and release review tiers. Development permits local fixtures; pilot permits bounded operator-readiness evidence with caveats; release requires candidate-specific evidence across source gate, policy, Octet, Cairn, stack provenance, profile, generated export, and accepted Valence policy hash.

The profile's source language is not a runtime instruction. The runbooks state that daemon initialization consumes checked exports or reviewed refs and does not evaluate Nickel at runtime. The [configuration resolver](../../../src/node/profile/parts/config/p000/body.rs) implements that boundary by recognizing `nickel-source` as a source-kind value but producing a denial diagnostic for runtime Nickel evaluation. Recognizable syntax is not admission.

## Structural constraints are useful but not sufficient

State-layout validation requires an absolute state root outside `.git`, safe relative component directories, and distinct directory names. Resource limits must be positive integers; store bytes must be at least receipt bytes, and recovery time must be at least delivery latency. These relationships exclude internally contradictory profiles without claiming measured capacity.

The named exported thresholds are queue depth 1024, receipt size 128 MiB, store size 10 GiB, delivery latency 5000 ms, and recovery time 60000 ms. They are reviewed constants in the Nickel module, not benchmark results or universal safe operating points. The runbook requires threshold review and fixture updates when their exported values change. Adding operational vocabulary likewise requires contract and negative-typo-fixture changes rather than accepting arbitrary strings.

Reference checks also have layers. The inspected Nickel candidate predicate rejects malformed BLAKE3 refs and explicitly excludes the zero, all-`a`, and all-`f` values. The runbook states a broader release-tier prohibition on placeholder and repeated-character material. The narrow Nickel predicate should therefore not be paraphrased as a complete implementation of every release-tier evidence rule. Export success remains distinct from release validation and review.

## Runtime resolution preserves what was admitted

`CheckedNodeProfile` carries the profile and optional actual profile refs, source kind, tier, metadata, state-root ref, adapter bindings, and policy, capability, resource, and effect-profile refs. The resolver validates these inputs, collects mismatch and adapter diagnostics, applies admitted overrides, builds a configuration value, and produces configuration and resolution refs.

When an actual profile ref is supplied and differs from the expected ref, it records a mismatch. Absence of that optional field is not equivalent to independently observing matching bytes. Review should distinguish a caller-supplied reference assertion from the upstream process that calculated and reviewed it.

The [override implementation](../../../src/node/profile/parts/config/p001/body.rs) permits an override only when its field is explicitly listed and the tier is not release. Release overrides are denied even if their field appears in the allowlist. Effective configuration changes only for allowed non-release overrides. Diagnostics are sorted and deduplicated before resolution; denial-bearing diagnostics determine the decision, and the resolution binds the resulting config ref and metadata.

The no-profile route is deliberately different. Local-default resolution records `local-fixture-config`, uses development tier, and retains the caveat. It does not silently inherit release standing from a successful local startup.

## Worked example: a plausible relocation is still denied

Consider an illustrative release profile reviewed for one state-root ref. An operator initializes a node while supplying a different state-root override, perhaps because the original filesystem is full. Both refs may be well-formed, and the alternate location may be operationally sensible. The release resolver nevertheless denies the override; the allowlist does not relax the release rule.

That denial preserves the meaning of reviewed inputs. Quietly accepting the new location would conflate “a valid path exists” with “the reviewed deployment configuration still applies.” The appropriate review question is whether revised evidence and a revised profile support the intended deployment, not whether the original command can be made to pass by dropping its tier.

A second independent failure occurs if expected and actual profile refs disagree. Correcting the state-root issue does not resolve the content mismatch. Admission diagnostics should be reviewed as separate constraints, not as a single error string to suppress.

## Verification guidance and limits

Suggested verification follows the runbook: export with an explicit reviewed candidate, execute positive and negative profile fixtures, run release-profile validation for the same candidate, retain deployment-profile evidence, and retain node profile-resolution evidence. The runbook specifies that invalid release-profile input writes a canonical deny value before unsuccessful exit; its Nix fixture proves command wiring, not release eligibility.

No export, fixture, or runtime command was executed for this article. The inspected local daemon startup additionally uses a synthetic test source-gate receipt, as visible in the [startup implementation](../../../src/node/parts/daemon/p018/body.rs). Its local success cannot satisfy the runbook's requirement for a current candidate-and-policy source gate. Configuration metadata, content identity, resolution success, adapter observations, authority, and production promotion remain distinct review subjects.

## Sources

- [Production operator runbooks](../../production-operator-runbooks.md)
- [Production profile contracts and thresholds](../../production-profile-contracts.ncl)
- [Checked profile and source-kind validation](../../../src/node/profile/parts/config/p000/body.rs)
- [Override and resolution mechanics](../../../src/node/profile/parts/config/p001/body.rs)
- [Local daemon startup evidence](../../../src/node/parts/daemon/p018/body.rs)
- [Technical companion](../README.md)
