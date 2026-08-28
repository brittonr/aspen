# Design: Materialized native effect completions

## Boundary

Aspen owns provider routing mechanics and callback materialization. It does not own provider or workload terminal meaning.

The provider adapter returns a `PortEffectOutput`. The native service validates the output before it creates a callback-visible completion.

## Contract shape

`PortEffectOutput` gains `materialized_output: Option<NativeCallbackValue>` beside its existing schema and reference.

The option keeps generic reference-only effect ports compatible. A native profile with `requires_materialized_values = true` requires `Some`.

The exact value must satisfy all checks:

- its BLAKE3 reference is valid;
- its bytes match that reference;
- its reference equals `output_ref`; and
- its byte count fits `max_materialized_value_bytes`.

The canonical completion becomes `system-extension-effect-completion-v2` with schema `molten.system-extension.effect-completion.v2`.

The record adds one `output-value` field. The field uses the existing `some(native-callback-value-v2(...))` or `none` shape.

## Effect ordering

The native service keeps this order:

1. Commit the provider effect intent.
2. Invoke the admitted provider adapter once.
3. Record the provider terminal reference.
4. Validate the optional output value.
5. Build and publish the canonical completion.
6. Invoke the generation-fenced `message` callback.

If value validation fails after provider return, the effect remains terminal. The service returns a typed failure and does not retry the provider.

## Functional core and shell

Canonical record construction and identity checks remain deterministic.

The effect delegate, journal writes, value publication, and callback process stay in the imperative shell.

No workload type or provider-specific schema enters Aspen core.

## Tests

The positive separate-process test returns an exact materialized provider value. It verifies the canonical completion retains those bytes.

The negative test returns only a reference under a materializing native profile. It verifies callback delivery does not run and retry is not permitted.

Identity mismatch and bound checks remain covered by the shared native value admission path.

## Non-claims

Materialized output proves only exact bounded bytes supplied by the selected adapter. It does not prove provider correctness, terminal truth, durability, or application meaning.
