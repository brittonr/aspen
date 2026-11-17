//! Preflight checks for all host-provided environment schemes.

/// Reject any environment definition whose scheme is not in the accepted form.
pub(super) fn validate(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
) -> Result<(), super::Fail> {
    let ctx = super::parts::Ctx {
        env,
        request,
        signed_owner: None,
    };
    let mut index = 0;
    while index < env.defs.len() {
        attempt!(validate_scheme(&ctx, index));
        index += 1;
    }
    Ok(())
}

/// Keep scheme-form rejection ahead of type-pattern and identity checks.
fn validate_scheme(ctx: &super::parts::Ctx, index: usize) -> Result<(), super::Fail> {
    let scheme = &ctx.env.defs[index];
    if scheme.validate().is_err() {
        return Err(super::Fail::Unsupported(
            crate::untrusted::UnsupportedKind::SchemeForm,
        ));
    }
    if has_invalid_pattern(scheme, ctx.env) {
        return Err(invalid_scheme_pattern(index));
    }
    Ok(())
}

fn invalid_scheme_pattern(index: usize) -> super::Fail {
    let Ok(index) = u32::try_from(index) else {
        return super::Fail::Exhausted(crate::untrusted::LimitKind::Nodes);
    };
    super::parts::invalid_without_stacks(
        super::parts::site(None, Some(crate::contracts::Definition(index))),
        crate::untrusted::Constraint::InvalidType,
    )
}

fn has_invalid_pattern(scheme: &crate::words::Scheme, env: &crate::contracts::Env) -> bool {
    has_invalid_stack_pattern(&scheme.stack_in, env)
        || has_invalid_stack_pattern(&scheme.stack_out, env)
}

fn has_invalid_stack_pattern(
    patterns: &[crate::shapes::Pattern],
    env: &crate::contracts::Env,
) -> bool {
    let mut index = 0;
    let mut is_invalid = false;
    while index < patterns.len() {
        if !env.valid_pattern(&patterns[index], 512) {
            is_invalid = true;
            break;
        }
        index += 1;
    }
    is_invalid
}
