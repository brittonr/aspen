# Tasks: Reduce Aspen Tigerstyle Safety-Debt Noise

- [ ] Confirm the rollout order for `ignored_result`, `no_unwrap`, `no_panic`, `unchecked_narrowing`, and `unbounded_loop`
- [ ] Build a per-crate inventory for the first promoted family
- [ ] Fix or justify the highest-risk findings in that family without blanket allows
- [ ] Re-run the targeted tigerstyle check and capture the reduced baseline
- [ ] Repeat for the next family only after the current one has a bounded warning set
