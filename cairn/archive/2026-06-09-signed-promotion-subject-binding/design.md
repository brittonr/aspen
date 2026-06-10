# Design: signed-promotion-subject-binding

The `dogfood-local-node` check captures the `receipt=...` ref printed by `molten dogfood release-promote` and requires the subsequent `molten receipts verify-signed` invocation for `release-promotion-gate.signed.preserves` to use that value as `--subject-ref`.

If the signed envelope signs any other receipt, verification fails and the Nix check fails. The binding remains evidence-only: a matching signature subject ref does not grant release publication authority or replace any subsystem gate.
