# Proposal: Restore the Cairn exemption fixture port

## Why

`checks.x86_64-linux.contract-export-drift-gate` fails on `origin/molten` (`4e31cee55`) with
`positive fixture failed: cairn-policy/fixtures/valid-with-exemption.ncl` and a Nickel array-merge error ("array length
mismatch (expected `0`, got `1`)"). The vendored-policy refresh in `568f96f44` copied upstream Cairn `fde71b2`'s fixture
verbatim. That copy dropped Molten's earlier local port `exemptions | force = [...]`. Without `| force`, the fixture
merges a one-element array into `default.ncl`'s `exemptions = []`, and Nickel 1.17.0 (rev `1320a98`) rejects the merge.
With the port (at `568f96f44^`), the fixture exported successfully. Upstream Cairn never evaluates this fixture, so
upstream carries the same latent defect.

## What Changes

- Restore `exemptions | force = [...]` in the vendored `cairn-policy/fixtures/valid-with-exemption.ncl`.
- Record the port in `cairn-policy/UPSTREAM.md`'s local-port list, following the vendored-policy provenance procedure.
- Report the upstream fixture defect to the Cairn owner. This change does not edit `../cairn`.

## Impact

- **Files**: `cairn-policy/fixtures/valid-with-exemption.ncl`, `cairn-policy/UPSTREAM.md`.
- **Testing**: `nix build .#checks.x86_64-linux.contract-export-drift-gate`; the positive fixture exports; negative
  exemption fixtures still reject.

## Out of Scope

- Accepted specifications do not change. The fixture is test input for the vendored policy contracts. The policy
  contracts, `default.ncl`, and `generated/cairn-policy.json` are unchanged. A future upstream refresh must keep or
  upstream this port.
