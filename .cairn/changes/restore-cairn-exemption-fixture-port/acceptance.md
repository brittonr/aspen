# Change-local acceptance

a[restore-cairn-exemption-fixture-port.positive-export] `cairn-policy/fixtures/valid-with-exemption.ncl` exports one exemption under the vendored `Contracts.Policy` contract with Nickel 1.17.0.
a[restore-cairn-exemption-fixture-port.contract-still-applies] A forced exemption missing a required field, and the existing negative exemption fixture, still fail to export.
a[restore-cairn-exemption-fixture-port.provenance] `cairn-policy/UPSTREAM.md` lists the fixture override as a Molten-local port.
a[restore-cairn-exemption-fixture-port.gate-passes] `checks.x86_64-linux.contract-export-drift-gate` builds on the change branch.
