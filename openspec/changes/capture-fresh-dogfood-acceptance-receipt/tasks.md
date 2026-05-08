## Phase 1: Acceptance Run

- [x] [serial] Create the OpenSpec baseline for fresh dogfood acceptance receipts.
- [ ] [serial] Confirm the checkout is clean/current and run `nix run .#dogfood-local -- full` from the committed source revision.
- [ ] [depends:dogfood-run] Capture the resulting dogfood receipt path or run id plus bounded operator summary evidence.

## Phase 2: Operator Readback

- [ ] [depends:receipt] Run receipt list/show/diagnose readback against the captured receipt and save a secret-safe transcript or summary.
- [ ] [depends:readback] Verify acceptance evidence redacts secrets and does not require raw logs or chat history.

## Phase 3: Closeout

- [ ] [depends:evidence] Run focused OpenSpec/whitespace checks and update verification notes before archive.
