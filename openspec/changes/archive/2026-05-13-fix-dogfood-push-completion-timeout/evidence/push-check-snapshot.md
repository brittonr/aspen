# Dogfood push-completion snapshot evidence

## Focused verification

Command:

```bash
ASPEN_DOGFOOD_GIT_PUSH_TIMEOUT_SECS=120 \
  nix run .#dogfood-local -- --cluster-dir /home/brittonr/data/aspen-dogfood-push-check push-check \
  2>&1 | tee target/runtime-proof/dogfood-push-check-snapshot.log
```

Result: succeeded.

Receipt:

- `/home/brittonr/data/aspen-dogfood-push-check-receipts/dogfood-20260513T171735Z.json`
- `run_id`: `dogfood-20260513T171735Z`
- stages:
  - `start`: `succeeded`, elapsed `8714 ms`
  - `push`: `succeeded`, elapsed `22448 ms`
  - `stop`: `succeeded`, elapsed `2195 ms`

Key log boundary:

```text
2026-05-13T17:17:44.722073Z  INFO aspen_dogfood::forge:   repo created (id: 3c2e8784eda08e20)
2026-05-13T17:17:45.726734Z  INFO aspen_dogfood::forge:   CI watch registered for repo 3c2e8784eda08e20
2026-05-13T17:17:48.011130Z  INFO aspen_dogfood::forge:   prepared bounded source snapshot at /home/brittonr/data/aspen-dogfood-push-check/source-snapshot
2026-05-13T17:17:48.014068Z  INFO aspen_dogfood::forge:   git push aspen-dogfood main from bounded source snapshot (--no-verify)...
2026-05-13T17:18:07.156826Z  INFO aspen_dogfood: ✅ Source pushed to Forge
2026-05-13T17:18:09.352837Z  INFO aspen_dogfood: 🧾 Dogfood receipt: /home/brittonr/data/aspen-dogfood-push-check-receipts/dogfood-20260513T171735Z.json
```

Conclusion: the previous `push:push_completion` / `push_timeout` product blocker is cleared for the focused local dogfood rail. The push stage now uses a bounded single-commit current-source snapshot, still travels through the real `git-remote-aspen` Forge path, and completes before the 120s push timeout.
