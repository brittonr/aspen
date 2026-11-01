## ADDED Requirements

### Requirement: Hosted bundle codec distinguishes labels from content identities

r[molten.system_extension.hosted_codec] The service-host bundle codec MUST validate content references only for fields that carry content identities, MUST admit schema references as contract labels, and MUST NOT treat numeric reference-collection bounds as references.

#### Scenario: Fabric schema labels round-trip

- GIVEN a bundle whose admitted manifest names its port schemas by contract label
- WHEN the bundle is encoded and decoded
- THEN decoding succeeds and the schema labels are preserved.

#### Scenario: A label in a content position is still refused

- GIVEN a field that carries a content identity, such as `manifest_ref`
- WHEN its value is a contract label rather than a BLAKE3 reference
- THEN decoding refuses it.
