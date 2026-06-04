# Tasks: iroh-sealed-repro-exchange

- [x] [serial] r[molten.transport.iroh_sealed_repro_exchange.publish] Add publish flow for sealed bundles and receipt chains over Iroh blobs.
- [x] [serial] r[molten.transport.iroh_sealed_repro_exchange.fetch] Add fetch flow that verifies bundle refs before import or unpack.
- [x] [serial] r[molten.transport.iroh_sealed_repro_exchange.exchange_receipt] Define canonical publish/fetch exchange receipts bound to node, peer, blob, bundle, and verification refs.
- [x] [serial] r[molten.transport.iroh_sealed_repro_exchange.ledger_import] Integrate verified fetches with the local evidence ledger.
- [x] [parallel] r[molten.transport.iroh_sealed_repro_exchange.confidentiality] Preserve redaction/encrypted-ref/reveal policies across publish and fetch.
- [x] [parallel] r[molten.transport.iroh_sealed_repro_exchange.tests] Add tests for wrong blob content, stale tickets, missing receipts, unknown peers, tampered bundles, and unauthorized reveal attempts.
