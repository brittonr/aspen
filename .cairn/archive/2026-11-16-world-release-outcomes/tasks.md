## Phase 1: Implementation

- [x] [serial] Add the explicit attempt outcome; classify failed observations as `failed` and deny their acknowledgement and observation commit. r[molten.world_promotion.dispatch]
- [x] [serial] Version the canonical attempt record as v2, record its schema artifact, and reject unsupported outcomes and the v1 layout. r[molten.world_promotion.dispatch]
- [x] [serial] Record the attempt and reservation as uncertain on a dispatcher error and surface both errors when recording fails; prove retry and abandon start from that state. r[molten.world_promotion.dispatch]
- [x] [serial] Classify attempts interrupted before their result was recorded as uncertain in `reconcile`, and prove abandon applies afterwards. r[molten.world_promotion.reconciliation]
- [x] [serial] Run focused `molten-core` and root world promotion tests, formatting, Clippy and source/lifecycle gates before review, sync and archive. r[molten.world_promotion.dispatch]
