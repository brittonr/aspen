# Aspen release status

Aspen is the public project and release name. **Molten is the development codename.** The default branch, Cargo packages, executable, Nix attributes, schema identifiers, and historical artifacts still use `molten` where compatibility requires it.

The current branch is a development preview. There is no production-supported release. The following scope was reviewed against source revision `7c854d24d143ce788458e73e38500cfd31153896` for the public documentation rewrite; this review did not execute the build, runtime, VM, or release gates.

The [external pilot tracker](external-pilot-tracker.md) coordinates the remaining work for a selected external workflow.

## What the current tree provides

| Surface | Current scope | Qualification still required |
| --- | --- | --- |
| Local lifecycle fixture | Callbacks, bounded restart, checkpoint, upgrade/rollback, drain, shutdown, and canonical evidence. The `sandboxed-component` fixture invokes an import-free Wasmtime core-module probe. | A clean public checkout running the documented commands with its exact dependency closure. The fixture is not an installed-component or network deployment. |
| Runtime implementations | Capability-checked effects, deterministic cores, supervised extensions, content/state storage, and Iroh/Redb/Wasmtime adapters. | [Capability reports tied to actual executions](https://github.com/brittonr/aspen/issues/22), per profile and environment. |
| NixOS service module | `nixosModules.molten` configures a bounded local control-service loop and explicit source-trust input. | The corrected full VM run and retained startup, restart, shutdown, and trust-denial evidence; see the [module's validation boundary](nixos-service-module.md#validation-boundary). |
| Cross-host services | Transport and service mechanisms exist with local and process-level tests. | An [authenticated cross-host profile](https://github.com/brittonr/aspen/issues/15) and a [qualified external pilot](https://github.com/brittonr/aspen/issues/31). |
| Source acquisition | All Cargo and Nix sources are revision-pinned and fetched anonymously over HTTPS; VM Cohort ships as an owner-approved exact snapshot and the Mantle producer fixtures are bound by upstream blob identity (`vendor/`). The pinned Octet tool needs the documented [HTTPS rewrite](reproducible-dependencies.md#pinned-tool-transport) for one public SSH-named dependency. | A passing cold build of the full graph on a clean public runner; see [public-build work](https://github.com/brittonr/aspen/issues/23) and [environment diagnosis](handbook/contributing/diagnosing-development-environment-failures.md#cargo-cannot-fetch-a-pinned-git-dependency). |

A local fixture, successful command, replay check, or signed receipt establishes only its declared scope. It does not qualify production isolation, WAN operation, high availability, arbitrary hosted workloads, or exactly-once external effects. Unsupported or uncertain operations keep their documented denial or unresolved outcome.

## Historical pilot

The `v0.1.0` tag and [August 2026 pilot notes](releases/molten-0.1.0.md) refer to the frozen internal pilot at `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`. The original codename, paths, source references, and evidence remain intact. They do not qualify the current branch or announce a new Aspen release.

## Before an external release

A new external candidate is not ready until the following are true for the exact
source revision and dependency closure under review:

- [ ] A fresh checkout builds through the public HTTPS source graph without SSH
  credentials or sibling overrides, and the release dependency profile check
  passes against exact Nix source inputs.
- [ ] The selected NixOS or local deployment path has a positive executed
  workflow; negative startup-denial checks remain evidence of denial, not
  positive startup.
- [ ] VM fault receipts distinguish executed observations from unavailable or
  unexecuted fixtures; caller-authored logs cannot qualify an observed denial.
- [ ] Cross-host claims are backed by actual authenticated peer evidence for the
  selected profile; loopback evidence is labelled as loopback.
- [ ] Capability qualification distinguishes implementation status from recorded
  execution evidence for the exact profile and environment.
- [ ] Third-party notices match the selected dependency revisions, and the
  dependency and license audit records its advisory snapshot and dispositions.
- [ ] [SECURITY.md](../SECURITY.md) names a verified private reporting channel and
  [CONTRIBUTING.md](../CONTRIBUTING.md) matches the current no-PR integration route.
- [ ] Release notes, source acquisition or archive, checksums, supported
  commands, and retained evidence are reviewed before a new tag is published.


## License and notices

Repository-owned source—including cores, contracts, schemas, fixtures, templates, documentation, and policy—is licensed under `AGPL-3.0-or-later`; see [LICENSE](../LICENSE). Third-party dependencies, vendored manifest snapshots, generated material containing upstream code, and external references retain their original terms; see [THIRD_PARTY_LICENSES.md](../THIRD_PARTY_LICENSES.md).

Earlier permissive grants remain valid, including for byte-identical content previously received under those grants. The current declaration applies prospectively to project-owned contributions and does not revoke rights already received. Licensing metadata is distribution evidence only: it does not establish contributor authority, legal compliance, runtime correctness, or release eligibility. AGPL is an accepted project distribution choice; the typed distribution profile records project-required notices and source-export evidence, is not legal advice, and does not prove compliance in every jurisdiction.

The focused [license policy](../license-policy.tsv) covers every current workspace package exactly once, including `molten-node-host`; [the boundary checker](../tools/check-license-boundary.rs) rejects missing package rules before checking license expressions and notice artifacts. Each `notice-revision` rule names a Git dependency; the revision `Cargo.toml` selects for it must appear in that dependency's own `Cargo packages` notice row, so an older revision kept by a historical fixture row neither satisfies nor fails the rule. The `license-boundary` flake check runs the checker and its self-tests. This does not make a private Radicle source obtainable without its documented public seed or a local Radicle profile.
