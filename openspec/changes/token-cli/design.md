## Context

Aspen's `aspen-auth` crate provides a complete UCAN-inspired capability token system: `TokenBuilder` for creation, `TokenVerifier` for verification, `CapabilityToken` for wire format (postcard + base64), and delegation chain validation. The RPC pipeline in `aspen-rpc-handlers` enforces auth via `handle_client_request_check_auth()`, checking tokens against operations derived from each `ClientRpcRequest`.

The CLI (`aspen-cli`) already accepts `--token <base64>` globally and attaches it to every RPC request. But there's no way to **create** tokens outside of Rust code. The `generate_root_token()` function exists in the library but is only callable programmatically.

For self-hosted operations (Forge, CI), operators need to generate root tokens at bootstrap and mint scoped tokens for services — from the command line.

## Goals / Non-Goals

**Goals:**

- Add `token generate` to create root tokens from a secret key
- Add `token inspect` to decode and display any token's metadata
- Add `token delegate` to mint child tokens with reduced scope from a parent
- All commands work offline — no cluster connection needed
- Output tokens in the same base64 format accepted by `--token`

**Non-Goals:**

- Token revocation (requires cluster connection and KV store — separate concern)
- Key management / key rotation (Ed25519 keys are managed outside aspen-cli)
- Modifying the token wire format or auth verification pipeline
- Adding new capability types

## Decisions

### 1. Subcommand of `aspen-cli`, not a separate binary

**Choice**: `aspen-cli token {generate,inspect,delegate}`

**Rationale**: AGENTS.md references an `aspen-token` binary, but a subcommand is better because:

- Operators already use `aspen-cli` — one tool, not two
- The `--token` global flag and `token` subcommand live in the same binary (discoverability)
- No new crate, no new Cargo.toml, no new binary target
- Follows the pattern of every other CLI feature (kv, lock, cluster, ci, etc.)

**Alternative rejected**: Separate `aspen-token` crate. Adds packaging/distribution overhead for three simple commands that share no state with each other.

### 2. Secret key input via file path or environment variable

**Choice**: `--secret-key-file <path>` flag plus `ASPEN_SECRET_KEY` env var (hex-encoded).

**Rationale**: Secret keys should never appear in shell history or process lists. File path is the primary input (matches how iroh stores keys). Env var is the fallback for CI/automation where file mounts are awkward. The key file format matches iroh's existing `SecretKey::try_from_openssh()` or raw 32-byte hex.

**Alternative rejected**: `--secret-key <hex>` positional/flag argument. Leaks secrets to `ps aux`, shell history, and log files.

### 3. Capability specification via repeated `--cap` flags

**Choice**: `--cap <spec>` where spec is a simple string format:

```
--cap full:               # Full access, all prefixes
--cap full:myapp/         # Full access under myapp/
--cap read:logs/          # Read-only under logs/
--cap write:data/         # Write under data/
--cap delete:tmp/         # Delete under tmp/
--cap watch:events/       # Watch under events/
--cap cluster-admin       # Cluster admin operations
--cap delegate            # Can create child tokens
--cap shell:*             # All shell commands
--cap shell:pg_*          # Shell commands matching pg_*
--cap secrets-full:secret/:   # Full secrets access
--cap secrets-read:kv/:app/   # Read secrets under kv/app/
```

**Rationale**: Concise, greppable, shell-friendly. The `type:prefix` format maps directly to the `Capability` enum variants. Repeated `--cap` flags compose naturally.

**Alternative rejected**: JSON capability specs. Verbose, hard to type, error-prone quoting in shells.

### 4. Inspect outputs human-readable by default, JSON with `--json`

**Choice**: Default output is a formatted table showing issuer, audience, capabilities, expiry, delegation depth. `--json` flag (already a global option) outputs structured JSON.

**Rationale**: Operators debugging auth issues need to quickly see "what does this token allow?" Human-readable is the default. Scripts and automation use `--json`.

### 5. Delegate takes parent token as input and produces child token

**Choice**: `token delegate --parent <base64> --secret-key-file <path> --cap <spec> --lifetime <duration>`

The parent token is provided as base64 (same format as `--token`). The secret key signs the child. The builder validates that child capabilities are subsets of the parent's and that the parent has the `Delegate` capability.

**Rationale**: Reuses the existing `TokenBuilder::delegated_from()` API directly. No new validation logic needed — the library already enforces attenuation and depth limits.

### 6. Lifetime as human-readable duration

**Choice**: `--lifetime 365d` / `--lifetime 24h` / `--lifetime 30m`. Parsed to `Duration`.

**Rationale**: `--lifetime 31536000` (seconds) is unreadable. Simple suffix parsing (d/h/m/s) covers all real use cases.

## Risks / Trade-offs

**[Risk: Secret key exposure in memory]** → Secret keys are read from file/env, used once to sign, then dropped. No long-lived key material. Rust's `SecretKey` type zeros on drop.

**[Risk: Generated tokens written to stdout]** → This is intentional. Operators pipe to a file or secret store. Adding `--output <file>` would be convenient but isn't needed for v1 — shell redirection works.

**[Trade-off: No interactive mode]** → All commands are one-shot. No wizard for building capabilities interactively. This keeps the implementation simple and scriptable. Operators who need help can use `token inspect` to verify what they built.

**[Trade-off: No `token list` or `token revoke`]** → These require cluster state (KV store lookups). They're RPC operations, not local operations. Out of scope for this change — they'd go through the normal `--token` authenticated RPC path.
