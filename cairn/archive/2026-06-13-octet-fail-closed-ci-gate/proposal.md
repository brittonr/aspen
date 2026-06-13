## Why

The first Octet/TigerStyle run showed why source evidence must be fail-closed: `cargo-octet` can produce artifacts and exit successfully while still reporting `warning-only`. Treating those artifacts as informational would repeat the pattern Molten is trying to avoid: evidence exists, but CI/release/admission can still proceed without enforcing it. Current strict runs are configuration-clean, and this change records the canonical gate that makes warning-only, stale, malformed, missing, or disconnected artifacts deny.

Molten needs a canonical Octet gate that fails closed when artifacts are missing, stale, malformed, warning-only, or disconnected from the source/config/profile that produced them. The gate must be explicit about transition profiles: a temporary quarantine profile may exist while warnings are burned down, but strict CI/release/admission gates must reject unreviewed findings.

## What Changes

- Add a canonical `octet-gate-policy-v1` and `octet-gate-receipt-v1` model that binds the Octet command, workspace metadata config hash, profile hash, toolchain, status artifact, summary artifact, structured findings artifact, object corpus receipt, fingerprint evidence, baseline/review refs, and decision.
- Add a strict Octet gate profile where `warning-only` is a failing status unless every finding is either absent or covered by an unexpired reviewed quarantine receipt allowed for that profile.
- Require missing Octet artifacts, missing object corpus receipts, stale config/profile hashes, unsupported tool versions, malformed SARIF/JSON/status files, and noncanonical replay commands to deny before any harness, release, admission, or upgrade receipt can claim source-gate pass evidence.
- Escalate high-risk Tiger Style/Octet classes (`no_panic`, `no_unwrap`, `ambient_clock`, `unbounded_loop`, unbounded resource growth on critical surfaces, secret rendering, harness backdoors, and authority typing) to immediate deny in strict profiles.
- Add a local CLI/test command shape for producing the Octet gate receipt from `target/octet` artifacts, and wire it into the documented CI path alongside Cairn validation and harness gates.

## Impact

This turns Octet from an advisory warning report into a first-class fail-closed evidence gate. Strict profiles cannot pass with warning-only status, stale artifacts, missing object-corpus/fingerprint evidence, unaudited critical findings, or raw process output. It also preserves reproducibility by binding every gate decision to the exact command/config/toolchain/artifact refs that produced it and by requiring downstream source-gate validation before node startup, remote admission, upgrades, or release evidence can claim a pass.
