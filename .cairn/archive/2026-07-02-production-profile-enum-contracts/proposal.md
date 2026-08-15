# Change: production-profile-enum-contracts

## Why

Several production profile arrays are free-form strings even though they represent reviewed vocabularies: required adapters, redaction settings, live transport settings, startup expectations, and shutdown expectations. A typo currently exports successfully and can become ambiguous receipt evidence.

## What

- Define explicit Nickel allowed-value contracts for production profile vocabularies.
- Apply those contracts to the adapter, redaction, live transport, startup, and shutdown arrays.
- Document how vocabulary additions are reviewed so new strings cannot appear without changing the contract.

## Impact

Profile export rejects misspelled or unreviewed vocabulary entries. New production adapters or expectations require an intentional contract update instead of silently entering receipts as opaque strings.
