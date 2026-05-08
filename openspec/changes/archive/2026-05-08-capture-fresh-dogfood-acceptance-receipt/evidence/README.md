# Fresh dogfood acceptance receipt evidence

- command: `nix run .#dogfood-local -- full`
- run id: `dogfood-20260508T215019Z`
- commit: `096659c9760903e4956d07fa6323f7ae4d085c6b`
- local receipt: `/tmp/aspen-dogfood-receipts/dogfood-20260508T215019Z.json`
- cluster receipt key: `dogfood/receipts/dogfood-20260508T215019Z.json`
- result: all 7 receipt stages succeeded (`start`, `push`, `build`, `deploy`, `verify`, `publish_receipt`, `stop`)
- CI run: `714fe381-714c-4217-945c-e6440e9e15a1`
- CI jobs observed: `format-check`, `clippy`, `build-cli`, `build-node`, and `nextest-quick` passed before deployment.
- deployment observed: artifact `/nix/store/v6pfwxsq074r6mwd43h6m8nq13pfafgh-aspen-0.1.0`; deploy accepted as `deploy-1778277664885`; node 1 healthy; verification passed.
- receipt publication observed before cleanup: `dogfood/receipts/dogfood-20260508T215019Z.json`.

## Redacted evidence files

- `dogfood-20260508T215019Z.receipt.redacted.json` — local receipt with secret-bearing keys/values redacted.
- `receipt-show-096659c97.txt` — operator-facing receipt show output.
- `receipt-diagnose-096659c97.txt` — operator-facing receipt diagnosis output.

## Notes

Iroh relay/DNS warnings appeared during the local run, but the Iroh client path completed all dogfood stages and published the final receipt.
