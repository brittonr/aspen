# Bound resource deletion-gate facts

## Implemented and observed

- [x] [serial] Identify six duplicated `unbounded_collection_growth` append locations in one public deletion-gate producer, verify the ten hardening locations have input bounds, and confirm no product CLI invokes this resource function. r[molten.lifecycle.gc_cleanup_gates.bounded_facts]
- [x] [serial] Reproduce the public Rust API admitting a 257th pin and a 4097-byte pin fact with a disposable compiled external consumer; observe a valid `blocked` decision with 256 pins. r[molten.lifecycle.gc_cleanup_gates.bounded_facts]
- [x] [serial] Preflight all seven collection counts and all fact byte lengths before any deletion diagnostic is constructed; reuse established 64-owner/finalizer, 256-evidence, and 4096-byte limits without changing the input or decision DTOs. r[molten.lifecycle.gc_cleanup_gates.bounded_facts]
- [x] [serial] Add owner/pin true-boundary and fact-byte tests showing the exact limits retain blocked/deletion-ready transitions while one extra fact denies without a decision. r[molten.lifecycle.gc_cleanup_gates.bounded_facts]
- [x] [serial] Observe the compiled external consumer after the guard returning `InvalidHarness` for the 257th pin and 4097-byte fact while 256 pins still yield `blocked`. r[molten.lifecycle.gc_cleanup_gates.bounded_facts]

## Scope and non-claims

The Cairn requirement remains change-local pending independently authorized review; this package does not sync accepted specs, archive, commit, change pins, or waive Octet. The resource producer's original six static push-site warnings remain in a warning-only owner probe, despite the actual early runtime bound; a warning-only result is not strict-ci acceptance.
