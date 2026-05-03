# Auto-publish live dogfood smoke

Command:

```bash
rm -rf /tmp/aspen-dogfood-autopub /tmp/aspen-dogfood-autopub-receipts
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood-autopub full --leave-running
```

Result: exit 0.

Observed safe output excerpts:

```text
✅ Verification passed
🧾 Published final dogfood receipt to cluster key dogfood/receipts/dogfood-20260503T201147Z.json
🧾 Dogfood receipt: /tmp/aspen-dogfood-autopub-receipts/dogfood-20260503T201147Z.json
📌 Dogfood cluster left running for evidence inspection
```

Readback command:

```bash
cargo run -q -p aspen-dogfood -- --cluster-dir /tmp/aspen-dogfood-autopub receipts cluster-show dogfood-20260503T201147Z --json
```

Parsed cluster-backed receipt:

```text
schema aspen.dogfood.run-receipt.v1
run_id dogfood-20260503T201147Z
stages start:succeeded,push:succeeded,build:succeeded,deploy:succeeded,verify:succeeded,publish_receipt:succeeded
failure None
```

Cleanup:

```bash
cargo run -q -p aspen-dogfood -- --cluster-dir /tmp/aspen-dogfood-autopub stop
```

No cluster ticket contents were captured or preserved.
