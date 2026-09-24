# Verification: Restore the Cairn exemption fixture port

Base: `origin/molten` `4e31cee55167a38978961faac5c46476aca6f5ac`. Nickel: `nickel-lang-cli nickel 1.17.0 (rev 1320a98)`.

## Baseline failure

- `checks.x86_64-linux.contract-export-drift-gate` fails with "positive fixture failed:
  cairn-policy/fixtures/valid-with-exemption.ncl", array length mismatch (expected `0`, got `1`).
- The same fixture fails in upstream `../cairn` at `fde71b298b61856936c35b701876d4933393f0df`. It passed at
  `568f96f44^`, where the local `exemptions | force` port was still present. Upstream Cairn never evaluates
  `cairn-policy/fixtures/*.ncl`, so its defect is latent there. It is reported to the Cairn owner and not edited here.

## Positive

- `nickel export cairn-policy/fixtures/valid-with-exemption.ncl` succeeds and yields exactly one exemption
  (`scope = fixtures/basic/.cairn/changes/demo/tasks.md`, `owner = test`, `expires = 2099-01-01`).
- `nix build .#checks.x86_64-linux.contract-export-drift-gate` on the change branch: exit 0 (`/home/brittonr/git/OnixResearch/target/aspen-gate-blockers/c-contract-export-drift-gate.txt`).

## Negative

- The forced fixture with `owner` removed fails with "missing definition for `owner`" from the `Exemption`
  contract.
- `cairn-policy/fixtures/invalid-exemption-marker-policy.ncl` still fails to export.
