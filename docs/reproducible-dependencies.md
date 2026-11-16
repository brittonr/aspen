# Reproducible release dependencies

Aspen release dependency identity is authored in the typed Nickel profile at
[`config/release-dependencies/profile.ncl`](../config/release-dependencies/profile.ncl).
The profile binds each direct Git dependency to its package identity, reviewed
source coordinate, immutable revision, Nix input, release disposition, and
transport policy. Public GitHub runtime dependencies use immutable HTTPS pins;
that does not qualify every transitive development-tool source. The Artifact packages use one
exact HTTPS Git pin and one non-flake Nix input. No Cargo or Nix source uses
`rad://` transport: Radicle-hosted cohorts are fetched through their seed's
anonymous HTTPS Git endpoint, and the two exact sources without an anonymous
route ship as [reviewed snapshots](#vendored-exact-sources).
Bounded Exec uses the catalog-advertised anonymous GitHub source
`https://github.com/OnixResearch/bounded-exec.git` at unchanged revision
`29dac88ecded94457572db3fdfaaaab95fa91525` and one matching non-flake input.
Other OnixResearch Cargo sources retain their reviewed pins and immutable
archive inputs for sandboxed Nix builds.

## Anonymous acquisition observations

On 2026-11-13 a fresh Git checkout, with an empty HOME, no system/global Git
configuration, no SSH agent, no credential helper and no HTTP authorization
header, acquired that exact Bounded Exec commit from GitHub. Its Git tree is
`fc0b6f75a01088faf4f22be57edc073a3fe45e54`; its metadata-free NAR hash remains
`sha256-BVmqyUYyoNpY6LfOABxwPO3DY88ZtXeKNg3TPoGCcL0=`. The latest mirror head
was not selected. Its selected shell/core packages add no further Git source:
the core is local to the same provider and the Unix shell uses crates.io `nix`.

Bounded Tree remains exclusively identified by
`rad:zqhtZvsteJhxCJE96dMAZSZ9y1PX`; no GitHub fallback is declared. The existing
public Git transport at
`https://seed.radicle.garden/zqhtZvsteJhxCJE96dMAZSZ9y1PX.git` anonymously supplied
selected commit `250f3b25d0933c2fa106aa82d1cc17a496716f06`, not its latest head.
Its Git tree is `0cc4596f8335e25f197971afd4f5e6bbbb49d485` and its metadata-free
NAR hash remains `sha256-ZS8gQ8abQk5Pstm0VuQiGMmOrZUsK3TIJ3SItgwGoEY=`.
The acquired declaration retains that RID, branch `main`, no fallbacks and
AGPL-3.0-or-later. Its selected capability/core packages use only their shared
provider tree and declared registry dependencies. No credentials, sibling
override, provider publication or running Radicle node was needed for these
observed Git-over-HTTPS acquisitions. These observations do not assert a
Radicle signed-ref verification or replace source-trust admission.

Earlier commands and failed bootstrap attempts remain in
[the initial change evidence](../.cairn/changes/admit-anonymous-runtime-sources/evidence/anonymous-acquisition-20261113.json).
An earlier hash-generation pass observed HTTP 530 from Durable File Publication's
`git.onix.computer` route and reused its exact existing NAR identity; that was not
fresh-source availability evidence. A subsequent anonymous GitHub fetch rejected
the selected commit rather than supplying it. The provider's canonical RID
`rad:z3tAR4For7qw8ZirkJzoDw1VNDDLM` then supplied that exact commit through
`https://seed.radicle.garden/z3tAR4For7qw8ZirkJzoDw1VNDDLM.git`. Selected revision
`8e05e74e24b45f752d77145c4455385daaf6d6ab`, Git tree
`1c186b996dd3f85102b22035c57d133bf880fa76`, NAR
`sha256-kd74vui4byAC32OnH7Wn1sw9IW4PWjSw/VThvdnd380=` and the selected
`MIT OR Apache-2.0` grant are unchanged. Cargo, Nix, source hashes and both unit
graphs now select that actual public coordinate; no newer implementation or
license grant was substituted.

[Complete source-closure observations](../.cairn/changes/admit-anonymous-runtime-sources/evidence/anonymous-source-closure-20261113.json)
retain exact acquisition commands, outputs, revisions and measured NAR hashes
for all 21 Git source cohorts in the selected default/development unit graph.
They also enumerate all 78 fixed Nix source byte cohorts: 76 were acquired
anonymously and matched their locked NARs. Recursive submodules and required LFS
payloads are included, not pointer stubs. The executable-extent verifier's nested
Cairn Artifact input now uses verified public HTTPS at unchanged commit
`e41340bec587b6d049b5cc518ec7db925dde84be` and unchanged NAR
`sha256-2cM912L2YXVnVX9LquwhAPKyjPP/z/oFQRe7Qq9bHHE=`, instead of hidden SSH
credentials. Frozen provider contents and verifier implementation pins are
unchanged.

## Vendored exact sources

VM Cohort `31f1696ba9391bfda8577a58af84f72361d5573e` and the Mantle extent
producer `2c636b1b25353a1b0befa5af48dc68615cd686dd` were unavailable through every
attempted anonymous path: their seed requests returned HTTP 500, and the
advertised Mantle GitHub repository rejected the exact producer commit. Neither
revision changes:

- [`vendor/vm-cohort/`](../vendor/vm-cohort/) ships an owner-approved,
  source-only selection of the VM Cohort commit: the `vm-cohort-core` crate, the
  workspace manifests, license and notice files, the README, and the archived
  ownership record read by the world-snapshot identity check. Git history and all
  other repository material stay unpublished. `vm-cohort-core` is an optional path
  dependency outside the Aspen workspace; `crates/vm-cohort-core/Cargo.toml`
  resolves the upstream workspace inheritance as `cargo vendor` would, and the
  unchanged upstream manifest is kept beside it as `Cargo.toml.orig`.
- Aspen's two executable-extent producer fixtures are bound to the exact Mantle
  files by their upstream Git blob identities. No additional Mantle bytes are
  published.

[`vendor/sources.ncl`](../vendor/sources.ncl) is the reviewed selection policy.
`scripts/vendored-sources.py import --source vm-cohort=CHECKOUT
--fixture-repository rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb=CHECKOUT` regenerates
[`vendor/sources.lock.json`](../vendor/sources.lock.json) from explicit local
checkouts at the exact revisions. `scripts/vendored-sources.py check`, also run by
the `vendored-sources` flake check, verifies every file's size, mode, upstream Git
blob and BLAKE3 identity; rejects extra, missing, symlinked or undeclared files;
and requires a `THIRD_PARTY_LICENSES.md` row naming each snapshot path at its
exact revision. A snapshot is source identity only: it does not qualify the
optional feature, runtime behavior, current authority, legal distribution, or
release eligibility.

## Pinned tool transport

The pinned Octet producer `3f7abb641ee604f54629da415bc7686f89662065`, which
supplies `cargo-octet` to the default development shell and several flake checks,
declares Bounded Exec `44eb74eb639194665ee820562d5b2ce2c3ff6a94` by an
`ssh://git@github.com/` URL inside its own Cargo manifest. Aspen cannot override
that producer-internal source, but the repository and commit are public. Builds
therefore set an environment-scoped Git URL rewrite:

```sh
export GIT_CONFIG_COUNT=1 \
  GIT_CONFIG_KEY_0=url.https://github.com/.insteadOf \
  GIT_CONFIG_VALUE_0=ssh://git@github.com/
```

The hosted `Reproducible dependencies` workflow sets the same three variables,
does not persist checkout credentials, and runs `nix develop --command cargo build --locked`
before `nix flake check`. With a fresh empty home (no SSH keys, agent, Git configuration or Nix
fetcher cache), evaluating the default development shell failed at exactly that
fetch without the rewrite and succeeded with it, fetching the same commit over
HTTPS ([observation](../.cairn/changes/admit-anonymous-runtime-sources/evidence/anonymous-octet-transport-20261115.json)).
This is transport only: the Octet revision, its source bytes and its lint behavior
are unchanged. It is not a cold build; the hosted clean-runner result is the build
evidence. Remove the rewrite once Octet publishes HTTPS or vendored coordinates for
this dependency and Aspen adopts that release; Octet's published public release
candidate `4aba4019f77f95cb9c3e25ff27964898d708cef0` already vendors it but diverges
substantially from the current pin, so adopting it is a separate producer-cohort
review.

Hosted storage [run 43693265859](https://github.com/brittonr/aspen/actions/runs/43693265859) and full-flake [run 43693265823](https://github.com/brittonr/aspen/actions/runs/43693265823) at `2c4631bc88537d6d6323f738e8374683d7706cb4` failed before conformance. The pinned Octet CLI `3f7abb641ee604f54629da415bc7686f89662065` selects a separate Bounded Exec `44eb74eb639194665ee820562d5b2ce2c3ff6a94` through SSH in its Cargo manifest. The fixed Nix-input inventory above does not include this producer's internal Cargo sources. Full-flake evaluation also retained the unresolved VM Cohort and Mantle routes.

The storage workflow selects `nix develop .#object-storage-ci`: the unchanged pinned Rust compiler plus compiler/linker support, Git and Python, without unrelated proof-producer packages. Its dependency guard, tests, optional feature and canonical receipt requirements are unchanged. This is an explicit storage-profile boundary, not a source rewrite, a new Octet cohort or a qualification of the unchanged default development shell. The [retained hosted observation](../.cairn/changes/admit-anonymous-runtime-sources/evidence/hosted-ci-20261115.json) keeps failure and artifact evidence separate from any subsequent pass. Aspen's workflow maintainers own this shell and repeatability through the existing hosted workflow; issue #23 remains open for the full public graph.

The repaired hosted [storage run 43695373650](https://github.com/brittonr/aspen/actions/runs/43695373650) at `4055bab940178c300c66b635cd00f2cda0188cf3` initialized that shell and passed its dependency guard. Before the first Rust test, Cargo attempted the optional `vm-cohort-core` source and failed because `git-remote-rad` was absent. Optional feature selection did not prevent this cold metadata fetch. A [fresh credential-free direct attempt](../.cairn/changes/admit-anonymous-runtime-sources/evidence/anonymous-vm-source-20261115.json) to acquire the unchanged VM Cohort commit through its recorded seed returned HTTP 500; the workspace catalog advertises no GitHub source. The all-required-default-acquisition task is reopened. No test or canonical conformance receipt was produced, and neither source-acquisition counts nor the tool-shell fix satisfy the clean-public-build criterion.

The later full-flake [run 43816346417](https://github.com/brittonr/aspen/actions/runs/43816346417)
at `d2c4359d79b287514f054a799a7edd576b90f351` acquired the public source graph and
reached `molten` production/test crate builds before reporting “The operation was canceled.”
The [retained observation](../.cairn/changes/admit-anonymous-runtime-sources/evidence/hosted-ci-20261116/observations.json)
binds that result and the original diagnostic log digest. It establishes neither the cancellation
cause nor a successful cold build, passing tests, or a passing full flake check; #23 remains open.

At `aac7b5ba974cc9660749dab6d8ef4b7db04e3cb4`, hosted
[run 43925658138](https://github.com/brittonr/aspen/actions/runs/43925658138)
completed both the locked Cargo build and default Nix build. Full flake checks
then failed fetching the frozen verifier pilot's GitHub tree API response with
HTTP 403, so the README workflow step was skipped. The exact, already-hashed
tree artifact now ships with the [frozen pilot](verified-node-replication-pilot.md#exact-inputs)
and is restored through its checked import script; source/verifier revisions,
hashes and expected decisions are unchanged. This removes that live API fetch,
but successful full checks and the verbatim workflow still require a new run.


Schema `molten.release-dependency-profile.v2` adds `registry_dependencies`
rows for direct crates.io pins. Each row binds the manifest dependency, package
name, exact version, the crates.io index source, the Cargo.lock SHA-256
checksum, transport `crates.io`, a dependency plane, and a release disposition.
The validator requires an exact `=<version>` manifest requirement, a matching
`optional` flag for `optional-runtime` rows, and lock version, source, and
checksum equality. It classifies each pin onto its plane edge: `implementation`
rows must sit in the root `[dependencies]` table and `development` rows in
`[dev-dependencies]`, and a `development` plane requires the `development`
disposition. `marble` 16.0.2 and `art` 1.0.0 are optional implementation pins
behind the `marble-object-store` feature. A registry row binds the version, the
checksum, and the manifest plane. It does not review upstream code.

The `molten-release-policy` shell reads the Nickel export, `Cargo.toml`,
`Cargo.lock`, `flake.lock`, configured archive evidence, and distribution
artifacts. It passes normalized in-memory observations to the pure
`molten_core::release_dependency` validator. The core sorts diagnostics and
canonical report material, so input ordering cannot change the result or its
BLAKE3 identity. Filesystem reads, Nickel execution, hashing of supplied
archive files, and diagnostic rendering remain in the shell.

Run the focused check from the repository root:

```sh
nix develop -c cargo run -p molten-release-policy -- \
  --root . \
  --evidence-source valence-integrity=/path/to/exact-valence \
  --evidence-source octet-cutover=/path/to/exact-octet \
  --evidence-source hardened-wasmtime-resumable=/path/to/exact-hardened-wasmtime \
  --evidence-source mantle-wasm-consumer-verifier=/path/to/exact-mantle
```

Each evidence root must contain the exact archive bytes selected in the current
profile. An arbitrary current sibling checkout is not a substitute for those
immutable provider inputs; the dependency-audit runner retains the guard's
actual status and diagnostic if any required root is absent or mismatched.

`nix flake check` runs the same validator against exact Nix source inputs. Local
`--override-input` paths are development conveniences only and cannot replace
reviewed release identities.

## Canonical Valence migration

Molten consumes `valence-core` from standalone Valence revision
`9201074343cd9da0ffaf6ec9db690af5b445d663`. This retains the standalone
identity boundary established by Octet's completed cutover and Valence's archived
`harden-preserves-integrity-boundaries` change. It also admits actual Octet
policy-check records as recorded provenance, without admitting them as proof
cards, promoting their evidence tier, or accepting unknown tiers. The profile binds
the completed compatibility archive and its actual CLI verification evidence,
alongside the earlier boundary evidence:

- the Valence archive task bytes;
- Octet's archived `complete-standalone-valence-cutover` task bytes;
- Octet's generated cutover manifest and Aspen migration target; and
- the exact Nix revisions used to materialize both evidence sources.

The resolved Cargo graph must contain exactly one `valence-core` source
identity. Octet remains a separately pinned proof-tool provider; it no longer
provides Molten's canonical Valence semantics.

Molten and Valence share Artifact revision
`5f3962d7471d1547e044f57daeda72f56621fd52` through the single non-flake
`artifact-src` input. The release gate rejects duplicate direct/transitive
package source identities. The selected package directories retain identical
existing files and feature sets; the newer source adds package license files
and prospectively declares AGPL-3.0-or-later while preserving earlier grants.
Historical source-migration receipts remain scoped to their original cohort.

## Updating pins

1. Review the upstream revision and its license/security implications.
2. Update the Nickel profile and the matching Cargo manifest revision.
3. Update the matching `flake.nix` input.
4. Run Cargo to regenerate `Cargo.lock`.
5. Run `nix flake lock` or a targeted Nix lock command to regenerate
   `flake.lock`; never edit it manually.
6. Run `scripts/git-source-hashes.sh --write` to record the metadata-free NAR hash of
   every Cargo git source in `crate-hashes.json` under revision-qualified keys. Then
   regenerate both plans with the repository's pinned unit2nix tool:
   `unit2nix --workspace -o build-plan.json` and
   `unit2nix -p molten-release-policy --bin molten-release-policy -o release-policy-build-plan.json`.
   Do not let unit2nix prefetch a git source itself: its prefetch hashes a checkout that
   still contains `.git`, while `pkgs.fetchgit` removes `.git`, so such a hash fails on
   any builder that lacks the prefetched store path. `scripts/git-source-hashes.sh --check`
   and the `git-source-hash-binding` flake check reject unbound or drifted hashes.
7. Run positive and negative focused checks, then `nix flake check`.

Floating branches, tags without immutable commits, unreviewed SSH-only release
sources, manifest/lock/Nix drift, duplicate canonical package identity, stale archive
bytes, and missing configured distribution evidence fail closed.

## AGPL distribution profile

`AGPL-3.0-or-later` is an accepted Molten project choice. The profile records
an immutable project source coordinate/revision, notices, and project-required
source-export artifacts. These records are project-policy evidence only. They
are not legal advice and do not prove compliance in every jurisdiction,
corresponding-source completeness for an unreviewed distribution, upstream
correctness, runtime correctness, or release eligibility.
