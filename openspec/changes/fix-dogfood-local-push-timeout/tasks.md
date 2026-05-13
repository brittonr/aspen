## Tasks

- [x] [serial] Reproduce the local dogfood push timeout with a large writable cluster dir and preserve the redacted receipt path plus failure stage. ✅ 15m (started: 2026-05-13T14:53:41Z → completed: 2026-05-13T15:08:44Z; evidence: evidence/push-boundary.md + archived dogfood-full.md)
- [x] [depends:reproduce-timeout] Add push-stage sub-boundary instrumentation and bounded failure categories to dogfood receipts/logs. ✅ 15m (started: 2026-05-13T14:53:41Z → completed: 2026-05-13T15:08:44Z; evidence: `push:*` operations and `push_timeout` tests)
- [x] [depends:push-instrumentation] Add a focused local dogfood push/CI-trigger acceptance check that is cheaper than `dogfood-local full`. ✅ 15m (started: 2026-05-13T14:53:41Z → completed: 2026-05-13T15:08:44Z; evidence: `push-check` subcommand test)
- [ ] [depends:focused-push-check] Verify the focused check and rerun or reclassify `dogfood-local -- full` with the improved boundary evidence.
- [ ] [depends:dogfood-rerun] Run `openspec validate fix-dogfood-local-push-timeout --strict --json` and the smallest relevant Rust/Nix checks before completion.
