# Aspen security policy

<a id="security-policy"></a>

## Support boundary

Aspen has no production-supported release at present. The current branch is
`molten`; the `v0.1.0` tag records the August limited internal pilot, not a
general-purpose security or production qualification.

Passing a bounded simulation, NixOS VM fixture, dependency audit, source gate,
or Cairn lifecycle check does not establish a security boundary for arbitrary
hosts, guests, workloads, transport peers, or operators. The current supported
scope is one local x86_64 Linux node profile using the reviewed local adapters.
Cross-host transport, hosted actors, VM migration, WAN/relay paths, Byzantine
fault tolerance, and the `nixosModules.molten` operator module are not yet
qualified by this repository. See the open issues for their current status.

## Operating precautions

- Use a dedicated test host without production credentials, sensitive host
  data, or production network access. Treat kernels, Wasm modules, artifacts,
  snapshots, receipts, and imported evidence as untrusted inputs.
- Keep listener and node-control surfaces on loopback unless a separately
  reviewed profile explicitly admits a wider bind. Transport identity is
  connectivity evidence, not membership or authority.
- Logs, receipts, and replay artifacts can contain workload data. Review and
  redact artifacts before sharing them; preserve exact originals privately when
  redaction would invalidate an identity you need to reproduce. Evidence
  committed to this repository uses the placeholders and publication guard in
  [CONTRIBUTING.md](CONTRIBUTING.md#publishing-evidence).
- Run the repository's dependency, source, and lifecycle gates against the
  candidate you actually intend to use. Old passing output is not a current
  advisory assessment.

## Reporting a vulnerability

Do not put exploitable details, credentials, private workload data, or memory
dumps in a public issue. Report suspected Aspen vulnerabilities through the
maintainer-selected [private GitHub advisory form](https://github.com/brittonr/aspen/security/advisories/new).
Private vulnerability reporting is enabled for this repository. The repository
owner's account re-read that setting through the read-only GitHub API on
2026-11-16 and the response was `{"enabled":true}`; the
[retained observation](.cairn/archive/2026-11-16-close-doc-and-tool-gaps/evidence/private-vulnerability-reporting-2026-11-16.json)
records the endpoint, result and observer. That read confirms the setting only:
no test report was submitted, so delivery, notification and maintainer handling
of a report have not been exercised. No response-time, fix-time or backport
commitment is offered.

Include the exact Aspen source revision, dependency revision, host and kernel
versions, affected command or profile, a minimal reviewed reproducer, observed
impact, and relevant artifact identities. Keep sensitive artifacts private.

Use the private advisory thread for maintainer triage, reproducer review, fix
coordination and agreed disclosure. Keep sensitive technical details private
until that disclosure is agreed; a public issue or a passing fixture is not a
substitute for the private channel.
