mod names;
pub(super) mod nominals;
mod types;

#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the type name is the domain term at its use sites and renaming would churn the extracted subject"
)]
pub(super) enum ResolvedSchema {
    Concrete(noble_kernel::types::Ty, noble_kernel::contracts::NominalOps),
    Generic(
        noble_kernel::types::NominalTypeId,
        noble_kernel::contracts::NominalOps,
    ),
}

pub(super) type Resolved = alloc::vec::Vec<Option<ResolvedSchema>>;

pub(super) fn resolve(
    session: crate::source::declared::ModuleSession,
    collected: &super::collection::Collected,
    identity: u64,
    limits: crate::Limits,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        Resolved,
        crate::Meter,
    ),
    crate::source::Error,
> {
    let resolution = attempt!(Resolution::start(session, collected, identity, limits));
    attempt!(resolution.run()).finish()
}

struct Resolution<'a> {
    session: Option<crate::source::declared::ModuleSession>,
    local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
    local_generics: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::NominalTypeId)>,
    resolved: Resolved,
    type_nodes: usize,
    meter: crate::Meter,
    identity: u64,
    limits: crate::Limits,
    collected: &'a super::collection::Collected,
}

impl<'a> Resolution<'a> {
    fn run(mut self) -> Result<Self, crate::source::Error> {
        let mut count = 0;
        let mut failure = None;
        while count < self.collected.schemas.len() && failure.is_none() {
            let (next, progress) = self.advance();
            self = next;
            match progress {
                Ok(completed) if completed != 0 => count += completed,
                Ok(_) => {
                    failure = Some(crate::source::declared::error(
                        crate::source::Stage::Resolve,
                        "unknown or cyclic nominal payload type",
                    ))
                }
                Err(problem) => failure = Some(problem),
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(self),
        }
    }
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0277/E0658 Option::ok_or_else and E0493 owned Resolution/Result destructors prevent const on both pinned Rust compilers; reassess const ownership APIs."
    )]
    fn finish(
        mut self,
    ) -> Result<
        (
            crate::source::declared::ModuleSession,
            Resolved,
            crate::Meter,
        ),
        crate::source::Error,
    > {
        let session = attempt!(self.session.take().ok_or_else(|| {
            crate::source::declared::error(crate::source::Stage::Check, "missing module session")
        }));
        Ok((session, self.resolved, self.meter))
    }

    fn start(
        session: crate::source::declared::ModuleSession,
        collected: &'a super::collection::Collected,
        identity: u64,
        limits: crate::Limits,
    ) -> Result<Self, crate::source::Error> {
        let mut resolved: Resolved = alloc::vec::Vec::with_capacity(collected.schemas.len());
        resolved.resize_with(collected.schemas.len(), || None);
        let local_types = attempt!(names::available(&session, collected.schemas.len()));
        let meter = crate::Meter::pooled(limits, session.source.request_work());
        Ok(Self {
            local_types,
            local_generics: alloc::vec::Vec::new(),
            session: Some(session),
            resolved,
            type_nodes: 0,
            meter,
            identity,
            limits,
            collected,
        })
    }

    fn advance(mut self) -> (Self, Result<usize, crate::source::Error>) {
        let mut completed = 0;
        let mut ordinal = 0;
        let count = self.collected.schemas.len();
        let mut failure = None;
        while ordinal < count && failure.is_none() {
            let (next, progress) = self.advance_one(ordinal);
            self = next;
            match progress {
                Ok(true) => completed += 1,
                Ok(false) => {}
                Err(problem) => failure = Some(problem),
            }
            ordinal += 1;
        }
        let result = match failure {
            Some(problem) => Err(problem),
            None => Ok(completed),
        };
        (self, result)
    }
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0277/E0658 Vec Index and E0015 parse_shape plus E0493 owned results prevent const on both pinned Rust compilers; reassess const type resolution."
    )]
    fn advance_one(mut self, ordinal: usize) -> (Self, Result<bool, crate::source::Error>) {
        if self.resolved[ordinal].is_some() {
            return (self, Ok(false));
        }
        let schema = &self.collected.schemas[ordinal];
        if let crate::source::declared::SchemaKind::Variant {
            parameters,
            left_type,
            right_type,
            ..
        } = &schema.kind
        {
            if !parameters.is_empty() {
                return self.declare_generic(ordinal, parameters, left_type, right_type);
            }
        }
        let environment = self
            .session
            .as_ref()
            .and_then(|session| session.source.declared.as_ref())
            .map(|context| &context.environment);
        let (meter, parsed) = types::parse_shape(
            schema,
            &self.local_types,
            &self.local_generics,
            environment,
            self.meter,
        );
        self.meter = meter;
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(problem) => return (self, Err(problem)),
        };
        if let Some((shape, nodes)) = parsed {
            self.type_nodes = self.type_nodes.saturating_add(nodes).saturating_add(1);
            if usize::try_from(self.limits.nodes).is_ok_and(|limit| self.type_nodes > limit) {
                return (
                    self,
                    Err(crate::source::declared::error(
                        crate::source::Stage::Check,
                        "module type constructor count exceeded",
                    )),
                );
            }
            let candidate = nominals::Candidate {
                schema,
                exports: &self.collected.exports,
                shape,
                identity: self.identity,
                ordinal,
            };
            let session = match self.session.take().ok_or_else(|| {
                crate::source::declared::error(
                    crate::source::Stage::Check,
                    "missing module session",
                )
            }) {
                Ok(session) => session,
                Err(problem) => return (self, Err(problem)),
            };
            let (session, ty, ops) = match nominals::declare(session, candidate) {
                Ok(result) => result,
                Err(problem) => return (self, Err(problem)),
            };
            self.session = Some(session);
            self.local_types.push((schema.name.clone(), ty.clone()));
            self.resolved[ordinal] = Some(ResolvedSchema::Concrete(ty, ops));
            return (self, Ok(true));
        }
        (self, Ok(false))
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
    )]
    fn declare_generic(
        mut self,
        ordinal: usize,
        parameters: &[alloc::string::String],
        left_type: &str,
        right_type: &str,
    ) -> (Self, Result<bool, crate::source::Error>) {
        let parameter_index = |spelling: &str| -> Result<u8, crate::source::Error> {
            parameters
                .iter()
                .position(|name| name == spelling)
                .and_then(|index| u8::try_from(index).ok())
                .ok_or_else(|| {
                    crate::source::declared::error(
                        crate::source::Stage::Resolve,
                        "generic variant payload must name a declared type parameter",
                    )
                })
        };
        let left = match parameter_index(left_type) {
            Ok(index) => index,
            Err(problem) => return (self, Err(problem)),
        };
        let right = match parameter_index(right_type) {
            Ok(index) => index,
            Err(problem) => return (self, Err(problem)),
        };
        let payload_params = [left, right];
        let schema = &self.collected.schemas[ordinal];
        let is_exported = self.collected.exports.contains(&schema.name);
        let public = types::public_operations(schema, &self.collected.exports, is_exported);
        let id = noble_kernel::types::NominalTypeId {
            module: self.identity,
            ordinal: ordinal as u32,
        };
        let Some(session) = self.session.as_mut() else {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Check,
                    "missing module session",
                )),
            );
        };
        let Some(context) = session.source.declared.as_mut() else {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Check,
                    "missing declared context",
                )),
            );
        };
        let declaration = noble_kernel::contracts::GenericVariantDecl {
            id,
            payload_params,
            exported: is_exported,
            public,
        };
        let environment = core::mem::take(&mut context.environment);
        // As before, a refused declaration consumes the taken environment and
        // leaves its default in the context; the error aborts this resolution.
        let (environment, ops) = match environment.declare_generic_variant(declaration) {
            Ok(declared) => declared,
            Err(_) => {
                return (
                    self,
                    Err(crate::source::declared::error(
                        crate::source::Stage::Check,
                        "invalid generic variant schema",
                    )),
                );
            }
        };
        context.environment = environment;
        self.local_generics.push((schema.name.clone(), id));
        self.resolved[ordinal] = Some(ResolvedSchema::Generic(id, ops));
        (self, Ok(true))
    }
}
