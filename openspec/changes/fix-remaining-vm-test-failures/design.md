## Context

7 of 9 NixOS VM tests pass after recent fixes. The remaining 3 are pre-existing bugs in application code, not test infrastructure. Each has a distinct root cause requiring a targeted fix.

**Transit datakey**: The WASM secrets plugin's `handle_datakey()` conditionally includes plaintext based on `key_type == "plaintext"`. The SOPS client sends `"aes256-gcm"` (the encryption algorithm, not the return mode). The CLI defaults to `"plaintext"` and works fine — only the SOPS encrypt path is broken.

**Multinode deploy timeout**: 3-node QEMU (12GB total, 6 cores) running `nix build` for cowsay takes longer than the 600s timeout. The single-node deploy test already uses `validate_only` mode successfully.

**Snix bridge import**: `snix-store import` through the gRPC bridge fails. The bridge translates snix's BlobService/DirectoryService/PathInfoService gRPC protocol to Aspen's in-memory backends. The import protocol requires writing blobs, creating directory entries, and registering PathInfo — one of these steps fails.

## Goals / Non-Goals

**Goals:**

- All 9 NixOS VM tests pass
- Transit datakey returns plaintext for all callers by default
- Multinode deploy test exercises rolling deploy coordination without depending on nix build speed
- Snix bridge import works end-to-end with in-memory backends

**Non-Goals:**

- Optimizing nix build performance inside QEMU
- Adding actual binary deployment logic to the multinode test (validate_only is sufficient)
- Cluster-connected snix bridge mode (only in-memory mode is tested)

## Decisions

### 1. Transit datakey: invert the plaintext condition

Change `key_type == "plaintext"` to `key_type != "wrapped"` in the plugin. This matches Vault semantics: the "plaintext" endpoint returns both, the "wrapped" endpoint returns only ciphertext. Unknown values default to including plaintext.

Also fix the SOPS client to send `"plaintext"` instead of `"aes256-gcm"` — belt and suspenders.

**Alternative**: Only fix the SOPS client. Rejected because any future caller sending an unexpected key_type would hit the same bug.

### 2. Multinode deploy: validate_only mode

Add `validate_only: true` and `expected_binary: "bin/cowsay"` to the deploy job config. The deploy executor validates the artifact exists without switching nix profiles. Update the test assertion from "deploy should fail" to "deploy should succeed".

**Alternative**: Increase timeout to 1200s. Rejected because it doesn't fix the fundamental issue (test tests the wrong thing) and makes CI slower.

### 3. Snix bridge: debug-first approach

Add diagnostic tracing to the BlobService write path and PathInfoService put path. Run the simpler `snix-bridge-test` to isolate whether import fails there too. Check for proto version mismatches between snix-store and the bridge's snix crate deps.

## Risks / Trade-offs

- **[Transit]** Changing plaintext default could expose data keys to callers that previously got `None` → Low risk, the datakey endpoint is specifically for getting plaintext keys.
- **[Multinode]** `validate_only` means we never test actual profile switching in multi-node → Acceptable, the single-node test covers that path.
- **[Snix]** Root cause unclear until debugging → May take longer than estimated. Scope the fix to in-memory mode only.
