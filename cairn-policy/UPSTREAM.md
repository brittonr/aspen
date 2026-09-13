# Cairn policy provenance

The local lifecycle policy is vendored from the reviewed Cairn source revision `fde71b298b61856936c35b701876d4933393f0df`.

The files in this directory start from copies of `cairn-policy/*.ncl` from that revision, with these local ports of Molten's prior policy content:

- the `cairn-default` traceability profile uses `cairn/specs` requirement roots and `src`, `tests`, `docs`, and `scripts` evidence roots;
- the `fabric-transport` and `adopt-artifact-binding-and-semantic-effects` traceability profiles are local;
- the stack-provenance and function-address gates use Molten boundary text and placeholder policy hashes;
- `runtime_evidence_policy.aggregation_profiles` lists `smoke`, `release`, and `dogfood`;
- five `evidence .* validate` receipt schemas are local.

Note: `cairn policy export` writes evaluator scratch under `target/`. If `target` is a symlink, replace it with a real directory for the export, then restore the symlink.

The generated file `generated/cairn-policy.json` is the runtime input. Refresh it with the reviewed Cairn provider, then run lifecycle validation, lifecycle gates, and traceability checks.

## Consumer fixture adaptations

Molten lifecycle-policy maintainers own the fixture adaptations under `fixtures/`.
The exemption fixtures use Nickel `force` to replace the default empty array.
They still apply the complete `Contracts.Policy` contract.
Negative fixtures cover an empty owner, a missing owner, and an invalid array type.
The Nix gate requires their contract-specific diagnostics, not only a nonzero exit.
A positive fixture checks the exact replacement value and the unchanged empty default.

This adaptation changes test inputs and their checks only.
The vendored defaults, contracts, and generated runtime policy remain unchanged.
The runtime policy receives no new exemptions.
