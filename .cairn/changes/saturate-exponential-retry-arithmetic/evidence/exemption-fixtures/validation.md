# Exemption fixture validation

## Result and owner

The valid exemption fixture now exports successfully.
The Nix contract-export gate passes with exact-value and contract-diagnostic checks.
Molten lifecycle-policy maintainers own this consumer fixture adaptation.

The reviewed policy source remains Cairn revision `fde71b298b61856936c35b701876d4933393f0df`.
`cairn-policy/UPSTREAM.md` records the local adaptation.
No vendored default, contract, generated runtime policy, dependency revision, or lockfile changed.
The runtime policy still contains no exemptions.

## Defect and correction

The original valid fixture merged a one-element array with the default empty array at equal priority.
The fresh baseline failed with `array length mismatch (expected 0, got 1)`, exit 1.
The fixture now uses Nickel `force` for explicit replacement and retains `Contracts.Policy`.
This priority change selects a value. It does not bypass its contract.

Three negative fixtures retain the same override structure.
They reject an empty owner, a missing owner, and a non-array value.
Their observed diagnostics name the intended field and contract error.
A separate positive fixture compares the exact exemption record and the unchanged empty runtime default.

The Nix gate now requires those contract-specific diagnostics.
An unrelated syntax or import error cannot satisfy the new negative controls.
Existing fixture selection, export-drift checks, and other gates remain intact.

## Executed checks

| Check | Result |
|---|---|
| Original positive fixture | Expected reproduction failure, exit 1 |
| Corrected positive fixture | Exit 0 |
| Empty owner | Exit 1, ``contract broken by the value of `owner` `` |
| Missing owner | Exit 1, ``missing definition for `owner` `` |
| Wrong array type | Exit 1, ``contract broken by the value of `exemptions` `` and `expected an array` |
| Exact replacement and empty default | Exit 0 |
| Initial Nix contract-export gate | Exit 0 |
| Nix gate with value and diagnostic checks | Exit 0 |
| `nixfmt --check flake.nix` | Exit 0 |
| Whitespace and unchanged protected inputs | Exit 0 |

The final Nix derivation is `/nix/store/zww34yqvfli5dh21mqhzhrh2kay95nv3-molten-contract-export-drift-gate.drv`.
Cache connection errors preceded a successful build. They were not the earlier fixture defect.
The Nix gate exports the default policy and compares it with the generated runtime input.
It includes all new fixtures through the existing source selection.

## Reproduction commands

Run each command from the repository worktree.
The invalid fixture exports must exit 1 with the recorded contract diagnostics.

```console
nix develop --no-write-lock-file -c nickel export cairn-policy/fixtures/valid-with-exemption.ncl
nix develop --no-write-lock-file -c nickel export cairn-policy/fixtures/valid-exemption-replacement.ncl
nix develop --no-write-lock-file -c nickel export cairn-policy/fixtures/invalid-exemption-owner.ncl
nix develop --no-write-lock-file -c nickel export cairn-policy/fixtures/invalid-exemption-missing-owner.ncl
nix develop --no-write-lock-file -c nickel export cairn-policy/fixtures/invalid-exemption-array-type.ncl
nix build .#checks.x86_64-linux.contract-export-drift-gate --no-link -L --no-write-lock-file --max-jobs 1
nixfmt --check flake.nix
```

## Review and limits

A read-only reviewer produced an advisory report and reached its five-minute deadline, exit 124.
The report identified the risk of accepting any nonzero negative-fixture exit.
The coordinator added exact diagnostics and a positive value assertion, then repeated the Nix gate.
Direct evaluator diagnostics and the default export comparison supply behavioral evidence, not review agreement.

The retained archives and BLAKE3 identities bind these fixture inputs and command results.
They do not establish a passing whole-repository Nix gate, runtime exemption authority, F12 completion, or release acceptance.
The all-feature Cargo metadata blocker and other required gates remain separate.
