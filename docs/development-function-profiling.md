# Development function profiling

r[impl dev_profiling.dependency]
Aspen uses `flux-profiler` for optional function-level development traces.
The dependency is pinned to commit `2a1916465ae6649aebef3758233cfea98e5d33db`.
The Nix input and Cargo dependency use this same commit.

r[impl dev_profiling.platform]
Profiling is available only for `x86_64-linux` development builds.
Default builds do not enable the optional dependency.
Other targets compile each annotation out through the upstream `disable-profiling` feature.

## Build and capture

Enter the development shell and build the instrumented probe:

```sh
nix develop
cargo build --example profilerprobe --features profiler
```

Run the probe in one terminal:

```sh
cargo run --example profilerprobe --features profiler
```

r[impl dev_profiling.capture]
Attach from a second development shell.
Every documented capture has an explicit time or memory bound:

```sh
flux-profiler --duration 2s --max-mem 64MB --out target/molten-development.fxt
```

Open the `.fxt` file in Perfetto or magic-trace.
Each thread has a separate track.

r[impl dev_profiling.optional_features]
Use `profiler-perf` only when hardware counters are required.
The host must permit `rdpmc` access through its `perf_event_paranoid` policy.
A missing permission is an error, not an omitted counter.

Use `profiler-alloc` only when allocation counts are required.
This feature installs the upstream `CountingAllocator` for that build.
Default builds retain their normal allocator.

r[impl dev_profiling.build_gating]
Use `profiler-disabled` to compile annotated sites to plain function bodies.
This feature is suitable for explicit release-stripping checks.

## Placement boundary

r[impl dev_profiling.placement]
Annotations exist only in the standard runtime shell.
The initial sites cover the CLI vat command, dataspace routing, and live Iroh transfer calls.

`crates/molten-core` and `crates/aspen-core` must not contain profiler dependencies, annotations, shared-memory access, or startup calls.
A shell call site measures a pure-core operation without moving the side effect into that core.

## Claim boundary

r[impl dev_profiling.non_claim]
A trace is one machine-local development observation.
It is not deterministic evidence and does not prove a performance property.

Do not add `.fxt` files or profiler output to these surfaces:

- Valence evidence bundles
- Cairn receipts
- release-readiness inputs
- determinism claims

Use repeatable benchmarks and tests for performance claims.
Profiler observations can only guide local development decisions.


## Optional Rust 1.100 sample-PGO experiment (#32)

The optional experiment is separate from flux-profiler. It never consumes a
`.fxt`, enables a `profiler*` feature, installs an allocator, changes the
default nightly, or replaces the independently pinned Octet producer. Results
are machine-local development observations, not Cairn receipts, deterministic
evidence, Valence evidence, release admission, or universal performance claims.
Canonical harness artifacts retain their original owner and role; diagnostic
JSON/raw-byte digests do not create a new canonical identity scheme.

### Compiler and converter cohort

Stable Rust 1.100 cannot compile the repository's unconditional
`#![feature(register_tool)]` source contract. This experiment therefore names
an explicitly separate **nightly-2026-08-20 / Rust 1.100.0-nightly** variant for
both baseline and sample-PGO builds. It is not a stable-compiler compatibility
claim. No attributes are removed and `RUSTC_BOOTSTRAP` is not used.

The [dated official manifest](https://static.rust-lang.org/dist/2026-08-20/channel-rust-nightly.toml)
identifies compiler commit `f7d782a3be46d6bb4b9792fe69a61db389ba1769`.
`nix/sample-pgo-toolchain.nix` pins the actual rustc, cargo and standard-library
xz SHA-256 values from that manifest. The [compiler's LLVM submodule](https://api.github.com/repos/rust-lang/rust/contents/src/llvm-project?ref=f7d782a3be46d6bb4b9792fe69a61db389ba1769)
identifies `rust-lang/llvm-project` commit
`21cf28432798952d942bacc6bcee3a328faa3638`; the converter is built from that
immutable Git object, not guessed release hashes or ambient LLVM. Both variants
use x86-64/SSE2, optimization level 3, one codegen unit, no LTO, frame pointers,
line tables, ELF build IDs and the explicitly nightly
`-Zdebuginfo-for-profiling` accuracy option. Only the candidate adds
`-Cprofile-sample-use`. Compiler/dependency compatibility is not established
until the real experimental build succeeds.

### Exact root integration (integration owner)

The slice does not edit root wiring, locks, generated plans, default packages,
normal features, or Octet. In the per-system root flake `let`, import:

```nix
samplePgo = import ./nix/sample-pgo-experiment.nix {
  pkgs = unit2nixPkgsBase;
  inherit unit2nix;
  source = ./.;
  extraCrateOverrides = {
    nickel-lang-core = attrs: { };
    verus_prettyplease = attrs: { };
    kamacite-core = attrs: {
      src = kamaciteSource;
      sourceRoot = "source/crates/kamacite-core";
    };
  };
};
```

This reuses exactly the existing workspace-specific overrides; factor/share
that existing attrset if desired rather than diverging its contents. Add the
following **x86_64-linux-only** outputs, without adding them to default checks:

```nix
legacyPackages.sample-pgo = samplePgo;
apps.sample-pgo-experiment = {
  type = "app";
  program = "${samplePgo.app}/bin/molten-sample-pgo-experiment";
};
```

The runner reopens its immutable source flake through
`legacyPackages.x86_64-linux.sample-pgo`, not the mutable checkout. This output
name is part of its builder contract. Include all authored source files in the
flake source before execution. No new Cargo features, dependencies, build-plan
regeneration, flake inputs, or lock changes are needed.

### Host prerequisites and bounded execution

Use an x86-64 Linux host whose PMU exposes **`branches:u` with LBR** to the
invoking user. The host/kernel/virtualization and perf permissions must permit
that exact event and branch-stack recording; VMs frequently cannot. There is no
software-event fallback, global attach, privilege escalation, or sysctl change.
A missing event, permission, real application samples, or matching ELF build ID
blocks the experiment. The Nix daemon, all revision-pinned dependency sources
(including private sources if configured), and disk for two full builds, LLVM
and raw output must be available. Existing dependency/source gates stay on.

The recovery checkout may reside on tmpfs. Its presence is source-authoring
state, not NVMe performance or machine-loss durability evidence. Select and
record the real measurement/output filesystem deliberately; source migration
does not establish any storage-performance or persistence claim.

The source-owned policy is `config/sample-pgo/experiment.json`. Training
launches real native harness clients for 20 seconds, allowing one final bounded
30-second invocation; outer perf capture is bounded at 55 seconds and 128 MiB
per file. Every launched process has an 8 GiB address-space cap, zero core-dump
limit and a 128 MiB per-file cap. Converter and replay calls have finite
timeouts. A realization is bounded to four hours at the client; Nix-daemon
builder resource policy remains the host's responsibility. No timeout is a
valid profile. Training inputs and held-out success/deny/exhaustion/error
inputs have disjoint labels, seeds and payload namespaces. Their exact bytes
and raw BLAKE3/SHA-256 digests are retained before capture. This measures native
harness actor/assert/send/recorded-clock behavior, not distributed service or
steady-state actor throughput.

Capture uses `perf record --no-buildid-cache` to avoid copying the potentially
larger-than-128-MiB baseline ELF into perf's user cache under the same file-size
cap. Build IDs remain recorded in `perf.data`; the actual
`perf buildid-list --with-hits` association must still match the exact baseline
ELF build ID and store path. Conversion uses that store binary directly. This
does not increase any capture, file-size, time or address-space bound.

The candidate's sandbox verifier copies imported raw bytes into its own fresh
validation directory under the same 128 MiB per-file cap and deadline, then
rehashes the copy before offline association and conversion. A store import may
have another build user's UID; this preserves perf's ownership check instead
of forcing past it. Original raw/profile/source bindings remain mandatory.
The complete original association text remains hash-bound. Offline comparison
preserves exact build-ID/path pairs and unidentified real-object paths, rejecting
malformed or conflicting rows. Only an unidentified `/tmp/perf-vdso.so-*` name is
nonbinding: perf materializes a differently named temporary vDSO on every read
of identical raw bytes. An identified vDSO row remains binding.


Baseline and candidate realizations retain Nix result links inside the new
experiment output directory. Keep those links until the experiment finishes;
the tool must not let garbage collection remove an executing or later-measured
baseline after a `--no-link` realization.

After parent integration, run deliberately:

```sh
# Real native consumer tests and experimental baseline compilation:
nix build --impure --no-link --expr \
  '(builtins.getFlake (toString ./.)).legacyPackages.x86_64-linux.sample-pgo.consumerTests'
# One entrypoint builds baseline, captures/converts, builds candidate,
# compares canonical behavior and evaluates held-out measurements:
nix run .#sample-pgo-experiment -- --out /absolute/new/sample-pgo-run
```

The consumer-check expression explicitly permits development checkout evaluation. The experiment
itself uses a store-source flake and never relies on the ambient working tree.
An existing output directory is rejected; failed runs are never overwritten.

### Replay, staleness and measurements

Successful and denied harness reports are replayed in **both** binaries, using
the embedded suite and recorded effect log. The owner compares canonical
observations, receipts, output and final-state refs. Both binaries must also
produce byte-identical owner-rendered canonical artifacts for all four paths.
Suite-step exhaustion and unknown-actor errors fail before a runnable report
exists: compare their actual structured canonical failure artifacts and exit
class; do not invent a post-failure state or claim those failure records are
replayable reports. Any canonical drift makes the experiment ineligible,
regardless of speed.

A fresh profile bundle binds exact source-store identity, Cargo.lock and
unit2nix-plan SHA-256, compiler/converter executable hashes and revisions, common
flags, baseline output/hash/ELF build ID, raw perf bytes and hit association,
converted sample bytes, and policy. The candidate builder rejects any missing
or mismatched binding. It also independently re-reads the raw perf association
and regenerates the sample with the pinned converter before consumption. It
requires at least 100 nonzero application (`molten`-crate) function sample weights. No arbitrary
profile-import/resume path exists in the experiment entrypoint.

After three warmups, 30 paired held-out repetitions alternate baseline/candidate
order. Raw per-invocation high-resolution client wall time, GNU-time user/system
CPU and peak RSS, commands, exit codes, stdout/stderr and canonical artifact
digests are retained; every measured artifact must match the semantic gate.
Binary bytes and Nix-realization wall time are retained. Nix-client CPU/RSS
are explicitly not daemon-builder CPU/RSS; substituter/cache effects remain
visible in retained realization commands/logs. Exited CLI clients do not
provide retained-heap memory, and 30 samples do not justify p99: both are
reported unsupported, not zero or invented. Medians, ranges, inverse-request
wall throughput and raw data describe the observed distribution. Host CPU,
affinity, governor, load, memory, perf policy and cgroup facts are captured
before/after.

The predeclared keep rule requires the paired mean wall-improvement 95%
Student-t interval lower bound (30 pairs, df=29, multiplier 2.04523) to exceed
**5%**, with no path median regression above **2%**, no paired peak-RSS growth
above **5%**, no binary-size growth above **10%**, and all semantic gates. The
interval assumes independent pairs; it is not a universal machine claim. A
regression records `revise`; an upper bound below the threshold records
`stop`; overlap records `inconclusive`. These decisions require actual raw
measurements. Setup/capture/conversion/build failures record `blocked` with
no measured decision. A `keep` recommendation does not enable a profile: a
separate reviewed Cairn change is required for maintained-profile adoption.

### Current status

On 2026-11-13, the real Rust 1.100-nightly baseline built and the 10 existing
native consumer controls passed. A bounded LBR capture/conversion retained 294
application sample weights in `/dev/shm/aspen-sample-pgo-run-nocache-20261113`.
Its first candidate failed perf's foreign-UID check; the repaired verifier then
passed a real `--option sandbox true` Nix derivation against that exact original
record, baseline and converter, without `--force` or sandbox relaxation:

| Repair-control input/output | Exact retained identity |
| --- | --- |
| Imported bundle | `/nix/store/xj41rssnj874jgsig6zcd7fgx76yx9wv-0zk04glcl69ig83lva1d1gh5ms3cfjd1-profile-bundle` |
| Baseline binary | `/nix/store/pjyj70p6szibqg35y876cpg3bqgdh5vf-rust_molten-0.1.0/bin/molten` |
| Baseline ELF build ID | `ebf971c70e8bc0cb098a0da4e1d42e59beb3d008` |
| Converter | `/nix/store/qpcj5lmxsrai5fyrh8w8br21msaxkij8-molten-sample-pgo-llvm-profgen-21cf28432798952d942bacc6bcee3a328faa3638/bin/llvm-profgen` |
| Successful sandbox result | `/nix/store/vkfqifq63y09s2824g3r0lbm83214l3s-aspen-owned-profile-validation-control` |

The result retains `ownership.txt`, `hashes.txt`, `binding.json` and every
validation command/log. During the build the imported record's UID was 65534
and the owned copy's UID was 1000. Both raw SHA-256 values were
`75323826100df338f1cc875d92af830ebf95bb1233cc1eb34c5e13e570a9cd63`;
both converted-profile SHA-256 values were
`68f9f8304906ce85dcb7534332bcc0de44489e12d3beca3aac5ed1f0eac74e25`.
Offline association matched the exact baseline ID/path, and pinned conversion
regenerated the byte-identical 294-weight profile. This is a verifier repair
control, not a candidate build or permission to reuse the profile for changed
source/binaries.

A fresh-checkout baseline dry-run required three local crate builds
(`molten-core`, `molten-node-host`, `molten`); the configured cache endpoint
refused connections. Storage readback showed 2,625,503,232 bytes available on
the store pool and 12,284,141,568 on `/dev/shm`. The old baseline output alone
has a 2,969,899,696-byte NAR (955,154,944 allocated bytes on this compressed
filesystem), before a fresh profile-consuming dependency graph and builder
scratch. NAR size is not an exact physical-space prerequisite. The integration
owner did not admit a large build with this shared-pool reserve; no full fresh
experiment was launched, and no actual ENOSPC or exact future peak is claimed.
The dry-run/storage commands and identities are retained in
`/dev/shm/aspen-sample-pgo-feasibility-20261113.json`. Provision an admitted
store/build reserve for both variants, then use the fresh entrypoint above;
do not reuse the retained repair-control profile. No candidate replay,
held-out comparison or measured keep/revise/stop/inconclusive decision exists.

After the parser repair, the integration owner exercised all four new
association boundary controls successfully (4/4):

```sh
python3 -m unittest discover -s tools/sample-pgo -p test_experiment.py -k test_associations_
```

These cover unstable unidentified vDSO names, exact real build IDs/paths,
preserved unidentified real paths, and malformed/conflicting association
rejection. They do not replace the sandbox proof or establish candidate
performance. The existing 10 consumer controls were not repeated unchanged.

Small diagnostic records are retained under the existing change's
[`evidence/2026-11-13/`](../.cairn/archive/2026-11-15-evaluate-rust-sample-pgo/evidence/2026-11-13/result.json):
the original binding/association, copy and regeneration hashes, build-time UIDs,
all validation commands/logs, original failed-candidate diagnostics, and the
fresh-build feasibility observation. Their result explicitly records candidate
replay and paired measurements as not executed. The original raw/profile bytes
remain only at the exact retained run/bundle locations above; these small
records do not promote a verifier control into canonical replay evidence.


### Completed fresh measurement

The admitted fresh experiment completed with **status `measured`, decision `revise`**. It built both variants from immutable source `/nix/store/ys9rpf8f3lv00fqcjyx4ncn13ypxqxnh-1gp0ya502nxnjpazbycqw13lb7hmfvm5-source`, captured/converted 478 real application sample weights and passed the four-path canonical gate. Success and denied reports replayed in both binaries; exhausted/error outputs were matching preflight failures, not invented replayable reports. The earlier 294-weight repair-control profile was not reused.

After three warmups, the existing policy ran 30 alternating-order pairs on each of four held-out paths. Review of all 240 measured invocations found matching paired exit classes and canonical-artifact BLAKE3 identities and no measured timeout.

| Held-out path | Baseline median wall, ms | Candidate median wall, ms | Median regression |
| --- | ---: | ---: | ---: |
| Success | 66.909 | 116.975 | +74.83% |
| Denied | 34.809 | 66.911 | +92.22% |
| Exhausted | 10.684 | 9.651 | -9.67% |
| Error | 9.677 | 9.887 | +2.16% |

Paired mean wall improvement was **-46.22%**, with the predeclared approximate 95% Student-t interval **[-58.06%, -34.39%]**. The independence assumption is not a universal host/workload guarantee. Maximum paired peak-RSS growth was **8.14%** against the 5% limit; binary size grew from **517,380,840** to **582,102,816 bytes** (**12.51%**, above the 10% limit). The performance and size/RSS guards therefore reject keeping this candidate. Leave it disabled; any revised experiment or maintained-profile proposal requires its own reviewed source and measurements. No default or Octet toolchain changed.

The [measured result](../.cairn/archive/2026-11-15-evaluate-rust-sample-pgo/evidence/2026-11-14/result.json), complete raw-pair record, source/cohort/profile/binary/dataset bindings, semantic comparison, realization records and raw review are retained under the archive's `evidence/2026-11-14/`. Full perf/profile bytes, per-invocation outputs, host facts and Nix result links remain at `/home/brittonr/git/OnixResearch/aspen/target/aspen-sample-pgo-fresh-20261114`. This is machine-local native CLI evidence, not actor-service throughput, retained heap, p99, daemon-builder CPU/RSS, release eligibility or a general performance claim.

