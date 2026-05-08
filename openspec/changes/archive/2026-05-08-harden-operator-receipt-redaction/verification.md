# Verification

Commands run for `harden-operator-receipt-redaction`:

```bash
cargo test -p aspen-dogfood receipt -- --nocapture
cargo test -p aspen-cli --features ci ci_receipt_operator_outputs_redact_secret_markers -- --nocapture
cargo test -p aspen-cli --features ci ci_receipt_human_output_includes_artifact_metadata -- --nocapture
cargo test --test operator_receipts_docs -- --nocapture
openspec validate harden-operator-receipt-redaction --strict --json
python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify harden-operator-receipt-redaction --json
git diff --check
openspec validate dogfood-evidence --strict --json
openspec validate --all --strict --json
```

All commands passed. Existing warnings from vendored/client crates about unknown lints and vendored lifetime suggestions were informational and did not fail the focused tests.
