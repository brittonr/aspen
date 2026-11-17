pub(super) struct Collected {
    pub schemas: alloc::vec::Vec<crate::source::declared::Schema>,
    pub definitions: alloc::vec::Vec<(
        alloc::string::String,
        alloc::vec::Vec<u8>,
        crate::source::Tree,
    )>,
    pub requirements: alloc::vec::Vec<(
        alloc::string::String,
        alloc::string::String,
        alloc::string::String,
    )>,
    pub signatures: alloc::vec::Vec<(
        alloc::string::String,
        alloc::string::String,
        alloc::vec::Vec<alloc::string::String>,
        crate::Span,
    )>,
    pub contracts: alloc::vec::Vec<crate::source::declared::parsing::Contract>,
    pub proofs: alloc::vec::Vec<crate::source::declared::parsing::Proof>,
    pub definition_spans: alloc::vec::Vec<crate::source::declared::DefinitionSource>,
    pub exports: alloc::vec::Vec<alloc::string::String>,
    names: alloc::vec::Vec<alloc::string::String>,
}

struct OpaqueDeclaration {
    name: alloc::string::String,
    base: alloc::string::String,
    public: bool,
}

#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
impl Collected {
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; the only mutable input is the K-CHECK-07 request budget, a request-local counter debited by the meters of this checking stage; no source, published compiler state or caller value is mutated."
    )]
    pub(super) fn collect(
        members: alloc::vec::Vec<crate::source::declared::parsing::Member>,
        limits: crate::Limits,
        selected_locals: bool,
        request_work: Option<&alloc::rc::Rc<core::cell::Cell<u32>>>,
    ) -> Result<Self, crate::source::Error> {
        let count = members.len();
        let mut collected = Ok(Self {
            schemas: alloc::vec::Vec::with_capacity(count),
            definitions: alloc::vec::Vec::with_capacity(count),
            requirements: alloc::vec::Vec::with_capacity(count),
            signatures: alloc::vec::Vec::with_capacity(count),
            contracts: alloc::vec::Vec::with_capacity(count),
            proofs: alloc::vec::Vec::with_capacity(count),
            definition_spans: alloc::vec::Vec::with_capacity(count),
            exports: alloc::vec::Vec::with_capacity(count),
            names: alloc::vec::Vec::with_capacity(count),
        });
        let mut members = members;
        members.reverse();
        while let Some(member) = members.pop() {
            collected = match collected {
                Ok(collected) => match member {
                    crate::source::declared::parsing::Member::Export(name) => {
                        collected.add_export(name)
                    }
                    crate::source::declared::parsing::Member::Opaque { name, base, public } => {
                        collected.add_opaque(OpaqueDeclaration { name, base, public })
                    }
                    crate::source::declared::parsing::Member::Variant {
                        name,
                        parameters,
                        left,
                        left_type,
                        left_public,
                        right,
                        right_type,
                        right_public,
                    } => collected.add_variant(
                        name,
                        parameters,
                        (left, left_type, left_public),
                        (right, right_type, right_public),
                    ),
                    crate::source::declared::parsing::Member::Require {
                        name,
                        input,
                        operation,
                    } => collected.add_requirement((name, input, operation)),
                    crate::source::declared::parsing::Member::Signature {
                        name,
                        binders,
                        words,
                        span,
                    } => collected.add_signature(name, binders, words, span),
                    crate::source::declared::parsing::Member::Definition {
                        name,
                        bytes,
                        span,
                        body_span,
                    } => collected.add_definition(
                        name,
                        bytes,
                        span,
                        body_span,
                        limits,
                        selected_locals,
                        request_work,
                    ),
                    crate::source::declared::parsing::Member::Contract(contract) => {
                        collected.add_contract(contract)
                    }
                    crate::source::declared::parsing::Member::Proof(proof) => {
                        collected.add_proof(proof)
                    }
                },
                Err(problem) => Err(problem),
            };
            if collected.is_err() {
                break;
            }
        }
        let collected = attempt!(collected);
        attempt!(collected.validate());
        Ok(collected)
    }

    fn add_export(mut self, name: alloc::string::String) -> Result<Self, crate::source::Error> {
        if self.exports.contains(&name) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate export",
            ));
        }
        self.exports.push(name);
        Ok(self)
    }

    fn add_contract(
        mut self,
        contract: crate::source::declared::parsing::Contract,
    ) -> Result<Self, crate::source::Error> {
        if self.names.contains(&contract.name) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate module contract",
            ));
        }
        self.names.push(contract.name.clone());
        self.contracts.push(contract);
        Ok(self)
    }

    fn add_proof(
        mut self,
        proof: crate::source::declared::parsing::Proof,
    ) -> Result<Self, crate::source::Error> {
        if self.names.contains(&proof.name) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate module proof",
            ));
        }
        self.names.push(proof.name.clone());
        self.proofs.push(proof);
        Ok(self)
    }

    fn add_opaque(self, declaration: OpaqueDeclaration) -> Result<Self, crate::source::Error> {
        let OpaqueDeclaration { name, base, public } = declaration;
        if self.names.contains(&name) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate module declaration",
            ));
        }
        Ok(self.insert_schema(
            name,
            crate::source::declared::SchemaKind::Opaque { base, public },
        ))
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; Vec<String>::contains, String equality, insert_schema and owned destructor require runtime on both pinned compilers (E0277/E0658/E0015/E0493); reassess when these APIs become const-capable."
    )]
    fn add_variant(
        self,
        name: alloc::string::String,
        parameters: alloc::vec::Vec<alloc::string::String>,
        left_arm: (alloc::string::String, alloc::string::String, bool),
        right_arm: (alloc::string::String, alloc::string::String, bool),
    ) -> Result<Self, crate::source::Error> {
        let (left, left_type, left_public) = left_arm;
        let (right, right_type, right_public) = right_arm;
        if self.names.contains(&name) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate constructor or module declaration",
            ));
        }
        if left == right {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate constructor or module declaration",
            ));
        }
        Ok(self.insert_schema(
            name,
            crate::source::declared::SchemaKind::Variant {
                parameters,
                left,
                left_type,
                left_public,
                right,
                right_type,
                right_public,
            },
        ))
    }

    fn insert_schema(
        mut self,
        name: alloc::string::String,
        kind: crate::source::declared::SchemaKind,
    ) -> Self {
        let registered_name = name.clone();
        self.names.push(registered_name);
        self.schemas
            .push(crate::source::declared::Schema { name, kind });
        self
    }

    fn add_requirement(
        mut self,
        requirement: (
            alloc::string::String,
            alloc::string::String,
            alloc::string::String,
        ),
    ) -> Result<Self, crate::source::Error> {
        if self.names.contains(&requirement.0) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate required operation",
            ));
        }
        self.names.push(requirement.0.clone());
        self.requirements.push(requirement);
        Ok(self)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
    )]
    fn add_signature(
        mut self,
        name: alloc::string::String,
        binders: alloc::string::String,
        words: alloc::vec::Vec<alloc::string::String>,
        span: crate::Span,
    ) -> Result<Self, crate::source::Error> {
        if self
            .signatures
            .iter()
            .any(|(known, _, _, _)| known == &name)
        {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate source signature",
            ));
        }
        self.signatures.push((name, binders, words, span));
        Ok(self)
    }

    #[expect(
        tigerstyle::too_many_parameters,
        clippy::too_many_arguments,
        reason = "Owner: noble-contracts; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
    )]
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; the only mutable input is the K-CHECK-07 request budget, a request-local counter debited by the meters of this checking stage; no source, published compiler state or caller value is mutated."
    )]
    fn add_definition(
        mut self,
        name: alloc::string::String,
        bytes: alloc::vec::Vec<u8>,
        span: crate::Span,
        body_span: crate::Span,
        limits: crate::Limits,
        selected_locals: bool,
        request_work: Option<&alloc::rc::Rc<core::cell::Cell<u32>>>,
    ) -> Result<Self, crate::source::Error> {
        let mut meter = crate::Meter::pooled(limits, request_work);
        let parsed = crate::source::parsing::parse_declared(&bytes, &mut meter);
        let (mut tree, parsed_name) = attempt!(parsed.map_err(|problem| {
            crate::source::declared::diagnostic(crate::source::Stage::Parse, problem)
        }));
        if selected_locals {
            attempt!(
                crate::source::locals::lower(&mut tree, &mut meter).map_err(|problem| {
                    crate::source::declared::diagnostic(crate::source::Stage::Resolve, problem)
                })
            );
        }
        let parsed_name = attempt!(parsed_name.ok_or_else(|| crate::source::declared::error(
            crate::source::Stage::Parse,
            "module definition missing name"
        )));
        if parsed_name != name {
            return Err(crate::source::declared::error(
                crate::source::Stage::Parse,
                "module and ordinary definition names differ",
            ));
        }
        if self.names.contains(&name) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "duplicate module definition",
            ));
        }
        self.names.push(name.clone());
        self.definition_spans
            .push(crate::source::declared::DefinitionSource {
                name: name.clone(),
                span,
                body_span,
            });
        self.definitions.push((name, bytes, tree));
        Ok(self)
    }

    fn validate(&self) -> Result<(), crate::source::Error> {
        if self.signatures.iter().any(|(name, _, _, _)| {
            !self
                .definitions
                .iter()
                .any(|(defined, _, _)| defined == name)
        }) {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "signature has no matching source definition",
            ));
        }
        let mut constructors = 0usize;
        let mut schema_index = 0;
        while schema_index < self.schemas.len() {
            constructors += match &self.schemas[schema_index].kind {
                crate::source::declared::SchemaKind::Opaque { .. } => 2,
                crate::source::declared::SchemaKind::Variant { .. } => 3,
            };
            schema_index += 1;
        }
        if constructors > crate::source::declared::CONSTRUCTOR_CAP
            || self.definitions.len() > crate::source::declared::CONSTRUCTOR_CAP
        {
            return Err(crate::source::declared::error(
                crate::source::Stage::Check,
                "module schema/definition count exceeded",
            ));
        }
        let mut position = 0;
        let mut has_missing_export = false;
        while position < self.exports.len() {
            let exported = &self.exports[position];
            let exported_bytes = exported.as_bytes();
            let mut split = 0;
            while split < exported_bytes.len() {
                if exported_bytes[split] == b'.' {
                    break;
                }
                split += 1;
            }
            let schema = &exported[..split];
            let mut has_declaration = false;
            let mut index = 0;
            while index < self.schemas.len() {
                if self.schemas[index].name == schema {
                    has_declaration = true;
                    break;
                }
                index += 1;
            }
            if !has_declaration {
                index = 0;
                while index < self.definitions.len() {
                    if self.definitions[index].0 == *exported {
                        has_declaration = true;
                        break;
                    }
                    index += 1;
                }
            }
            if !has_declaration {
                has_declaration = self
                    .contracts
                    .iter()
                    .any(|contract| contract.name == *exported)
                    || self.proofs.iter().any(|proof| proof.name == *exported);
            }
            if !has_declaration {
                has_missing_export = true;
                break;
            }
            position += 1;
        }
        if has_missing_export {
            return Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "export references missing declaration",
            ));
        }
        Ok(())
    }
}
