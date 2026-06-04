# Tasks: sealed-repro-verify-unpack

- [x] [serial] r[molten.testing.sealed_repro_verify_unpack.verify_cli] Add `molten test repro verify` with canonical verification receipt output.
- [x] [serial] r[molten.testing.sealed_repro_verify_unpack.verify_receipt] Add `<repro-verify-receipt-v1 ...>` parsing, summaries, and pass checks.
- [x] [serial] r[molten.testing.sealed_repro_verify_unpack.unpack_cli] Add `molten test repro unpack` for verified sealed bundles.
- [x] [serial] r[molten.testing.sealed_repro_verify_unpack.fail_closed] Reject failure, unsealed, and tampered bundles from verify/unpack with canonical failure artifacts.
- [x] [parallel] r[molten.testing.sealed_repro_verify_unpack.tests] Add CLI/unit coverage for valid verify/unpack and failure-bundle rejection.
- [x] [parallel] r[molten.testing.sealed_repro_verify_unpack.docs] Update docs and CLI command listings.
