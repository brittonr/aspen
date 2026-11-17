//! Candidate declaration and exposure of local nominal schemas.

pub(super) struct Candidate<'a> {
    pub(super) schema: &'a crate::source::declared::Schema,
    pub(super) exports: &'a [alloc::string::String],
    pub(super) shape: noble_kernel::types::NominalShape,
    pub(super) identity: u64,
    pub(super) ordinal: usize,
}

pub(super) fn declare(
    session: crate::source::declared::ModuleSession,
    candidate: Candidate<'_>,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        noble_kernel::types::Ty,
        noble_kernel::contracts::NominalOps,
    ),
    crate::source::Error,
> {
    let is_exported = candidate.exports.contains(&candidate.schema.name);
    let public = super::types::public_operations(candidate.schema, candidate.exports, is_exported);
    let environment =
        &attempt!(session
            .source
            .declared
            .as_ref()
            .ok_or_else(|| crate::source::declared::error(
                crate::source::Stage::Check,
                "missing declared context"
            )))
        .environment;
    attempt!(check_exposure(&candidate.shape, public, environment));
    commit(session, candidate, public, is_exported)
}

fn commit(
    session: crate::source::declared::ModuleSession,
    candidate: Candidate<'_>,
    public: [bool; 2],
    is_exported: bool,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        noble_kernel::types::Ty,
        noble_kernel::contracts::NominalOps,
    ),
    crate::source::Error,
> {
    let id = noble_kernel::types::NominalTypeId {
        module: candidate.identity,
        ordinal: candidate.ordinal as u32,
    };
    let declaration = noble_kernel::contracts::NominalDecl {
        id,
        shape: candidate.shape.clone(),
        public,
        exported: is_exported,
    };
    let (session, ops) = attempt!(install_nominal(session, declaration));
    let ty = noble_kernel::types::Ty::Nominal(id, alloc::boxed::Box::new(candidate.shape));
    Ok((session, ty, ops))
}

fn install_nominal(
    mut session: crate::source::declared::ModuleSession,
    declaration: noble_kernel::contracts::NominalDecl,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        noble_kernel::contracts::NominalOps,
    ),
    crate::source::Error,
> {
    let Some(context) = session.source.declared.as_mut() else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "missing declared context",
        ));
    };
    let environment = core::mem::take(&mut context.environment);
    let (environment, ops) = attempt!(environment.declare_nominal(declaration).map_err(|_| {
        crate::source::declared::error(
            crate::source::Stage::Check,
            "invalid or recursive nominal schema",
        )
    }));
    context.environment = environment;
    Ok((session, ops))
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 exposed_payload and declared::error prevent const on both pinned Rust compilers; reassess const payload traversal and diagnostics."
)]
fn check_exposure(
    shape: &noble_kernel::types::NominalShape,
    public: [bool; 2],
    environment: &noble_kernel::contracts::Env,
) -> Result<(), crate::source::Error> {
    let exposed = match shape {
        noble_kernel::types::NominalShape::Opaque(base) => {
            let is_allowed = crate::source::declared::exposed_payload(base, environment);
            [is_allowed, is_allowed]
        }
        noble_kernel::types::NominalShape::Variant(left, right) => [
            crate::source::declared::exposed_payload(left, environment),
            crate::source::declared::exposed_payload(right, environment),
        ],
    };
    if (public[0] && !exposed[0]) || (public[1] && !exposed[1]) {
        return Err(crate::source::declared::error(
            crate::source::Stage::Link,
            "exported constructor exposes private or unsupported payload",
        ));
    }
    Ok(())
}

pub(in super::super) fn nominal_public(
    session: &crate::source::declared::ModuleSession,
    ty: &noble_kernel::types::Ty,
) -> Result<[bool; 2], crate::source::Error> {
    let noble_kernel::types::Ty::Nominal(id, _) = ty else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "schema resolved to nonnominal type",
        ));
    };
    public_for_id(session, *id)
}

fn public_for_id(
    session: &crate::source::declared::ModuleSession,
    id: noble_kernel::types::NominalTypeId,
) -> Result<[bool; 2], crate::source::Error> {
    if let Some(context) = session.source.declared.as_ref() {
        if let Some(declaration) = context.environment.nominal(id) {
            return Ok(declaration.public);
        }
    }
    Err(crate::source::declared::error(
        crate::source::Stage::Check,
        "unregistered nominal schema",
    ))
}
