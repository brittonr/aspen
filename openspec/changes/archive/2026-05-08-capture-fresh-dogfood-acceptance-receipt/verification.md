# Verification: Fresh dogfood acceptance receipt

## Repository fixes required before acceptance

The first three `nix run .#dogfood-local -- full` attempts correctly failed before acceptance:

- `dogfood-20260508T205611Z`: CI format-check failed; fixed by committing Rust formatting (`ae63f6b91`) and then Nix formatting (`e0a1ec472`).
- `dogfood-20260508T210709Z`: CI format-check still failed until `nix fmt .` output was committed.
- `dogfood-20260508T211236Z`: CI clippy failed on `empty_line_after_doc_comments`; fixed in `a9d61f8bf`.
- `dogfood-20260508T212812Z`: CI nextest failed because the CI source snapshot omitted doc-test inputs used by runtime-host readiness tests; fixed in `096659c97`.

These failed receipts were not accepted as completion evidence.

## Accepted run

Accepted dogfood command:

```bash
nix run .#dogfood-local -- full
```

Accepted receipt:

- run id: `dogfood-20260508T215019Z`
- source commit: `096659c9760903e4956d07fa6323f7ae4d085c6b`
- local receipt path: `/tmp/aspen-dogfood-receipts/dogfood-20260508T215019Z.json`
- published cluster key observed before cleanup: `dogfood/receipts/dogfood-20260508T215019Z.json`
- final process exit: `0`
- receipt stages: `start`, `push`, `build`, `deploy`, `verify`, `publish_receipt`, and `stop` all succeeded
- CI pipeline observed: `714fe381-714c-4217-945c-e6440e9e15a1`
- CI jobs observed: `format-check`, `clippy`, `build-cli`, `build-node`, and `nextest-quick` succeeded
- deploy observed: artifact `/nix/store/v6pfwxsq074r6mwd43h6m8nq13pfafgh-aspen-0.1.0`; deploy id `deploy-1778277664885`; node 1 healthy; verification passed

## Operator readback evidence

Saved under `openspec/changes/capture-fresh-dogfood-acceptance-receipt/evidence/`:

- `README.md`
- `dogfood-20260508T215019Z.receipt.redacted.json`
- `receipt-list-096659c97.txt`
- `receipt-show-096659c97.txt`
- `receipt-diagnose-096659c97.txt`

Readback commands used:

```bash
nix run .#dogfood-local -- receipts list
nix run .#dogfood-local -- receipts show /tmp/aspen-dogfood-receipts/dogfood-20260508T215019Z.json
nix run .#dogfood-local -- receipts diagnose /tmp/aspen-dogfood-receipts/dogfood-20260508T215019Z.json
```

## Focused checks

```bash
nix fmt . -- --check
nix run .#rustfmt -- check
nix build .#checks.x86_64-linux.fmt --no-link -L
nix build .#checks.x86_64-linux.clippy --no-link -L
nix build .#checks.x86_64-linux.nextest-quick --no-link -L
openspec validate capture-fresh-dogfood-acceptance-receipt --strict --json
git diff --check
```

Post-archive validation:

```bash
git diff --check
openspec validate --all --strict --json
```
