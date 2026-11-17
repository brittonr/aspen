#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
pub(super) fn collect(
    state: &super::super::inference::State,
    environment: &mut noble_kernel::contracts::Env,
    definitions: &[super::super::Named],
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<noble_kernel::untrusted::Expected>, crate::Diagnostic> {
    let mut expected = alloc::vec::Vec::with_capacity(state.bodies.len());
    let mut at = 0usize;
    let mut failure = None;
    while at < state.bodies.len() {
        match install_interface(
            &state.bodies[at],
            &state.arena,
            environment,
            definitions,
            meter,
        ) {
            Ok(mut interface) => {
                if at == 0 {
                    if let Some(logical) = &state.logical_inputs {
                        interface.stack_in = logical.clone();
                    }
                }
                expected.push(interface);
            }
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
        at += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(expected),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn install_interface(
    body: &super::super::inference::Body,
    arena: &crate::inference::Arena,
    environment: &mut noble_kernel::contracts::Env,
    definitions: &[super::super::Named],
    meter: &mut crate::Meter,
) -> Result<noble_kernel::untrusted::Expected, crate::Diagnostic> {
    let interface = attempt!(super::materialization::interface(arena, body, meter));
    if body.identity.is_some() {
        // Each signed source owner retains the original rank-1 Scheme in
        // the concrete Env. Its fresh, exact owner Inst is materialized
        // separately from this body's source-derived variables and checked
        // against the body's concrete expected interface by the kernel.
        let is_signed = body
            .nodes
            .iter()
            .any(|node| matches!(node.kind, super::super::inference::DraftKind::SignedSelf(_)));
        let scheme = if is_signed {
            let source = body
                .owner_slot
                .and_then(|slot| usize::try_from(slot).ok())
                .and_then(|slot| definitions.get(slot))
                .ok_or_else(|| {
                    crate::invalid(body.span, "signed body has no original source owner")
                })?;
            if Some(source.identity) != body.identity
                || source.owner != body.owner
                || source.signed.is_none()
            {
                return Err(crate::invalid(
                    body.span,
                    "signed body differs from original source owner",
                ));
            }
            source
                .signature
                .as_ref()
                .ok_or_else(|| {
                    crate::invalid(body.span, "signed body has no original rank-1 signature")
                })?
                .clone()
        } else {
            attempt!(super::contracts::scheme(
                &interface,
                environment,
                body.span,
                meter
            ))
        };
        let deps = attempt!(super::materialization::dependencies(body, meter));
        environment.defs.reserve(1);
        environment.kinds.reserve(1);
        environment.deps.reserve(1);
        environment.defs.push(scheme);
        environment
            .kinds
            .push(noble_kernel::contracts::Behavior::Named);
        environment.deps.push(deps);
        environment.definition_owners.push(body.owner);
    }
    Ok(interface)
}
