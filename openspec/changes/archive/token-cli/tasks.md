## 1. Parsing Utilities

- [x] 1.1 Add `parse_duration` function to parse human-readable durations (`365d`, `24h`, `30m`, `3600s`, bare seconds) into `std::time::Duration`
- [x] 1.2 Add `parse_capability` function to parse `--cap` string format (`full:prefix`, `read:prefix`, `cluster-admin`, `delegate`, `shell:pattern`, etc.) into `Capability` enum variants
- [x] 1.3 Add `parse_secret_key` function to load an Ed25519 `SecretKey` from file path (`--secret-key-file`) or hex env var (`ASPEN_SECRET_KEY`)

## 2. Token Command Module

- [x] 2.1 Create `crates/aspen-cli/src/bin/aspen-cli/commands/token.rs` with `TokenCommand` enum (`Generate`, `Inspect`, `Delegate`) using clap derive
- [x] 2.2 Register `TokenCommand` in `Commands` enum in `cli.rs` and add the subcommand dispatch

## 3. Generate Subcommand

- [x] 3.1 Implement `token generate` — read secret key, parse `--cap` flags (default to root caps if none provided), parse `--lifetime`, build token via `TokenBuilder`, output base64
- [x] 3.2 Add tests for generate: default root token, specific capabilities, missing key error, invalid lifetime error

## 4. Inspect Subcommand

- [x] 4.1 Implement `token inspect` — decode base64 token, display issuer, audience, capabilities, timestamps (human-readable and UTC), delegation depth, nonce presence, expired indicator
- [x] 4.2 Implement JSON output mode for inspect (reuse `--json` global flag)
- [x] 4.3 Add tests for inspect: valid token display, expired token indicator, invalid base64 error

## 5. Delegate Subcommand

- [x] 5.1 Implement `token delegate` — parse parent token base64, read secret key, parse child `--cap` flags, parse `--lifetime`, build via `TokenBuilder::delegated_from()`, output base64
- [x] 5.2 Add tests for delegate: successful attenuation, escalation rejection, missing Delegate capability error, max depth exceeded error

## 6. Integration

- [x] 6.1 Add clap parse tests for all token subcommands (arg parsing, help text)
- [x] 6.2 Verify round-trip: generate token → inspect token → confirm fields match
- [x] 6.3 Verify round-trip: generate root → delegate child → inspect child → confirm reduced capabilities and incremented depth
