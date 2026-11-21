# Fresh guest source admission and single-vCPU timer controls

The earlier timings in [guest-profile host evidence](2026-11-21-guest-profile-host.md) describe an experimental shortcut. Node-root cache bytes plus a pathname and length do not establish current source authentication. The corrected profile retains runtime-only scope and uses fresh Nix signature verification, executing-file measurement, and gate validation on startup. The existing immutable-store closure cache remains separate, downstream of those checks.

## Exact signed source controls

The candidate was built from the Aspen `701633f4701402480c56c53fedd82cca379f22c3` base plus the cache removal and baseline observation-retention edits. Pueue task 36 passed `cargo build --locked --bin molten` and `cargo test --locked --test collaborationnative --no-run`; test execution is separate. The prepared binary and driver were stripped before measurement. An isolated experimental Nix key signed `/nix/store/13b3705ym3g56ll6yrp6l10grpq2hilr-molten-guest-runtime-binding-v1`, made by `scripts/make-guest-runtime-binding` with `MOLTEN_GUEST_EXECUTABLE` set to the newly built candidate. The retained build policy and gate came from the full donor; they are not a fresh proof of this binary's build closure.

| Member | BLAKE3 |
| --- | --- |
| Signed Aspen binary | `d63c4a4987143d12312922eb943f73aa5d5a139125266471c4aa26be345e34c5` |
| Collaboration driver | `6e0b403627f52a69b17951ef1e9520bf76f70081e0693844dc406332183d2cc7` |
| Trust selection | `c74a2b7fdde52d9efde338760028dba89400986d7f8cb85459158b7a635aa017` |
| Binding | `f0fa00d4a727548c83b9a2a570ee684184e429d9bcb03a35a3f9b3b19c40c00c` |
| Runtime-only closure | `78fb99e2aa4342485b7a8b433be8cd3433660ec511faadff1eb86f7519077359` |

Two fresh `molten node init` roots received the earlier experimental cache file. `molten node run --state-root <root>` denied a different selected public key (exit 1, no signature by the independently selected key). Running the altered executable against the real signed output and original trust denied with `WrongCandidate` (exit 1). Neither root published a startup receipt. The first attempt stopped before running either control because the disposable executable inherited a read-only permission; the retry made only that scratch copy writable before changing it.

Pueue task 44 also passed the four focused `source_binding::tests` library controls and focused Clippy for the hardened `molten` executable and collaboration test target. These are source-admission controls, not transport or guest execution.

Pueue task 41 ran `collaborationnative --ignored --exact driver::collaboration_native_baseline --nocapture` with `MOLTEN_TEST_SOURCE_TRUST` selecting the output above and `MOLTEN_TEST_EVIDENCE_ROOT` selecting a fresh output directory. It passed in 347.02 seconds and emitted the [canonical baseline observation](b0ec5388b55a312049f21de2d99fb82e92b6bacc91cfe8a07f970fdd93b411f0.preserves). Independently measured BLAKE3 matches the observation reference. This is runtime-only, three distinct processes on one host, not a guest campaign or replay result.

## Timer discrimination, not a campaign

On ChaosControl `9b2d53663c622b059b7833629fd194057864fbc2`, an isolated Rust PID-1 control sent a 14-byte datagram between two loopback sockets and read it back. It then performed a one-second `std::thread::sleep`. With an outer 100-second bound, sleep started but did not finish. With an outer 220-second bound, the exact same initrd reported `sleep-done elapsed_ms=1000`; the host limit later stopped the idle probe. The first packaging attempt failed before the probe because its `libgcc_s` RUNPATH did not match the copied runtime. No assertion catalog, Aspen runtime, QUIC, Raft, campaign success, or replay is implied by this control.

Both readouts used `chaoscontrol-explore run` with one VM/vCPU, 256 MiB, one round/branch, `--ticks 20000 --bootstrap-budget 120000 --seed 42 --mode input-tree --random-faults false`. Corrected probe initrd BLAKE3 `208da886af73a45fcf69c029d98db2d188f06187b6cff1a08d8c8e05c1264ff1`; kernel `f2abae8a52c83c975ed15de504535445e2ac159cc0198f8028867b0c18cdf67c`; explorer `e3f2386e0193a25d4a51545300f603c553e3f00cdb8f7a5b825df467d4f3be76`.

The guest used `clocksource=jiffies notsc`. Architectural idle permits ChaosControl to advance to a PIT deadline after the host watchdog returns control. The test distinguishes working guest UDP from slow idle timer progress; it does not prove that every real Raft failure is caused by timing. The revised wrapper uses the existing SDK progress boundary during the actual driver wait, never a synthetic commit or host-clock-derived guest time. Aspen startup maintains the source trust boundary; ChaosControl retains guest timing and campaign evidence ownership.

## Hardened KVM attempts: no positive completion observation

Tasks 42 and 48 ran the hardened cohort above, with kernel and explorer bytes matching the timer probe. Wrapper BLAKE3 `6ef770e55f2a074dc76f4e9f19cbc6aa3136f3e0c2c2240f49c768f4161312bc`, initrd BLAKE3 `ab07774d575b3295ccb79b36af79ff4524d637509ed14e4dc2a4e8bf5b3226b8`. Common selection:

```sh
chaoscontrol-explore run --kernel "$KERNEL" --initrd "$INITRD" \
  --memory-mb 2048 --vms 1 --vcpus 1 --rounds 1 --branches 1 \
  --quantum 100 --ticks "$TICKS" --bootstrap-budget 120000 --seed 42 \
  --mode input-tree --random-faults false \
  --extra-cmdline 'molten_source_trust=/etc/molten/source-gate-trust.json molten_guest_workspace=/data/molten' \
  --output "$EVIDENCE"
```

| Task | Tick limit | Bound / actual wall time | Last observed scope round | Checkpoint BLAKE3 |
| --- | ---: | --- | ---: | --- |
| 42 | 50,000 | 1,200 s / 467.59 s | 1 | `bc469f265cf70bf372c794380b3ffabedfad3c7301be6e636c509528429f2e35` |
| 48 | 100,000 | 1,800 s / 523.84 s | 3 | `47e35db505db60ec113763dcd74069aa202e56fb878cc8676ec9d6c48b95cd99` |

Both produced the same [assertion summary](8d1bed7a016be71d65770d3cfd80095d158c52d926fddd67d75139225e403161.json). All three signed nodes reported hosting readiness; every observed diagnostic read returned `collaboration-scope/partition`. The completion assertion (2857709567) had zero hits and verdict `failed`; no property violation or successful driver exit was observed. Both explorer exits were zero, which proves neither completion nor a passing campaign. Neither run reported a discarded branch or a branch watchdog firing. No baseline observation, replay candidate, or replay verdict was retained. The host passed after its seventh scope round; neither guest horizon reached that many rounds, so these results do not distinguish a short horizon from an abiding quorum failure.
