# Design: Restore the Cairn exemption fixture port

## Context

Molten vendors Cairn's Nickel policy from reviewed Cairn revision `fde71b2` and records its Molten-local ports in
`cairn-policy/UPSTREAM.md`. The contract-export drift gate exports every `cairn-policy/fixtures/valid*.ncl` fixture as a
positive case and every other fixture as a negative case.

## Decisions

### Decision: Restore the `| force` override as a documented local port

**Choice:** Use `exemptions | force = [...]` in the fixture and list the port in `UPSTREAM.md`.

**Rationale:** `default.ncl` intentionally ships `exemptions = []`. Changing it to `| default` would edit the runtime
policy source and the generated `cairn-policy.json`, which widens the change. The fixture exists to demonstrate one
bounded exemption, so overriding the field in the fixture expresses that intent directly. The `Contracts.Policy`
contract still applies to the forced value.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: `cairn-policy/fixtures/valid-with-exemption.ncl`,
`cairn-policy/contracts.ncl` (`Exemption` contract), `cairn-policy/default.ncl` (`exemptions = []`), and
`cairn-policy/UPSTREAM.md`.

## Failure behavior

If the forced exemption violates the `Exemption` contract (for example, a missing `owner`), export still fails, and the
drift gate reports the positive fixture as failed.

## Risks / Trade-offs

- A later upstream refresh can drop the port again. The `UPSTREAM.md` entry makes the port visible to that procedure,
  and the drift gate catches the regression.
