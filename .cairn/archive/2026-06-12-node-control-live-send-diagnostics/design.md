# Design: Node Control Live Send Diagnostics

## Ticket binding guards

`control-ingress-live-send` accepts optional expected receiver node, topic, and endpoint guards. These are checked against the parsed `node-control-live-ticket-v1` before transport setup. A mismatch emits a deny `node-control-live-send-receipt-v1` with concrete diagnostics and a failing check label.

## Sender state-root evidence preflight

When `--state-root` is present, live send now checks the sender ledger for supplied peer bootstrap and authority refs before opening a live Iroh endpoint. Peer bootstrap refs must resolve to passing `node-control-live-peer-admission-v1` receipts whose ticket/node/topic/peer/freshness bind the outgoing envelope. Authority refs must resolve to `node-control-authority-grant-v1` grants that match peer, node, operation, scopes, epoch, expiry, and revocation requirements.

Missing or malformed peer-admission evidence adds a deterministic hint to run `live-ticket-import --peer-admission`. Missing or malformed authority evidence adds a deterministic hint to run `authority-grant-import`. These diagnostics deny before transport and are also reflected in receipt checks.

## Receipt checks

The existing live-send receipt shape is preserved. The checks sequence now includes labels for:

- receiver ticket expectation binding;
- receiver address availability;
- receiver address support/parsing;
- operation-id binding;
- sender state-root evidence;
- join-or-publish success.

These labels let runbook receipts and CLI users classify failures without depending only on free-form diagnostics.

## Transport failures

Join and publish failures continue to use retry receipts. The final send receipt records failed join/publish outcome through the transport success check, while retry receipts retain attempt-level diagnostics.
