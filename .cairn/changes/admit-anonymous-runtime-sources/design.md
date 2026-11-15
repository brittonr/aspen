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

## Risks / Trade-offs

- HTTPS availability and full Git-object acquisition are different observations; clean builds remain required.
- VM Cohort's seed returned HTTP 500 and the alternate public endpoint was absent. The extent producer's seed also returned HTTP 500. Preserve those blockers instead of disabling optional resolution or weakening source policy.
- Source trust, current authority and runtime readiness remain separate from acquisition.
