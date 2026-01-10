# Dependency Vulnerabilities

**Severity:** HIGH
**Category:** Security
**Date:** 2026-01-10

## Summary

`cargo audit` identified 5 security vulnerabilities in dependencies.

## Vulnerabilities Found

### RUSTSEC-2024-0344: curve25519-dalek (v3.2.0)

- **Title:** Timing variability in `Scalar29::sub`/`Scalar52::sub`
- **Solution:** Upgrade to >=4.1.3
- **Dependency Path:** `libpijul` -> `ed25519-dalek` -> `curve25519-dalek`

### RUSTSEC-2022-0093: ed25519-dalek (v1.0.1)

- **Title:** Double Public Key Signing Function Oracle Attack
- **Solution:** Upgrade to >=2
- **Dependency Path:** `libpijul` -> `ed25519-dalek`

### RUSTSEC-2025-0140: gix-date (v0.9.4)

- **Title:** Non-utf8 String can be created with `TimeBuf::as_str`
- **Solution:** Upgrade to >=0.12.0
- **Dependency Path:** `aspen-forge` -> `gix-*` -> `gix-date`

### RUSTSEC-2025-0021: gix-features (v0.39.1)

- **Title:** SHA-1 collision attacks are not detected
- **Severity:** 6.8 (medium)
- **Solution:** Upgrade to >=0.41.0
- **Dependency Path:** `aspen-forge` -> `gix-pack` -> `gix-features`

### RUSTSEC-2023-0071: rsa (v0.9.9)

- **Title:** Marvin Attack: potential key recovery through timing sidechannels
- **Severity:** 5.9 (medium)
- **Solution:** No fixed upgrade available
- **Dependency Path:** `aspen-forge` -> `ssh-key` -> `rsa`

## Unmaintained Dependencies (Warnings)

- `atomic-polyfill 1.0.3` (RUSTSEC-2023-0089)
- `atty 0.2.14` (RUSTSEC-2024-0375)
- `bincode 1.3.3` (RUSTSEC-2025-0141)
- `fxhash 0.2.1` (RUSTSEC-2025-0057)
- `instant 0.1.13` (RUSTSEC-2024-0384)

## Recommendation

1. Coordinate with `libpijul` maintainers to upgrade ed25519 dependencies
2. Update gix-* dependencies to latest versions
3. Monitor rsa crate for security patches
4. Consider replacing unmaintained dependencies where feasible
