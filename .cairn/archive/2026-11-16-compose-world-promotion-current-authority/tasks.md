## Phase 1: Implementation

- [x] [serial] Add canonical intent material, the content-addressed `LocalIntentMaterial` store, and the `publish-intent` command; read every plan intent's material at application time and deny `IntentMaterialUnavailable` before any head or reservation change. r[molten.world_promotion.transaction]
- [x] [serial] Add the pure `validate_operator_authority` decision and route deny, retry planning, and abandon through it with a verified operator grant; rebuild the original plan only from the committed identity, keeping duplicate-risk and unknown-outcome acknowledgements. r[molten.world_promotion.reconciliation]
- [x] [serial] Add positive and negative core, store, service, and public-command tests for missing and mismatched material, stale/revoked/foreign/absent operator authority, replacement requests, replay after a lost result, and lost completion. r[molten.world_promotion.transaction] r[molten.world_promotion.reconciliation]
- [x] [serial] Update `docs/world-promotion-current-facts.md` and `docs/world-promotion-and-effect-release.md`, and record the intent-material Preserves boundary schema. r[molten.world_promotion.transaction]

## Phase 2: Verification

- [x] [serial] Run the public command matrix (`worldpromotioncli`, `worldpromotiondenials`), the focused `world_promotion` library filters for `molten` and `molten-core`, Clippy with `-D warnings` on the touched targets, and `cargo fmt --check` on the source-bound tree; retain the execution record under `evidence/`. r[molten.world_promotion.transaction] r[molten.world_promotion.reconciliation]

The 2026-11-16 [ordered integrated execution](../../../evidence/integrated-cutover-2026-11-16/checks.json) passed both named public CLI suites, the complete workspace library selection containing both promotion owners, workspace/all-target Clippy, formatting and strict provider-policy Cairn validation. Its private original digest is preserved beside the command-path-redacted publication view.
