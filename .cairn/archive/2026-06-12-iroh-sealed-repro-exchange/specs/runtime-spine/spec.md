# Runtime Spine Delta: Iroh sealed repro exchange

### Requirement: Published bundles remain content-addressed evidence
r[molten.transport.iroh_sealed_repro_exchange.publish] Publishing a sealed repro bundle over Iroh MUST preserve canonical bundle and receipt refs.

#### Scenario: Publish emits exchange receipt
- GIVEN a verified sealed repro bundle
- WHEN it is published through Iroh blobs
- THEN Molten emits a repro exchange receipt naming the blob ticket/ref, bundle ref, local node identity, and included receipt refs

### Requirement: Fetch verifies before import or unpack
r[molten.transport.iroh_sealed_repro_exchange.fetch] Fetched repro bundles MUST be verified before they can be imported into the evidence ledger, unpacked, or used as gate evidence.

#### Scenario: Blob content does not match bundle ref
- GIVEN an Iroh fetch returns bytes that do not hash to the advertised bundle ref
- WHEN fetch verification runs
- THEN the fetch fails closed and emits a diagnostic exchange receipt

#### Scenario: Tampered fetched bundle is not imported
- GIVEN a fetched bundle whose embedded gate receipt or redaction gate has been tampered
- WHEN fetch verification runs
- THEN the bundle is rejected before ledger import or unpack

### Requirement: Transport does not grant trust or reveal rights
r[molten.transport.iroh_sealed_repro_exchange.confidentiality] Iroh transport success MUST NOT imply gate acceptance, signer trust, or private reveal authority.

#### Scenario: Unauthorized encrypted content stays opaque
- GIVEN a fetched encrypted-private bundle without reveal authority
- WHEN unpack runs
- THEN private content remains opaque
- AND no reveal receipt is emitted

#### Scenario: Unknown peer can provide bytes but not trust
- GIVEN a bundle fetched from an unknown peer
- WHEN local policy requires trusted signed receipts
- THEN transport succeeds but gate acceptance fails until trust evidence validates
