## 1. Classify the affected encoders

- [ ] 1.1 Inventory ticket and signed-identity serializers that currently use `unwrap_or_default()` or equivalent silent fallback
- [ ] 1.2 Split them into two groups: APIs that can return `Result` and trait-imposed `to_bytes() -> Vec<u8>` implementations that cannot
- [ ] 1.3 Confirm which of those payloads are externally shared or authority-bearing and prioritize them first

## 2. Fix fallible wrapper APIs

- [ ] 2.1 Change `AutomergeSyncTicket` construction and serialization helpers to surface encoding failures explicitly
- [ ] 2.2 Update affected callers and tests to handle the new `Result`-returning API
- [ ] 2.3 Audit other wrapper types around capability tokens or signed identities for the same silent-fallback pattern

## 3. Fix trait-constrained ticket encoders

- [ ] 3.1 Replace silent default-byte fallbacks in `Ticket::to_bytes()` implementations with fail-fast handling and invariant documentation
- [ ] 3.2 Review client ticket and signed cluster ticket implementations to ensure they never emit empty payloads on serializer failure
- [ ] 3.3 Apply the same rule to other externally shared identity/ticket encoders that cannot return `Result`

## 4. Add regression coverage

- [ ] 4.1 Add a test showing oversized capability-token input is rejected at `AutomergeSyncTicket` creation time
- [ ] 4.2 Add round-trip tests for client and signed ticket encoders after the fallback removal
- [ ] 4.3 Add a targeted test or assertion that no empty payload is produced on the old silent-fallback path

## 5. Verify the resulting API surface

- [ ] 5.1 Run crate tests for the touched ticket and identity modules
- [ ] 5.2 Check any CLI or handler code that mints these tickets still reports actionable errors to the caller
- [ ] 5.3 Document any remaining internal-only serializers left unchanged and why they are lower risk
