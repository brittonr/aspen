mod patterns;

pub(super) fn scheme(
    expected: &noble_kernel::untrusted::Expected,
    environment: &noble_kernel::contracts::Env,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::words::Scheme, crate::Diagnostic> {
    let stack_in = attempt!(stack(&expected.stack_in, environment, span, meter));
    let stack_out = attempt!(stack(&expected.stack_out, environment, span, meter));
    let effects = attempt!(effect_slots(&expected.allowed_effects, span, meter));
    Ok(noble_kernel::words::Scheme {
        var_kinds: alloc::vec::Vec::new(),
        stack_in,
        stack_out,
        effects,
    })
}

fn stack(
    types: &[noble_kernel::types::Ty],
    environment: &noble_kernel::contracts::Env,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<noble_kernel::shapes::Pattern>, crate::Diagnostic> {
    let mut patterns = alloc::vec::Vec::with_capacity(types.len());
    let mut at = 0usize;
    let mut failure = None;
    while at < types.len() {
        match patterns::convert(&types[at], environment, span, meter) {
            Ok(pattern) => patterns.push(pattern),
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
        at += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(patterns),
    }
}

fn effect_slots(
    effects: &noble_kernel::types::EffSet,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<noble_kernel::shapes::EffectSlot>, crate::Diagnostic> {
    let effects = effects.as_slice();
    let mut slots = alloc::vec::Vec::with_capacity(effects.len());
    let mut at = 0usize;
    let mut failure = None;
    while let Some(effect) = effects.get(at).copied() {
        if let Err(problem) = meter.node(span) {
            failure = Some(problem);
            break;
        }
        slots.push(noble_kernel::shapes::EffectSlot::Effect(effect));
        at += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(slots),
    }
}
