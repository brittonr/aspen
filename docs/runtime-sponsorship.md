# Sponsored runtime grants

Aspen sponsored runtime grants let a sponsor authorize bounded runtime capacity for a beneficiary workload while a provider independently decides whether to admit the workload. This is a resource-authority and receipt boundary, not a payment rail.

## Boundary

Aspen owns:

- principal references for sponsors, beneficiaries, providers, workloads, services, nodes, and plugins;
- resource grant DTOs with bounded CPU, memory, disk, network, I/O, and operation budgets;
- grant scopes for workload, service, provider, isolation mode, and validity windows;
- quota ledgers, reservations, consumption totals, and remaining-capacity arithmetic;
- fail-closed admission checks before runtime-service, job, or CI placement;
- redacted usage receipts for execution start, reservation, consumption, completion, failure, and revocation-denial paths.

External systems own:

- bilateral settlement terms;
- currency, pricing, invoicing, tax, and payment workflows;
- provider-specific accounting backends;
- off-chain signatures or attestations referenced by opaque handles.

Aspen records only method tags and redacted/opaque settlement references. Token bodies, private keys, bearer values, connection strings, and raw signatures must not appear in grants, policies, diagnostics, or receipts.

## Policy contracts

`schemas/sponsored-runtime-policy.ncl` is Nickel-authored because operators write provider offers, sponsor policies, resource class catalogs, and admission profiles. It rejects invalid principal role combinations, unbounded limits, and secret-bearing settlement refs before Rust runtime code consumes the policy.

The runtime DTOs are Rust-owned and generated as Nickel contracts:

- `schemas/sponsored-runtime-grant.ncl`
- `schemas/sponsored-quota-ledger.ncl`
- `schemas/sponsored-usage-receipt.ncl`

The generated contracts are freshness-checked by `scripts/generate-typed-nickel-contracts.py --check` and fixture-checked by `scripts/check-typed-nickel-contract-fixtures.py`.

## Admission

Sponsored admission is optional until a workload or placement surface marks sponsorship as required. When required, `aspen-runtime-core` fails closed if there is no accepted grant or if any of these checks fail:

- missing sponsor, beneficiary, or provider principal proof reference;
- expired or revoked grant;
- provider does not accept the sponsor, settlement method, workload, service, provider principal, or isolation mode;
- requested resources exceed grant, provider policy, or quota-ledger remaining capacity;
- settlement references or diagnostics look like raw secrets.

The same pure placement constraint can be used by runtime services, jobs, and CI runs without coupling those subsystems to a payment implementation.

## Receipts

`SignedSponsoredUsageReceipt` wraps a redacted `SponsoredUsageReceipt` with a signer principal and redacted signature handle. Receipts identify the grant, sponsor, provider, workload, optional service, measured resources, artifact refs, isolation summary, and outcome. Required outcomes are:

- `started`
- `reserved`
- `consumed`
- `completed`
- `failed`
- `revocation-denied`

Receipt signatures are represented by redacted handles in this slice. The actual signing backend can be added later without changing the portable receipt shape.

## Verification

Focused checks:

```bash
CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets
python3 scripts/generate-typed-nickel-contracts.py --check
python3 scripts/check-typed-nickel-contract-fixtures.py
python3 scripts/check-typed-nickel-contract-registry.py
openspec validate define-sponsored-runtime-grants --strict
```
