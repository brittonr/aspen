# Aspen original CPU batch renderer

Original AGPL-3.0-or-later implementation for Aspen issue #37; no Hatchet/rpt/Fidget renderer code or fixtures were copied. Scene interpretation stays outside progress infrastructure. Existing batch admission, retries, reconciliation and accepted output records remain authoritative. Existing `molten-*` crate and command names retain Aspen's internal Molten codename; they are not a separate public product.

## Evidence boundary

The Nix-supported release build and demo executed successfully on 2026-11-14, using the advertised pure pilot shell, pinned `wasm32-unknown-unknown` toolchain, daemon store/local builder, warm Cargo target, two Cargo jobs, incremental disabled and unchanged release settings. This is cached same-host x86_64 Linux evidence, **not** clean public bootstrap, release, two-host/LAN, machine-loss, cross-architecture or native/Wasm equality qualification. Public-shareable local artifacts are retained at `/dev/shm/aspen-render-demo-20261114`: exact invocation/environment, build/demo logs, source archives, prepared manifest/inputs, statuses/progress, duplicate/repeat/restart observations, accepted envelope, PPM/PNG preview, `verification.json` and `metrics.json`; node roots/keys/databases remain private. Build and demo exited zero (565s and 19s external whole-second wall observations). All 32 tiles plus assembly succeeded; duplicate submission reused the run, replayed progress applied once, completed controller restart preserved outputs/tasks/watermark, and a fresh run reproduced all 33 accepted output identities. Accepted final content is `aaf4b7d0977e3e222e3ed178d5cca2e9b2691210ee56a440a423d138ccbc4bd4`; raw RGB BLAKE3 is `30c0c2bd015720d4468305f90452fc1673958fcce7bdf610958ed30a0c3e3a35`. Whole-second preparation/execution-plus-scheduling-transfer-assembly/download/local-verification measurements were 0/7/0/0; zero means below this precision, not instantaneous. Scheduling, tile transfer, assembly-only time, retry cost and peak memory remain explicitly unmeasured/null. The inspected 640x360 image shows blue, red-brown and green spheres over a gray ground under a pale sky, with expected sparse-sample noise and no visible tile seams.

Build-source archive BLAKE3: `21487020b2599de092abe538847f16914598b9730a56bb6c167a397ee93372f2`; demo-source archive BLAKE3: `d9f2f0f0c39f70b8d633815d4260f4025411d693d309d559e0c12dd056b43965`. `source-comparison.json` records that only `pilots/orchestration/docs/PROGRESS.md` changed between those retained archives; executable source payloads were identical. These are historical observations of that source cohort, not evidence for any subsequent runtime repair. Earlier implementation-agent status was authored/not executed; that history and the earlier WASI-import rejection below are not converted into passing evidence. No fault campaign, general test suite, formatting/lint gate or physical-host experiment is established by this demo. `scripts/render-faults.sh` separately authors/runs component-dependent renderer cases; an ordinary Cargo test run does not execute those external-artifact cases.

After the scoped-status RPC v2 repair settled, one further warm Nix-supported release build and demo succeeded on 2026-11-14 (exit zero; 58s build and 56s demo external whole-second wall observations). This separate cohort is retained at `/dev/shm/aspen-render-demo-rpc-v2-20261114`, with the same explicit loopback ports 46640/46641/46642 and unchanged scene/guest/policy/budgets. Its source was archived **before** building; build and demo source archives have identical BLAKE3 `fecf97428b93511ced1910be801cc3d1c954cace2d453a9169a15d4db34cedd7`, with no file-payload differences. Exact release binaries/component, commands/environment, logs, prepared inputs/manifest, envelope/image and all observations are retained. CLI BLAKE3 changed to `0cabdd83a6a34b534d175e58de7e03348e5d584c221cc8bbcea3852e5a75d053`; renderer/component binary identities stayed unchanged. Duplicate same-run, apply-once progress replay, completed controller restart and fresh repeat all passed for all 33 tasks. `pre-repair-comparison.json` records equality of every accepted output identity, manifest/component/final/RGB identities, exact envelope bytes and exact PPM bytes against the preserved earlier cohort. Whole-second preparation/execution-plus-scheduling-transfer-assembly/download/local-verification measurements were 0/23/0/1; scheduling, tile transfer, assembly-only time, retry cost and peak memory remain null/unmeasured. The inspected lossless preview has the same three noisy spheres and no visible tile seams. This adds only cached same-host rendering evidence for the repaired cohort; all qualification limits above still apply, and earlier attempts/history remain intact.

A parent-run follow-on under the same RPC v2 release profile is retained separately at `/dev/shm/aspen-render-demo-rpcv2-20261114`. Its build and demo archives both have BLAKE3 `056886d7fae7a1ffd730eb2c162b7ae8aefe74484dea222f8d7438b334d0c907`; the exercised CLI has BLAKE3 `0cabdd83a6a34b534d175e58de7e03348e5d584c221cc8bbcea3852e5a75d053`, with v2 present and no v1 ALPN fallback. All 33 accepted output identities, the manifest, final envelope and raw RGB matched the original cohort; the verified PPM bytes were identical. Its existing whole-second metrics recorded preparation 0, combined execution/scheduling/transfer/assembly 5, download 0 and verification 0; separate scheduling, tile-transfer, assembly, retry and peak-memory values remain unavailable. These observations are not a controlled timing comparison.

## Entrypoints

From the repository root, after the reviewed dependency locks have been generated:

```sh
nix-shell --pure pilots/orchestration/render/shell.nix \
  --run 'bash pilots/orchestration/scripts/render-demo.sh'
```

The pilot-only shell consumes repository `flake.lock` public nixpkgs/rust-overlay pins and root Rust channel, providing the prebuilt `wasm32-unknown-unknown` standard library, wasm-tools, Python and C build tools. The intended import-free core module is wrapped with `wasm-tools component new`; the build script then validates the component and rejects exposed imports without stripping them or waiving admission. A component-producing target is not interchangeable: the observed `wasm32-wasip2` output imported WASI and was rejected as outside this unchanged profile. No ambient rustup target installation or root hosted-Wasm cohort change is performed. Paths are checkout-root aware.

Both standalone pilot and `examples/render-guest/Cargo.lock` dependency locks were present and accepted by the actual `--locked` release builds above; no dependency resolution, tool reinstall or pin change was performed for this campaign. Missing/stale locks still fail closed. The pilot shell consumes public pinned providers, but this invocation used existing local/private caches and therefore does not establish clean public-runner reproducibility.

Equivalent already-provisioned shell:

```sh
bash pilots/orchestration/scripts/render-build.sh
RENDER_SKIP_BUILD=1 bash pilots/orchestration/scripts/render-demo.sh
RENDER_SKIP_BUILD=1 bash pilots/orchestration/scripts/render-faults.sh
```

The demo starts separate isolated controller/two-worker processes through the existing CLI, uploads the component/assignment objects, submits twice with one key (same run), obtains a consistent progress snapshot/watermark, disconnects and resumes bounded accepted-output pages, verifies apply-once projection, renders/downloads/verifies, restarts the completed controller (same accepted output), and executes a fresh run comparing every tile/final accepted output identity. It never deletes deployment roots; private temporary inputs/state/logs/artifacts remain on success or failure. Default loopback UDP ports 45200/45201/45202 can be overridden with `CONTROLLER_PORT`, `WORKER_PORT`, `WORKER2_PORT`. `CARGO_TARGET_DIR` is honored; `RENDER_POLL_LIMIT` bounds 0.2s polls (default 3000). Do not publish private roots/keys/databases.

Prepared artifacts: decoded `scene.json`, complete `manifest.json`, identity-checked `manifest.frame`, exact expected `coverage.json` rectangles/assignment identities, `job.json`, identity `plan.json` and published `*.input` literals. Retained statuses contain accepted tile/final identities. `output.rgb-envelope` is accepted content; `image.ppm` is a create-only presentation export after strict verification. `verification.json` records content/manifest/raw RGB identities. Actual successful script execution writes `metrics.json` with measured whole-second preparation, combined execution/scheduling/transfer/assembly, download and local verification times. Scheduling, tile transfer, assembly-only time, retry cost and peak memory remain explicit null (existing CLI does not expose them); no phase splits or memory values are invented.

## Manifest and deterministic rendering

Manifest v1 binds versioned canonical scene bytes/schema; camera eye/target/up/sensor half-height (integer millionths at unit focal distance); dimensions; sample count; nonzero seed and RNG policy; bounce limit; row-major columns/rows; renderer component **content identity**; batch and runtime profile; CPU/memory/slots/placement labels; fuel/deadline/attempt/retry limits; and RGB8/gamma2/top-left output. Output bounds are derived exactly from manifest dimensions and envelope overhead. Camera/schema/profile changes cannot reuse compatible-looking old pixels.

Frozen fixture: 640x360, 2 samples/pixel, 3 bounces, 8x4 tiles, seed `0x4d4f4c54454e`. This is a diffuse visual approximation using cube-direction hemisphere sampling, not a claimed unbiased physical-light-transport estimator. Paths escaping contribute sky; paths still occluded at the bounce limit contribute black. Output is top-row-first RGB8, sqrt channel transfer, clamp and nearest-integer rounding by floor(x+0.5).

The frozen runtime stays within the existing default policy: at most 1,000,000,000 fuel and 60,000ms per task, with 64MiB guest memory. No policy widening is part of this fixture. Actual resource sufficiency must be demonstrated by the component/process rail; exhaustion is a failed result, not permission to raise the profile silently.

Each pixel/sample owns a fresh xorshift64* stream. Initial state is seed XOR (linear pixel index * `0x9e3779b97f4a7c15`) XOR ((sample+1) * `0xd1b54a32d192ed03`), with wrapping u64 products; zero is replaced by `0xa0761d6478bd642f`. Draws shift right12/left25/right27 and multiply by 2685821657736338717; high53 bits yield [0,1) f64. Two draws jitter the primary ray; three draws choose each hemisphere direction. Stream order is independent of tile scheduling/partition. No clocks, imports, files, environment or random devices enter the guest.

The declared engine profile is Wasmtime45/Cranelift NaN canonicalization/f64 through the pinned import-free component. Profile strings and worker labels are declarations, not attestation or authority. Native tests qualify only their exercised native profile; the process fixture compares repeats on its actual same host. Narrower qualified cohorts must be enforced through existing worker labels; other architectures or native/Wasm equality require separately executed evidence.

## Identities, bounds and acceptance

Canonical semantic identity uses existing Preserves+BLAKE3, never Rust layout or JSON formatting. Assignment identities bind the complete manifest and tile index/assembly operation. `tile-000` etc are dependency labels, not independent semantic identities. Literal assignments precede dependency outputs; sorted tile dependencies supply deterministic input order. `assemble` is lexically first in the sorted JobSpec but executes after its dependencies.

At most 63 tiles + 1 assembly task; assembly's 1 literal + 63 dependencies fit 64 inputs. Default RGB is 691,200 bytes + 80-byte final header. Tile headers are 124 bytes. Validation checks aggregate assembly inputs INCLUDING its literal against 4MiB, not merely each object. The fixture stays within 4MiB/object, 4096 objects and 1GiB retained content for an isolated run; repeated submissions are still bounded by existing fail-closed retention, not automatic GC.

Tile format: `MRNTIL01`, 32-byte manifest ID, 32-byte assignment ID, five little-endian u32 fields index/x/y/width/height, 32-byte raw-pixel BLAKE3, RGB8. Final: `MRNRGB01`, manifest ID, width/height as little-endian u32, pixel BLAKE3, RGB8. No alternate versions/encodings accepted.

Assembly checks exact count/order/rectangles, assignments, lengths, manifest and digest. It rejects missing, duplicate/overlapping, malformed, stale/settings-mismatched tiles. It deliberately recomputes expected pixels before emitting output: self-consistent changed pixels with a rehashed digest are also rejected. That extra CPU is correctness validation, not cryptographic proof that remote work occurred. Strict final verification repeats this with tile-sized temporary memory. `decode_final` validates only structure/digest; it is NOT the strict acceptance API.

## Exact owner integration

The standalone workspace already includes `render`; CLI tests depend on it for real component fault fixtures. Scripts call existing `new-policy/init/allow-operator/add-worker/serve/put/submit/status/get` and the batch-specific `progress` observation command. No new scheduler or scene-aware backend protocol is introduced.

The helper binary and component guest call these pure workload APIs; CLI's test-only dependency supports real renderer fault campaigns:

- `frozen_manifest(component_id) -> Result<Manifest>` with actual uploaded component content-manifest identity, not a raw digest.
- `prepare(manifest) -> Result<Prepared>` exposing `.manifest`, `.objects: Vec<(Id, Vec<u8>)>` (Data content IDs/exact literals), `.job: JobSpec`. Publish component/objects first, submit through existing admission.
- `Manifest::{validate,id,encode,decode,tile,tile_count,image_bytes,scene}` and `Assignment::{id,encode}` for inspection.
- `content_id(kind, bytes)` uses exact existing `content-manifest.v1` semantics.
- `execute(inputs)` is the guest ABI body; `render_tile`, `assemble` and `verify_final` are the real pure workload APIs.
- For fetched results first compare `content_id(Data, bytes)` to the authoritative accepted output ID, then call `verify_final(manifest, bytes)`. Identity is not authorization.

Helper CLI:

```text
molten-render-fixture prepare COMPONENT NEW_DIR [MANIFEST_FRAME]
molten-render-fixture verify MANIFEST_FRAME OUTPUT ACCEPTED_CONTENT_ID NEW_PPM
```

Preparation binds the supplied component and emits inspectable inputs. Verification checks accepted content identity and deterministic pixels before create-only PPM output. No scene logic belongs in progress callbacks.

## Owner verification commands (demo execution recorded above)

```sh
cargo test --manifest-path pilots/orchestration/Cargo.toml -p molten-render-pilot
bash pilots/orchestration/scripts/render-build.sh
RENDER_SKIP_BUILD=1 bash pilots/orchestration/scripts/render-demo.sh
RENDER_SKIP_BUILD=1 bash pilots/orchestration/scripts/render-faults.sh
```

Native tests exercise analytical constant-sky RGB, fixed-stream repetition, partition independence, complete identity sensitivity, 64-task/64-input bounds, real ABI execution, malformed lengths/headers/coverage, missing/overlapping regions, stale settings and rehashed substitution. Process fixture adds real component/runtime/transport. The owner must apply normal formatting/lints/Cairn gates and existing lifecycle/progress suites, retain actual outcomes and separately execute two-host direct-LAN qualification before advertising it.
