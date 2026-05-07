# I10 auth/federation/operator UCAN docs

- Change: `adopt-sibling-ucan-auth`
- Task: Update auth/federation/operator docs with the adapter boundary, sibling dependency policy, migration notes, and unsupported UCAN interoperability caveats.
- Started: 2026-05-06T23:58:22Z
- Completed: 2026-05-06T23:59:29Z
- Status: PASS

## Documentation updates

- Added `docs/auth-ucan-adapter.md` with:
  - `aspen-auth-core` versus runtime `aspen-auth` boundary.
  - Current runtime behavior: legacy Aspen token wire format plus UCAN capability projection during verification.
  - Resource/ability mapping examples and pointer to the authoritative OpenSpec table.
  - Cargo/Nix sibling dependency policy and private-SSH failure mode.
  - Migration caveats for compact-token interoperability and Aspen-only authorization semantics.
  - Focused verification commands.
- Updated `docs/FEDERATION.md` overview with the UCAN-adapter auth boundary link.
- Updated `docs/operator-receipts.md` with auth/UCAN receipt redaction and evidence policy.

## Caveats retained

- No sibling UCAN compact-token interoperability is promised in this slice.
- Aspen CLI/RPC token fields remain legacy `CapabilityToken` based.
- Token bodies, signatures, private keys, bearer values, and private checkout paths must be redacted in operator receipts.
