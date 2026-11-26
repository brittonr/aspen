/// Registers the complete sandbox primitive table, in a fixed order.
fn register_sandbox(engine: &mut Engine, meter: &Meter, expected_operation: &str, allowed_hostcalls: &[String]) {
    register_metering_primitives(engine, meter, expected_operation, allowed_hostcalls);
    register_string_primitives(engine, meter);
    register_value_primitives(engine, meter);
}

/// Fuel ticks, closure charges, and the admitted hostcall.
fn register_metering_primitives(
    engine: &mut Engine,
    meter: &Meter,
    expected_operation: &str,
    allowed_hostcalls: &[String],
) {
    let tick = meter.clone();
    engine.register_fn(FUEL_TICK, move || -> SteelResult<bool> { tick.charge(1, 0).map(|()| true) });

    let closure = meter.clone();
    engine.register_fn(CLOSURE, move |bytes: isize, value: SteelValue| -> SteelResult<SteelValue> {
        closure.charge_allocation(1, u64::try_from(bytes).ok())?;
        Ok(value)
    });

    let hostcall = meter.clone();
    let expected_operation = expected_operation.to_string();
    let allowed_hostcalls = allowed_hostcalls.to_vec();
    engine.register_fn(PRIM_HOSTCALL, move |operation: String, envelope: String| -> SteelResult<String> {
        hostcall.admit_hostcall()?;
        let response = if operation == expected_operation && allowed_hostcalls.contains(&operation) {
            format!("<steel-hostcall-response \"pass\" \"{}\">", escape_steel_string(&envelope))
        } else {
            format!("<steel-hostcall-response \"deny\" \"{}\">", escape_steel_string(&operation))
        };
        hostcall.charge_string(response.len())?;
        Ok(response)
    });
}

fn register_string_primitives(engine: &mut Engine, meter: &Meter) {
    let append = meter.clone();
    engine.register_fn(PRIM_STRING_APPEND, move |left: String, right: String| -> SteelResult<String> {
        append.charge_string(left.len().saturating_add(right.len()))?;
        Ok(left + &right)
    });
    let length = meter.clone();
    engine.register_fn(PRIM_STRING_LENGTH, move |text: String| -> SteelResult<isize> {
        length.charge_scalar()?;
        isize::try_from(text.chars().count()).map_err(|_| integer_overflow())
    });
    let slice = meter.clone();
    engine.register_fn(PRIM_SUBSTRING, move |text: String, start: isize, end: isize| -> SteelResult<String> {
        let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
            return Err(sandbox_error("substring bounds must be non-negative"));
        };
        if start > end || end > text.chars().count() {
            return Err(sandbox_error("substring bounds are outside the string"));
        }
        let result = text.chars().skip(start).take(end - start).collect::<String>();
        slice.charge_string(result.len())?;
        Ok(result)
    });
    let same_text = meter.clone();
    engine.register_fn(PRIM_STRING_EQUAL, move |left: String, right: String| -> SteelResult<bool> {
        same_text.charge_scalar()?;
        Ok(left == right)
    });
}

fn register_value_primitives(engine: &mut Engine, meter: &Meter) {
    let equal = meter.clone();
    engine.register_fn(PRIM_EQUAL, move |left: SteelValue, right: SteelValue| -> SteelResult<bool> {
        equal.charge_scalar()?;
        Ok(left == right)
    });
    let negate = meter.clone();
    engine.register_fn(PRIM_NOT, move |value: SteelValue| -> SteelResult<bool> {
        negate.charge_scalar()?;
        Ok(matches!(value, SteelValue::BoolV(false)))
    });
    register_integer(engine, meter, PRIM_ADD, isize::checked_add);
    register_integer(engine, meter, PRIM_SUBTRACT, isize::checked_sub);
    register_integer(engine, meter, PRIM_MULTIPLY, isize::checked_mul);
    let equal_integer = meter.clone();
    engine.register_fn(PRIM_INTEGER_EQUAL, move |left: isize, right: isize| -> SteelResult<bool> {
        equal_integer.charge_scalar()?;
        Ok(left == right)
    });
    let less = meter.clone();
    engine.register_fn(PRIM_LESS, move |left: isize, right: isize| -> SteelResult<bool> {
        less.charge_scalar()?;
        Ok(left < right)
    });
}

fn register_integer(
    engine: &mut Engine,
    meter: &Meter,
    name: &'static str,
    operation: fn(isize, isize) -> Option<isize>,
) {
    let meter = meter.clone();
    engine.register_fn(name, move |left: isize, right: isize| -> SteelResult<isize> {
        meter.charge_scalar()?;
        operation(left, right).ok_or_else(integer_overflow)
    });
}

fn finish_value(input: FinishInput<'_>) -> Result<PreservesValue> {
    let output_value = string(&input.execution.output_text);
    let input_ref = canonical_hash(input.actor_input)?;
    let output_ref = canonical_hash(&output_value)?;
    let hostcalls = vec![input.operation.to_string()];

    Ok(receipt_value(RunReceipt {
        actor_id: input.actor_id,
        source_ref: input.source_ref,
        callable: input.callable,
        operation: input.operation,
        input_ref: &input_ref,
        output_ref: &output_ref,
        hostcalls: &hostcalls,
        resource_limits: ResourceReceipt {
            fuel_limit: STEEL_FUEL_LIMIT,
            fuel_remaining: input.execution.fuel_remaining,
            source_bytes: input.source_bytes,
            input_bytes: input.input_text.len() as u64,
            output_bytes: input.execution.output_text.len() as u64,
            hostcall_limit: STEEL_MAX_HOSTCALLS,
            hostcall_count: input.execution.hostcall_count,
        },
    }))
}

fn escape_steel_string(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}
