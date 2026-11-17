## Context

The selected provider revisions are immutable, but Cargo and Nix still select Radicle transport for public runtime sources. Anonymous HTTPS probes advertised executable-extent `025d9636f0161777710dac37b3c210ca0ad9483f` and hardened runtime `57c834dfd0da2f092e2cbbd0e48bb60b61e2d06e` without an SSH agent or global Git configuration.

## Decisions

### Decision: Change transport, not implementation cohort

**Choice:** Select the published seed HTTPS Git coordinate for those two providers, use Choregraph's existing public GitHub coordinate consistently, and regenerate every derived dependency artifact.

**Rationale:** The exact commits remain fixed. A successful advertisement proves only anonymous source availability at that endpoint; only subsequent lock/hash/build checks establish the selected consumer closure. No local sibling override becomes release evidence.

### Decision: Admit the exact anonymous Bounded Exec mirror, not its latest head

The catalog-advertised `https://github.com/OnixResearch/bounded-exec.git` anonymously supplied selected commit `29dac88ecded94457572db3fdfaaaab95fa91525` in a fresh checkout with no global/system Git configuration, helper, HTTP authorization header or SSH agent. Its Git tree is `fc0b6f75a01088faf4f22be57edc073a3fe45e54`; the metadata-free NAR hash remains `sha256-BVmqyUYyoNpY6LfOABxwPO3DY88ZtXeKNg3TPoGCcL0=`. Change the coordinate consistently through owning generators, preserve all revisions and unrelated source/bin changes, and retain acquisition observations separately from build/source-gate qualification. Bounded Tree has no catalog GitHub route; discover and try actual advertised public Radicle bootstrap paths before declaring a concrete publication prerequisite.

On 2026-11-13 the existing Bounded Tree seed anonymously served exact selected commit `250f3b25d0933c2fa106aa82d1cc17a496716f06` and Radicle identity ref objects. Git tree `0cc4596f8335e25f197971afd4f5e6bbbb49d485` and metadata-free NAR `sha256-ZS8gQ8abQk5Pstm0VuQiGMmOrZUsK3TIJ3SItgwGoEY=` match the existing selected source. No Bounded Tree coordinate, revision or source fallback changes are required. Direct Git acquisition does not establish Radicle signed-ref verification; preserve failed inspection/bootstrap observations without turning them into success. The unchanged Durable File Publication route separately returned HTTP 530 during the owning hash generator; its existing exact Nix lock identity was reused, not promoted to anonymous-availability evidence.



### Decision: Resolve the required default source graph, not only its first blocker

The catalog-advertised Durable File Publication GitHub repository rejected exact selected commit `8e05e74e24b45f752d77145c4455385daaf6d6ab` with `upload-pack: not our ref`; selecting its latest published head is prohibited. Its provider-declared canonical RID `rad:z3tAR4For7qw8ZirkJzoDw1VNDDLM` anonymously supplied that exact commit through `https://seed.radicle.garden/z3tAR4For7qw8ZirkJzoDw1VNDDLM.git`. Git tree `1c186b996dd3f85102b22035c57d133bf880fa76` and NAR `sha256-kd74vui4byAC32OnH7Wn1sw9IW4PWjSw/VThvdnd380=` are unchanged, as is the selected `MIT OR Apache-2.0` grant. Migrate the failed `git.onix.computer` coordinate to that actual public RID transport through all owning generators. Do not manufacture the missing GitHub commit or repin to newer AGPL source.

Fresh anonymous acquisitions cover all 21 distinct Git source cohorts in the current merged default/development unit graph, plus the six primary default Nix tool inputs. Each requested exact revision and metadata-free NAR is compared with its selected binding. This is a conservative acquisition observation for the documented default profile, not optional-feature qualification, a source-policy check or a clean build. Preserve earlier failed observations as historical evidence and append the resolved default-graph outcome separately.

### Decision: Cover fixed Nix inputs and remove the remaining nested SSH transport

Enumerate every distinct fixed Nix input by its selected source NAR, retaining all node aliases. Acquire official GitHub revision archives or the declared Git source in fresh anonymous environments; include recursive submodules and LFS payloads where the lock requires them. Compare actual source bytes to the unchanged locked NAR, not merely an advertised ref. Preserve duplicate raw observations and missing-source outcomes; deduplicating source inventory must not turn a hidden SSH coordinate into a public-build claim.

The executable-extent verifier's pinned Cairn input selects Artifact `e41340bec587b6d049b5cc518ec7db925dde84be` through SSH, while the current Octet toolchain already overrides the same exact source to public GitHub. A fresh anonymous HTTPS Git checkout acquired that exact commit and matched `sha256-2cM912L2YXVnVX9LquwhAPKyjPP/z/oFQRe7Qq9bHHE=`. Add an equivalent root-owned nested input override and update only its lock transport. Do not modify frozen provider contents, advance its implementation cohort, or treat source acquisition as verifier execution. VM Cohort and the Mantle extent producer remain explicit unavailable exact sources.

The retained bounded closure observation records all 21 selected default/development Git source cohorts and all 78 distinct fixed Nix source byte cohorts. Actual anonymous acquisitions match all 21 selected Git cohorts and 76 fixed Nix cohorts. VM Cohort `31f1696ba9391bfda8577a58af84f72361d5573e` and Mantle producer `2c636b1b25353a1b0befa5af48dc68615cd686dd` remain unavailable at the attempted public paths; neither is replaced, repinned or marked qualified. All raw duplicate observations remain retained, while summary coverage counts distinct locked source NARs. The updated Artifact lock retains its exact original NAR.

### Decision: Qualify storage with its declared tools, not unrelated proof producers

Hosted storage run `43693265859` at `2c4631bc88537d6d6323f738e8374683d7706cb4` failed during `nix develop`, before executing its dependency guard or Rust tests. Pinned Octet `3f7abb641ee604f54629da415bc7686f89662065` declares Bounded Exec `44eb74eb639194665ee820562d5b2ce2c3ff6a94` through SSH in `cargo-octet/Cargo.toml`; this is not Aspen's separately pinned runtime dependency. The uploaded artifact contains endpoint diagnostics, not canonical conformance receipts.

The storage job does not execute Octet. Add an explicit `object-storage-ci` shell containing the existing pinned Rust toolchain, compiler/linker support, Git and Python. Select that shell for the existing job without removing or changing its dependency guard, tests, features, endpoint, or required canonical receipt files. Do not patch the frozen producer, change its revision, add credentials or redirect Git globally. The default development shell remains unchanged.

A passing actual hosted run and its uploaded canonical receipts qualify only this storage profile. They do not satisfy the default development shell, full-flake, cold-cache or optional-source acceptance criteria. Preserve the failed runs and keep the source-admission change active while the producer-owned default-tool route and unavailable VM Cohort/Mantle sources remain unresolved.

The repaired hosted run `43695373650` initialized that shell and passed its guard, then cold Cargo resolution attempted optional VM Cohort `31f1696ba9391bfda8577a58af84f72361d5573e` before running any Rust test. A fresh anonymous direct fetch from the recorded public seed returned HTTP 500. Optional feature selection is therefore not a source-availability boundary for this Cargo invocation. Reopen all-required-default-acquisition acceptance; do not remove the dependency, substitute another revision, prefill a private cache, or describe the endpoint-log-only artifact as conformance.

### Decision: Ship owner-approved exact snapshots for the two unavailable sources

No anonymous route serves VM Cohort `31f1696ba9391bfda8577a58af84f72361d5573e` or Mantle producer `2c636b1b25353a1b0befa5af48dc68615cd686dd`, and both repositories stay private. On 2026-11-15 the owner approved publishing exactly the build-required files, not the repositories or their history:

- VM Cohort: the `vm-cohort-core` crate and tests, both workspace manifests, `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `README.md`, and the archived ownership record the world-snapshot identity check reads (23 files). They ship unchanged in `vendor/vm-cohort/`. Cargo cannot build the crate's workspace-inheriting manifest outside its upstream workspace, so `crates/vm-cohort-core/Cargo.toml` is the already-reviewed inheritance-resolved manifest (previously `checks/world-snapshot-octet/vm-cohort-core.Cargo.toml`), and the unchanged upstream manifest is kept as `Cargo.toml.orig`, as `cargo vendor` does. `vm-cohort-core` becomes an optional path dependency excluded from the Aspen workspace; the lock change is exactly the removed `rad://` source line.
- Mantle: Aspen's two executable-extent fixtures already equal the producer's files. They are bound by upstream Git blob identity instead of a producer checkout, so no Mantle bytes beyond those already in Aspen are published.

`vendor/sources.ncl` is the reviewed policy; `scripts/vendored-sources.py import` regenerates `vendor/sources.lock.json` from explicit checkouts at the exact revisions, and `check` (the `vendored-sources` flake check) verifies size, mode, upstream Git blob and BLAKE3 identity of every file, rejects extra, missing, symlinked and undeclared files, and requires a license-notice row naming each path at its revision. Both `git+rad` flake inputs, the VM `rad://` Cargo source, its crate hash and its Git-transport profile row are removed through their owning generators. Revisions, implementation bytes and checks are unchanged; the snapshot is source identity only, not feature, runtime, authority or release qualification.

### Decision: Fetch the pinned Octet producer's public SSH-named dependency over HTTPS

Octet `3f7abb641ee604f54629da415bc7686f89662065` supplies `cargo-octet` to the default shell and several checks, and declares Bounded Exec `44eb74eb639194665ee820562d5b2ce2c3ff6a94` by an `ssh://git@github.com/` URL inside its own Cargo manifest; its vendoring hook does not reach that source and Aspen cannot override it. Octet's published public release candidate `4aba4019f77f95cb9c3e25ff27964898d708cef0` vendors the dependency but diverges by 1,289/677 commits, so adopting it would replace the source-gate producer and is a separate review.

The repository and commit are public. Builds set an environment-scoped Git rewrite (`GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=url.https://github.com/.insteadOf`, `GIT_CONFIG_VALUE_0=ssh://git@github.com/`), documented in the README and the dependency policy and set per job in the hosted `Reproducible dependencies` workflow. It touches no global configuration or `git@github.com:` remote. In a fresh empty home without SSH, Git configuration or fetcher cache, evaluating the default development shell failed at exactly that fetch without the rewrite and succeeded with it over HTTPS (`evidence/anonymous-octet-transport-20261115.json`). Producer revision, source bytes and lint behavior are unchanged. Remove the rewrite when Octet publishes HTTPS or vendored coordinates and Aspen adopts that release.

### Decision: Retain successful storage observations separately from full-flake acceptance

Hosted storage runs `43810362842` (`fcbf60a3d`) and `43813756730` (`f663d2f47`) succeeded. The latter's 47 passing tests include the live local-S3 command/cache arms and mock fault controls. Retain its 14 original packed Preserves receipts, their BLAKE3 identities, both artifact records, an explicitly normalized test excerpt and decoded diagnostic views under `evidence/hosted-ci-20261116/`. The simulated put receipt already binds canonical fixture-command refs and each observed terminal. Thirteen mock/fault receipts match between runs; live conformance keeps run-specific latency observations.

This satisfies the storage CI observation only. The default `Reproducible dependencies` workflow still needs its own passing retained result; the tool-scoped storage shell, public substitutes and local endpoint do not establish a full cold-source build, production durability, performance benefit or release eligibility. Preserve all earlier failures unchanged.

### Decision: Execute the public commands, not only their packaged substitute

The clean hosted workflow explicitly runs the locked Cargo build, default `nix build`, full flake checks and the README fixture and cleanup blocks unchanged through `scripts/readme-quickstart.py --cargo`. Cargo and its README execution share one clean runner; the default Nix build and full flake checks share a separate clean runner. The required `flake-check` aggregate succeeds only when both jobs succeed. The README mode runs in the checkout; its own `mktemp` creates the isolated output. The existing `--molten` mode remains the sandboxed packaged-binary smoke in a temporary working directory. Its result cannot substitute for the verbatim Cargo or cold-source result. Retain actual hosted outcomes before claiming either #23 or #25's clean-runner acceptance.

### Decision: Preserve the frozen tree artifact without a live API dependency

Hosted run `43925658138` at `aac7b5ba974cc9660749dab6d8ef4b7db04e3cb4` completed the locked Cargo build and default Nix build, then full flake checks failed fetching the frozen verified-node-replication tree API response with HTTP 403. The README workflow step did not execute. Preserve those separate outcomes; the response does not establish the HTTP denial's cause.

The previously admitted 29,256-byte tree response for upstream `eb8e27244a4f1b7714defe55aa495c6173a1c330` already has an exact fixed-output `tree_api_hash`. Ship precisely those bytes in the pilot profile directory through `scripts/import-node-replication-tree.py`, which checks the unchanged hash before replacing the artifact. Nix consumes the local file under that same hash and retains its actual Verus-gitlink validation. No runtime source, source hash, frozen profile, verifier identity, expected decision, lock or build plan is changed; anonymous default checks no longer need a live GitHub tree API response. Only a new actual hosted result can complete public-check acceptance.

### Decision: Separate independent acceptance paths without weakening the required gate

At `afc8308bff9a8c50b2315cb4a17c9454252d7c45`, hosted run `43946811080` passed the locked Cargo build and default Nix build. The full-flake step was canceled after the serial job had run for 90 minutes; the README step was skipped. The retained metadata and original log establish neither the cancellation cause nor a failing check predicate.

Run Cargo plus the unchanged README commands in `cargo-workflow`, and default Nix build plus full flake checks in `nix-acceptance`. Both use fresh checkouts without persisted credentials and the same environment-scoped anonymous transport rewrite. Keep the existing required status name `flake-check` as an always-evaluated aggregate that fails for either non-success result, including failure, cancellation or skipped execution. No command, test, timeout, source pin or expected verdict is removed or relaxed. This removes the serial dependency between independent paths and preserves Nix build reuse within its job; only actual hosted execution can establish the new outcome. Aspen workflow maintainers own the split and its repeatability evidence.

### Decision: Declare bounded Nix scheduling for hosted acceptance

Run `43965597007` at `ace29e598a74caa14f2ca338f9ffc3ce068d938d` passed the locked Cargo build, unchanged README Cargo fixture/cleanup and default Nix build. The Nix runner lost communication during full checks; its job log was unavailable, and the aggregate correctly failed. Preserve the actual README stdout, provider metadata/annotation and unsuccessful log retrieval separately. This establishes no CPU, memory, disk, network or runner-loss root cause.

The next `nix-acceptance` job declares `NIX_CONFIG` scheduling with `max-jobs = 1` and `cores = 2`, reducing concurrent build work on the same hosted runner class. These are Nix build-scheduling requests, not hard process isolation or a diagnosed fix. Keep every command, source/verifier cohort, compiler profile, expected decision and test/operation deadline unchanged. Actual new clean-runner full-check success is still required; the successful Cargo/README job cannot mask Nix failure.

### Decision: Archive only the observed complete public candidate

Run `43986122156` at `1e81020e309c46be6251db36021df25577ad6676` passed both clean-checkout acceptance jobs and their required aggregate. Locked Cargo, the unchanged README Cargo fixture/cleanup, default Nix build and full flake checks all succeeded. Retain the exact job/step metadata, durations, original diagnostic digests and normalized stdout separately from earlier failures. The result establishes this default candidate on the declared hosted profile, not the cause of earlier runner loss, optional-feature qualification, independent trust or release admission.

Publish the three historical acquisition JSON files as explicitly marked host-material-redacted views. Preserve their exact originals privately and bind each view with `original_record_blake3`; do not rewrite immutable revisions, NAR identities, recorded outcomes, historical snapshot bindings or packed canonical receipts. Remove their old publication-baseline exceptions instead of relocating or expanding exemptions. Workflow maintainers own the existing hosted commands and their repeatability for later candidates.

## Risks / Trade-offs

- HTTPS availability and full Git-object acquisition are different observations; clean builds remain required.
- VM Cohort's seed returned HTTP 500 and the alternate public endpoint was absent. The extent producer's seed also returned HTTP 500. Instead of disabling optional resolution or weakening source policy, the owner approved the exact snapshots above; a future source refresh must repeat the import and review.
- Source trust, current authority and runtime readiness remain separate from acquisition.
