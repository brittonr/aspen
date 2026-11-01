## Context

`codec::wire::field` decides whether a field's value must be a content reference from the field
name alone. Two conventions collide: host/domain references end in `_ref`/`_refs` and are
BLAKE3 content identities, while fabric schema references end in `schema_ref`/`schema_refs` and
are contract labels such as `molten.fabric.transport.command.v1` or
`molten.collaboration-scope.staged-command.v1`. Numeric bounds named `max_*_refs` are counts,
not references, which the codec already excludes.

## Decisions

### Decision: schema labels are excluded from the content-reference check

**Choice:** Treat a field as a content reference only when it ends in `_ref`/`_refs`, does not
start with `max_`, and does not contain `schema_ref`.

**Rationale:** The three families are distinct by contract: content identities, counts, and
schema labels. Excluding the label family keeps every actual identity checked (the regression
asserts `manifest_ref` still rejects a label) while making a real admitted manifest decodable.

## Risks / Trade-offs

- Excluding `schema_ref` fields would hide a genuine misuse of that name for a content
  identity. The fabric and collaboration schemas define those names as labels, and the codec
  still validates the record shape and the remaining reference fields.
