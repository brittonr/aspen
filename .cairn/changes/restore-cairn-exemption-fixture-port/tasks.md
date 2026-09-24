# Tasks: Restore the Cairn exemption fixture port

## Phase 1: Implementation

- [x] [serial] Restore `exemptions | force = [...]` in the vendored exemption fixture. a[restore-cairn-exemption-fixture-port.positive-export]
- [x] [serial] Add the fixture override to the `cairn-policy/UPSTREAM.md` local-port list. a[restore-cairn-exemption-fixture-port.provenance]

## Phase 2: Validation

- [x] [serial] Positive: export the fixture with the pinned Nickel and confirm exactly one exemption. a[restore-cairn-exemption-fixture-port.positive-export]
- [x] [serial] Negative: a forced exemption without `owner` and `invalid-exemption-marker-policy.ncl` both fail to export. a[restore-cairn-exemption-fixture-port.contract-still-applies]
- [x] [serial] Build `checks.x86_64-linux.contract-export-drift-gate` and record the result in `evidence/verification.md`. a[restore-cairn-exemption-fixture-port.gate-passes]
