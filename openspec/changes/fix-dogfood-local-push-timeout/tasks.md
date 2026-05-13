## Tasks

- [ ] [serial] Reproduce the local dogfood push timeout with a large writable cluster dir and preserve the redacted receipt path plus failure stage.
- [ ] [depends:reproduce-timeout] Add push-stage sub-boundary instrumentation and bounded failure categories to dogfood receipts/logs.
- [ ] [depends:push-instrumentation] Add a focused local dogfood push/CI-trigger acceptance check that is cheaper than `dogfood-local full`.
- [ ] [depends:focused-push-check] Verify the focused check and rerun or reclassify `dogfood-local -- full` with the improved boundary evidence.
- [ ] [depends:dogfood-rerun] Run `openspec validate fix-dogfood-local-push-timeout --strict --json` and the smallest relevant Rust/Nix checks before completion.
