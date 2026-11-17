mod namespace;
mod prepare;
mod proof;
mod rebind;
mod register;

macro_rules! present {
    ($candidate:expr) => {
        match $candidate {
            Some(value) => value,
            None => return None,
        }
    };
}

// A v1 unit is classified into exactly one of these four syntactic forms.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    Module,
    Import,
    Definition,
    Expression,
}

#[derive(Debug)]
pub struct ModuleSession {
    bindings: alloc::vec::Vec<super::BoundOperation>,
    pub(super) modules: alloc::vec::Vec<super::Module>,
    aliases: alloc::vec::Vec<super::Alias>,
    // A namespace owner is never identified by the length of a retained
    // collection: retirement must not make an old captured owner reusable.
    next_module_identity: u64,
    generation: u64,
    #[expect(
        tigerstyle::numeric_units,
        reason = "Owner: noble-contracts; the field stores RetainedSourceLimit::bytes(), so its unit is source bytes as documented on that type."
    )]
    retained_source_limit: u32,
    pub(super) source: crate::source::Session,
}

// The calculator profile is selected by new_calculator, never by caller-supplied Limits.
const CALCULATOR_RETAINED_SOURCE_BYTES: u32 = 512 * 1024;

#[derive(Debug)]
pub struct ModulePrepared {
    kind: ModuleKind,
    generation: u64,
    limits: crate::Limits,
    previous_source_generation: u64,
    previous_source_definitions_len: usize,
    previous_modules: alloc::vec::Vec<super::Module>,
    previous_aliases: alloc::vec::Vec<super::Alias>,
    previous_next_module_identity: u64,
    previous_history: alloc::vec::Vec<u8>,
    staged: ModuleSession,
    submission: Option<noble_kernel::execution::Submission>,
    output: alloc::vec::Vec<noble_kernel::types::Ty>,
    pending: Option<crate::intrinsic::ProofBatch>,
    /// Only a signed executable root retains its original expression bytes;
    /// the module bodies themselves are retained in their module sources.
    signed_expression: alloc::vec::Vec<u8>,
    /// Some(empty) proves a genuine old-to-new import alias rebind with no
    /// current outside dependents. None is an initial import or other unit.
    rebuilt_import_definitions: Option<alloc::vec::Vec<RebuiltImportDefinition>>,
}

#[derive(Clone, Debug)]
pub struct RebuiltImportDefinition {
    name: alloc::string::String,
    previous_identity: u64,
    new_identity: u64,
}

impl RebuiltImportDefinition {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn previous_identity(&self) -> u64 {
        self.previous_identity
    }
    pub const fn new_identity(&self) -> u64 {
        self.new_identity
    }
}

/// Borrowed nominal declared by the newly prepared, proof-free module.
/// Its kernel declaration and source-checked module identity cannot be
/// constructed from a worker-supplied schema row.
pub struct CheckedNominal<'a> {
    declaration: &'a noble_kernel::contracts::NominalDecl,
    module_identity: u64,
    module_version: u32,
}

impl CheckedNominal<'_> {
    pub const fn declaration(&self) -> &noble_kernel::contracts::NominalDecl {
        self.declaration
    }
    pub const fn module_identity(&self) -> u64 {
        self.module_identity
    }
    pub const fn module_version(&self) -> u32 {
        self.module_version
    }
}

/// One exported named definition from the staged module's checked source.
/// The slot is a session-local source index, not a package-provided owner.
pub struct CheckedProgramOwner {
    module_identity: u64,
    module_version: u32,
    definition_ordinal: u32,
    definition_identity: u64,
    definition_slot: u32,
    /// The definition calls itself through its own verified signed owner.
    recursive: bool,
}

/// Why a closed Worker Program closure was not established. Exhaustion stays
/// distinct so a caller's shared request budget keeps its primary outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramClosureError {
    /// The walk ran out of the work limit this preparation was given.
    Exhausted,
    /// A slot invocation: live-slot graph shapes are not closed Worker
    /// Program closures.
    Unsupported,
    /// A missing, foreign, unverified, unexported or unreachable definition,
    /// or a self call outside its own verified signed owner.
    Invalid,
}

/// Source-checked, owner-bound Program interface for a separately selected
/// Worker WIT resource mapping. A type token is not an installed guest cell,
/// host-operation authority or portable package acceptance.
pub struct CheckedProgramInterface {
    input: alloc::vec::Vec<noble_kernel::types::Ty>,
    output: alloc::vec::Vec<noble_kernel::types::Ty>,
    effects: noble_kernel::types::EffSet,
}

impl CheckedProgramInterface {
    pub fn is_pure_i64_endomorphism(&self) -> bool {
        self.input.as_slice() == [noble_kernel::types::Ty::I64]
            && self.output.as_slice() == [noble_kernel::types::Ty::I64]
            && self.effects.is_empty()
    }

    pub fn is_test_emit_i64_endomorphism(&self) -> bool {
        self.input.as_slice() == [noble_kernel::types::Ty::I64]
            && self.output.as_slice() == [noble_kernel::types::Ty::I64]
            && self.effects.as_slice() == [noble_kernel::contracts::TEST_EMIT]
    }

    pub fn is_test_code_export_i64_endomorphism(&self) -> bool {
        self.input.as_slice() == [noble_kernel::types::Ty::I64]
            && self.output.as_slice() == [noble_kernel::types::Ty::I64]
            && self.effects.as_slice() == [noble_kernel::contracts::TEST_CODE_EXPORT]
    }

    pub fn is_test_send_i64_endomorphism(&self) -> bool {
        self.input.as_slice() == [noble_kernel::types::Ty::I64]
            && self.output.as_slice() == [noble_kernel::types::Ty::I64]
            && self.effects.as_slice() == [noble_kernel::contracts::TEST_SEND]
    }

    pub fn ty(&self) -> noble_kernel::types::Ty {
        noble_kernel::types::Ty::program(
            self.input.clone(),
            self.output.clone(),
            self.effects.clone(),
        )
    }
}

impl CheckedProgramOwner {
    pub const fn module_identity(&self) -> u64 {
        self.module_identity
    }
    pub const fn module_version(&self) -> u32 {
        self.module_version
    }
    pub const fn definition_ordinal(&self) -> u32 {
        self.definition_ordinal
    }
    pub const fn definition_identity(&self) -> u64 {
        self.definition_identity
    }
    pub const fn definition_slot(&self) -> u32 {
        self.definition_slot
    }
    /// True when this definition is its own scoped recursive owner.
    pub const fn is_recursive(&self) -> bool {
        self.recursive
    }
}

/// Borrowed, unforgeable-from-DTO ingress for signed source rederivation.
/// Neither an Env, a Candidate, nor the structural kernel result can make one.
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the name matches the established vocabulary used by callers; renaming it would churn call sites without clarifying them."
)]
pub struct SignedSourceOwner<'a> {
    prepared: &'a ModulePrepared,
}

/// Checked source-only declared-text ingress. No public Env or Submission can
/// select the merged declared/Text profile at the selected Wasm compiler.
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the name matches the established vocabulary used by callers; renaming it would churn call sites without clarifying them."
)]
pub struct DeclaredTextSourceOwner<'a> {
    prepared: &'a ModulePrepared,
}

impl DeclaredTextSourceOwner<'_> {
    pub fn rederive(&self) -> Result<noble_kernel::execution::Submission, crate::source::Error> {
        let prepared = self.prepared;
        if !prepared
            .staged
            .source
            .declared
            .as_ref()
            .is_some_and(|context| context.declared_text)
            || !prepared.staged.bindings.is_empty()
        {
            return Err(super::error(
                crate::source::Stage::Check,
                "declared text source is not in the selected pure profile",
            ));
        }
        attempt!(prepared.verify_signed_origins());
        let original = prepared.submission.as_ref().ok_or_else(|| {
            super::error(
                crate::source::Stage::Check,
                "declared text expression has no submission",
            )
        })?;
        let replay = attempt!(prepared.staged.snapshot().pending_signed_expression_words());
        let mut rebuilt = attempt!(replay.source.prepare_with_retention(
            &prepared.signed_expression,
            &original.request.expected.stack_in,
            prepared.limits,
            replay.retained_source_limit,
        ));
        rebuilt.submission.take().ok_or_else(|| {
            super::error(
                crate::source::Stage::Check,
                "declared text source did not rederive an executable body",
            )
        })
    }
}

impl SignedSourceOwner<'_> {
    /// Recreate the executable program from the staged, private source
    /// snapshot after authenticating each signed lexical owner against the
    /// retained whole-module bytes. The backend compiles this new derivation,
    /// never the caller's mutable public Submission.
    pub fn rederive(&self) -> Result<noble_kernel::execution::Submission, crate::source::Error> {
        let prepared = self.prepared;
        attempt!(prepared.verify_signed_origins());
        let Some(original) = prepared.submission.as_ref() else {
            return Err(super::error(
                crate::source::Stage::Check,
                "signed expression has no submission",
            ));
        };
        let replay = attempt!(prepared.staged.snapshot().pending_signed_expression_words());
        let mut rebuilt = attempt!(replay.source.prepare_with_retention(
            &prepared.signed_expression,
            &original.request.expected.stack_in,
            prepared.limits,
            replay.retained_source_limit,
        ));
        rebuilt.submission.take().ok_or_else(|| {
            super::error(
                crate::source::Stage::Check,
                "signed source did not rederive an executable body",
            )
        })
    }
}

impl ModulePrepared {
    /// Resolve against the staged checked namespace, not a display spelling
    /// guessed from a package. Unchecked proof-bearing modules cannot lend it.
    pub fn resolve_type(
        &self,
        spelling: &str,
    ) -> Result<noble_kernel::types::Ty, crate::source::Error> {
        if self.pending.is_some() || !self.staged.bindings.is_empty() {
            return Err(super::error(
                crate::source::Stage::Acceptance,
                "unchecked proof or host-bound module cannot supply a worker type",
            ));
        }
        self.staged.resolve_type(spelling)
    }

    /// Only the newly prepared module's nominal declarations are exposed.
    pub fn checked_nominals(&self) -> Option<impl Iterator<Item = CheckedNominal<'_>> + '_> {
        if self.kind != ModuleKind::Module
            || self.pending.is_some()
            || !self.staged.bindings.is_empty()
        {
            return None;
        }
        let module = self.staged.modules.last()?;
        let environment = &self.staged.source.declared.as_ref()?.environment;
        Some(
            environment
                .nominals
                .iter()
                .filter(|declaration| declaration.id.module == module.identity)
                .map(move |declaration| CheckedNominal {
                    declaration,
                    module_identity: module.identity,
                    module_version: module.version,
                }),
        )
    }

    /// This owner comes from an exported, matching module-owned checked
    /// definition; an unverified signed self owner is not a worker capability.
    pub fn checked_program_owner(&self, name: &str) -> Option<CheckedProgramOwner> {
        if self.kind != ModuleKind::Module
            || self.pending.is_some()
            || !self.staged.bindings.is_empty()
        {
            return None;
        }
        let module = self.staged.modules.last()?;
        self.checked_program_owner_source(module, name)
    }

    /// Host-selected source provenance, not permission to invoke the bound
    /// operation. The binding must be the exact independently supplied row.
    pub fn checked_program_owner_for_binding(
        &self,
        name: &str,
        binding: &super::BoundOperation,
    ) -> Option<CheckedProgramOwner> {
        if self.kind != ModuleKind::Module || self.pending.is_some() {
            return None;
        }
        let module = self.staged.modules.last()?;
        self.checked_program_bound_closure(module, name, binding)
            .ok()
            .map(|(owner, _)| owner)
    }

    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; each compound guard states one ordered admission rule verbatim; extracting its operands would change short-circuit evaluation."
    )]
    fn checked_program_owner_source(
        &self,
        module: &super::Module,
        name: &str,
    ) -> Option<CheckedProgramOwner> {
        let ordinal = module
            .definition_spans
            .iter()
            .position(|definition| definition.name == name)?;
        let exported = module.exports.iter().find(|entry| entry.name == name)?;
        let (_, crate::source::Target::Named(index)) =
            exported.words.iter().find(|(word, _)| word == name)?
        else {
            return None;
        };
        let definition = self
            .staged
            .source
            .definitions
            .get(usize::try_from(*index).ok()?)?;
        if definition.owner != Some(module.identity)
            || definition.name != name
            || definition.signed.is_some() && !module.verified_signed_slots.contains(index)
        {
            return None;
        }
        Some(CheckedProgramOwner {
            module_identity: module.identity,
            module_version: module.version,
            definition_ordinal: u32::try_from(ordinal).ok()?,
            definition_identity: definition.identity,
            definition_slot: *index,
            recursive: calls_self(definition),
        })
    }

    /// A closed package must contain exactly the named definitions reachable
    /// from its exported root, including calls inside quotations and checked
    /// lexical expansions. Foreign owners, unverified lexical self and unused
    /// decoy rows fail closed.
    pub fn checked_program_closure(
        &self,
        root: &str,
    ) -> Result<alloc::vec::Vec<CheckedProgramOwner>, ProgramClosureError> {
        let owner = self
            .checked_program_owner(root)
            .ok_or(ProgramClosureError::Invalid)?;
        let module = self
            .staged
            .modules
            .last()
            .ok_or(ProgramClosureError::Invalid)?;
        self.checked_program_closure_source(module, &owner, &[])
    }

    /// The exported root and its closed dependency closure, established from
    /// an exact checked invocation of that root in the pure profile. A signed
    /// owner is verified only when an expression invoking it is prepared, so
    /// a scoped recursive owner can be established here, never from the
    /// module alone. `dependencies` names the host-selected committed modules
    /// whose definitions the root may reach; each must contribute a reached
    /// definition, and any other owner fails closed.
    pub fn checked_invocation_closure(
        &self,
        module_identity: u64,
        root: &str,
        dependencies: &[u64],
    ) -> Result<(CheckedProgramOwner, alloc::vec::Vec<CheckedProgramOwner>), ProgramClosureError>
    {
        if self.kind != ModuleKind::Expression
            || self.pending.is_some()
            || !self.staged.bindings.is_empty()
        {
            return Err(ProgramClosureError::Invalid);
        }
        let module = self
            .staged
            .modules
            .iter()
            .find(|module| module.identity == module_identity)
            .ok_or(ProgramClosureError::Invalid)?;
        let selected = self.selected_dependency_modules(module_identity, dependencies)?;
        let owner = self
            .checked_program_owner_source(module, root)
            .ok_or(ProgramClosureError::Invalid)?;
        if self.checked_program_invocation(&owner).is_none() {
            return Err(ProgramClosureError::Invalid);
        }
        let closure = self.checked_program_closure_source(module, &owner, &selected)?;
        Ok((owner, closure))
    }

    /// Distinct committed modules other than the root, in selection order.
    fn selected_dependency_modules(
        &self,
        root_module: u64,
        dependencies: &[u64],
    ) -> Result<alloc::vec::Vec<&super::Module>, ProgramClosureError> {
        let mut selected = alloc::vec::Vec::with_capacity(dependencies.len());
        for (at, identity) in dependencies.iter().enumerate() {
            if *identity == root_module || dependencies[..at].contains(identity) {
                return Err(ProgramClosureError::Invalid);
            }
            let module = self
                .staged
                .modules
                .iter()
                .find(|module| module.identity == *identity)
                .ok_or(ProgramClosureError::Invalid)?;
            selected.push(module);
        }
        Ok(selected)
    }

    /// Only a closed source graph that actually invokes the exact selected
    /// host operation can describe a bound Worker Program.
    pub fn checked_program_closure_for_binding(
        &self,
        root: &str,
        binding: &super::BoundOperation,
    ) -> Result<alloc::vec::Vec<CheckedProgramOwner>, ProgramClosureError> {
        if self.kind != ModuleKind::Module || self.pending.is_some() {
            return Err(ProgramClosureError::Invalid);
        }
        let module = self
            .staged
            .modules
            .last()
            .ok_or(ProgramClosureError::Invalid)?;
        self.checked_program_bound_closure(module, root, binding)
            .map(|(_, closure)| closure)
    }

    fn checked_program_bound_closure(
        &self,
        module: &super::Module,
        root: &str,
        binding: &super::BoundOperation,
    ) -> Result<(CheckedProgramOwner, alloc::vec::Vec<CheckedProgramOwner>), ProgramClosureError>
    {
        let adapter = self
            .checked_bound_adapter(module, binding)
            .ok_or(ProgramClosureError::Invalid)?;
        let owner = self
            .checked_program_owner_source(module, root)
            .ok_or(ProgramClosureError::Invalid)?;
        let closure = self.checked_program_closure_source(module, &owner, &[])?;
        if self.closure_invokes_bound(&closure, adapter.definition) {
            Ok((owner, closure))
        } else {
            Err(ProgramClosureError::Invalid)
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; iterator adapters and slice comparisons in the body are not const."
    )]
    fn closure_invokes_bound(
        &self,
        closure: &[CheckedProgramOwner],
        bound: noble_kernel::contracts::Definition,
    ) -> bool {
        closure.iter().any(|member| {
            usize::try_from(member.definition_slot)
                .ok()
                .and_then(|slot| self.staged.source.definitions.get(slot))
                .is_some_and(|definition| {
                    definition.tree.nodes.iter().any(|node| {
                        matches!(node.kind,
                        crate::source::Kind::Call(crate::source::Target::Builtin(target))
                            if target == bound.0)
                    })
                })
        })
    }

    #[expect(
        tigerstyle::compound_condition,
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-contracts; each compound guard states one ordered admission rule verbatim; extracting its operands would change short-circuit evaluation; usize is at least 32 bits on every supported target, so the u32-to-usize widening is lossless."
    )]
    fn checked_bound_adapter(
        &self,
        module: &super::Module,
        binding: &super::BoundOperation,
    ) -> Option<&noble_kernel::contracts::BoundAdapter> {
        if self.pending.is_some()
            || self.staged.bindings.as_slice() != core::slice::from_ref(binding)
            || !matches!(
                binding.operation.as_str(),
                "test.emit" | "test.code-export" | "test.send"
            )
            || module.name != binding.module_name
            || module.version != binding.module_version
            || module.adapter_slot != Some(binding.adapter_slot)
        {
            return None;
        }
        let environment = &self.staged.source.declared.as_ref()?.environment;
        let mut matching = environment.bound_adapters.iter().filter(|adapter| {
            adapter.adapter_slot == binding.adapter_slot
                && adapter.adapter_identity == binding.adapter_identity
                && adapter.input == binding.input
                && adapter.output == binding.output
                && adapter.effects.as_slice() == binding.effects.as_slice()
                && environment
                    .definition_owners
                    .get(adapter.definition.0 as usize)
                    == Some(&Some(module.identity))
                && (matches!(environment.kind(adapter.definition),
                    Some(noble_kernel::contracts::Behavior::BoundEmit(slot))
                        if slot == binding.adapter_slot
                            && binding.operation == "test.emit")
                    || matches!(environment.kind(adapter.definition),
                        Some(noble_kernel::contracts::Behavior::BoundCodeExport(slot))
                            if slot == binding.adapter_slot
                                && binding.operation == "test.code-export")
                    || matches!(environment.kind(adapter.definition),
                        Some(noble_kernel::contracts::Behavior::BoundSend(slot))
                            if slot == binding.adapter_slot
                                && binding.operation == "test.send"))
        });
        let adapter = matching.next()?;
        matching.next().is_none().then_some(adapter)
    }

    fn checked_program_closure_source(
        &self,
        module: &super::Module,
        owner: &CheckedProgramOwner,
        dependencies: &[&super::Module],
    ) -> Result<alloc::vec::Vec<CheckedProgramOwner>, ProgramClosureError> {
        if module.definition_spans.is_empty() {
            return Err(ProgramClosureError::Invalid);
        }
        // The closure traversal is bounded by the module definition cap that
        // admission enforces, not by the module *count* cap; a module at its
        // legal definition cap still closes.
        if core::iter::once(module)
            .chain(dependencies.iter().copied())
            .any(|selected| selected.definition_spans.len() > super::CONSTRUCTOR_CAP)
        {
            return Err(ProgramClosureError::Exhausted);
        }
        let meter = crate::Meter::pooled(self.limits, self.staged.source.request_work());
        let (visited, meter) = self.closure_walk(module, owner, dependencies, meter)?;
        // Every root-module definition is reachable; a selected dependency
        // module contributes exactly its reached definitions, and at least one.
        let (mut closure, unreached, mut meter) = self.reached_rows(module, &visited, meter)?;
        if unreached != 0 {
            return Err(ProgramClosureError::Invalid);
        }
        for dependency in dependencies {
            let (rows, _, next) = self.reached_rows(dependency, &visited, meter)?;
            meter = next;
            if rows.is_empty() {
                return Err(ProgramClosureError::Invalid);
            }
            closure.extend(rows);
        }
        if visited.iter().filter(|seen| **seen).count() == closure.len() {
            Ok(closure)
        } else {
            Err(ProgramClosureError::Invalid)
        }
    }

    /// Definitions reachable from the root; an owner outside the root and
    /// selected modules, or an unverified signed slot, fails closed. The
    /// meter is returned with what the walk charged.
    fn closure_walk(
        &self,
        module: &super::Module,
        owner: &CheckedProgramOwner,
        dependencies: &[&super::Module],
        meter: crate::Meter,
    ) -> Result<(alloc::vec::Vec<bool>, crate::Meter), ProgramClosureError> {
        let mut meter = meter;
        let named = &self.staged.source.definitions;
        let root_index =
            usize::try_from(owner.definition_slot).map_err(|_| ProgramClosureError::Invalid)?;
        let mut visited = alloc::vec![false; named.len()];
        let mut enqueued = alloc::vec![false; named.len()];
        let mut pending = alloc::vec![root_index];
        *enqueued
            .get_mut(root_index)
            .ok_or(ProgramClosureError::Invalid)? = true;
        while let Some(index) = pending.pop() {
            let definition = named.get(index).ok_or(ProgramClosureError::Invalid)?;
            let owning = closure_owner(module, dependencies, definition.owner)
                .ok_or(ProgramClosureError::Invalid)?;
            if definition.signed.is_some()
                && !owning
                    .verified_signed_slots
                    .contains(&u32::try_from(index).map_err(|_| ProgramClosureError::Invalid)?)
            {
                return Err(ProgramClosureError::Invalid);
            }
            if visited[index] {
                continue;
            }
            visited[index] = true;
            for node in &definition.tree.nodes {
                meter
                    .charge(1, node.span)
                    .map_err(|_| ProgramClosureError::Exhausted)?;
                match &node.kind {
                    crate::source::Kind::Call(crate::source::Target::Named(target)) => {
                        let next =
                            usize::try_from(*target).map_err(|_| ProgramClosureError::Invalid)?;
                        if !*enqueued.get(next).ok_or(ProgramClosureError::Invalid)? {
                            enqueued[next] = true;
                            pending.push(next);
                        }
                    }
                    // A scoped recursive call is admitted only inside its own
                    // verified signed owner; it adds no edge to the closure.
                    crate::source::Kind::Call(crate::source::Target::SelfOwner) => {
                        let is_own_owner = definition.signed.as_ref().is_some_and(|relation| {
                            usize::try_from(relation.source_slot).ok() == Some(index)
                        });
                        if !is_own_owner {
                            return Err(ProgramClosureError::Invalid);
                        }
                    }
                    crate::source::Kind::Call(crate::source::Target::SlotInvoke(_)) => {
                        return Err(ProgramClosureError::Unsupported)
                    }
                    _ => {}
                }
            }
        }
        Ok((visited, meter))
    }

    /// Rows for `module`'s reached definitions in ordinal order, and how many
    /// of its definitions the walk did not reach, with the charged meter.
    fn reached_rows(
        &self,
        module: &super::Module,
        visited: &[bool],
        meter: crate::Meter,
    ) -> Result<(alloc::vec::Vec<CheckedProgramOwner>, usize, crate::Meter), ProgramClosureError>
    {
        let mut meter = meter;
        let named = &self.staged.source.definitions;
        let mut closure = alloc::vec::Vec::with_capacity(module.definition_spans.len());
        let mut unreached = 0usize;
        for (ordinal, entry) in module.definition_spans.iter().enumerate() {
            meter
                .charge(1, super::SPAN)
                .map_err(|_| ProgramClosureError::Exhausted)?;
            let index = named
                .iter()
                .position(|definition| {
                    definition.name == entry.name && definition.owner == Some(module.identity)
                })
                .ok_or(ProgramClosureError::Invalid)?;
            if !*visited.get(index).ok_or(ProgramClosureError::Invalid)? {
                unreached = unreached.saturating_add(1);
                continue;
            }
            closure.push(CheckedProgramOwner {
                module_identity: module.identity,
                module_version: module.version,
                definition_ordinal: u32::try_from(ordinal)
                    .map_err(|_| ProgramClosureError::Invalid)?,
                definition_identity: named[index].identity,
                definition_slot: u32::try_from(index).map_err(|_| ProgramClosureError::Invalid)?,
                recursive: calls_self(&named[index]),
            });
        }
        Ok((closure, unreached, meter))
    }

    pub fn declared_text_source_owner(&self) -> Option<DeclaredTextSourceOwner<'_>> {
        if self.kind != ModuleKind::Expression
            || self.signed_expression.is_empty()
            || !self
                .staged
                .source
                .declared
                .as_ref()
                .is_some_and(|context| context.declared_text)
        {
            return None;
        }
        Some(DeclaredTextSourceOwner { prepared: self })
    }

    pub fn signed_source_owner(&self) -> Option<SignedSourceOwner<'_>> {
        if self.kind != ModuleKind::Expression || self.signed_expression.is_empty() {
            return None;
        }
        Some(SignedSourceOwner { prepared: self })
    }

    #[expect(
        tigerstyle::compound_condition,
        tigerstyle::unbounded_collection_growth,
        reason = "Owner: noble-contracts; each compound guard states one ordered admission rule verbatim; extracting its operands would change short-circuit evaluation; growth is bounded by the checked input or admission limits that drive the loop."
    )]
    #[expect(
        tigerstyle::sentinel_fallback,
        reason = "Owner: noble-contracts; an unrepresentable definition index maps to usize::MAX, which get() reports as absent, so the ownership guard fails closed."
    )]
    fn verify_signed_origins(&self) -> Result<(), crate::source::Error> {
        let Some(submission) = self.submission.as_ref() else {
            return Err(super::error(
                crate::source::Stage::Check,
                "missing signed submission",
            ));
        };
        let mut requested_slots = alloc::vec::Vec::new();
        for definition in &submission.definitions {
            let mut has_self = false;
            for node in &definition.body.candidate.nodes {
                if let noble_kernel::untrusted::Node::SignedSelf { def, .. } = node {
                    has_self = true;
                    if *def != definition.definition {
                        return Err(super::error(
                            crate::source::Stage::Check,
                            "signed call does not target its containing definition",
                        ));
                    }
                    let index = usize::try_from(def.0).map_err(|_| {
                        super::error(crate::source::Stage::Check, "signed owner index overflow")
                    })?;
                    let owner = submission
                        .environment
                        .definition_owners
                        .get(index)
                        .copied()
                        .flatten()
                        .ok_or_else(|| {
                            super::error(
                                crate::source::Stage::Check,
                                "signed call has no declared module owner",
                            )
                        })?;
                    let source_slot = definition.owner_slot.ok_or_else(|| {
                        super::error(
                            crate::source::Stage::Check,
                            "signed owner has no exact source slot",
                        )
                    })?;
                    let source_index = usize::try_from(source_slot).map_err(|_| {
                        super::error(
                            crate::source::Stage::Check,
                            "signed owner source slot overflow",
                        )
                    })?;
                    let source = self
                        .staged
                        .source
                        .definitions
                        .get(source_index)
                        .ok_or_else(|| {
                            super::error(
                                crate::source::Stage::Check,
                                "signed owner source slot is absent",
                            )
                        })?;
                    if source.owner != Some(owner)
                        || definition.source_identity != source.identity
                        || source
                            .signed
                            .as_ref()
                            .is_none_or(|relation| relation.source_slot != source_slot)
                        || definition.owner_inst.is_none()
                    {
                        return Err(super::error(
                            crate::source::Stage::Check,
                            "signed owner source slot, module and binder differ",
                        ));
                    }
                    if !requested_slots.contains(&source_index) {
                        requested_slots.push(source_index);
                    }
                }
            }
            if !has_self && definition.owner_inst.is_some() {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "unsigned body carries a signed owner binder",
                ));
            }
        }
        let mut checked_slots = alloc::vec::Vec::new();
        for (source_index, definition) in self.staged.source.definitions.iter().enumerate() {
            if !definition.tree.nodes.iter().any(|node| {
                matches!(
                    node.kind,
                    crate::source::Kind::Call(crate::source::Target::SelfOwner)
                )
            }) {
                continue;
            }
            if !requested_slots.contains(&source_index) {
                continue;
            }
            let Some(owner) = definition.owner else {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed source has no module owner",
                ));
            };
            let Some(relation) = definition.signed.as_ref() else {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed source lacks its precommit rigid owner relation",
                ));
            };
            if usize::try_from(relation.source_slot).ok() != Some(source_index)
                || relation.lexical_nodes.is_empty()
            {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed owner source slot differs from its checked reservation",
                ));
            }
            let Some(module) = self
                .staged
                .modules
                .iter()
                .find(|module| module.identity == owner)
            else {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed source owner has no original module",
                ));
            };
            let Some((ordinal, span)) = module
                .definition_spans
                .iter()
                .enumerate()
                .find(|(_, original)| original.name == definition.name)
            else {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed definition absent from original module",
                ));
            };
            let start = usize::try_from(span.span.start).map_err(|_| {
                super::error(
                    crate::source::Stage::Check,
                    "signed source span exceeds address space",
                )
            })?;
            let end = usize::try_from(span.span.end).map_err(|_| {
                super::error(
                    crate::source::Stage::Check,
                    "signed source span exceeds address space",
                )
            })?;
            let Some(bytes) = module.source.get(start..end) else {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed definition lies outside original module",
                ));
            };
            let lexical_body = attempt!(super::verify_definition_origin_in_request(
                super::DefinitionOrigin {
                    module_source: &module.source,
                    module: &module.name,
                    version: module.version,
                    definition: &definition.name,
                    ordinal: u32::try_from(ordinal).map_err(|_| super::error(
                        crate::source::Stage::Check,
                        "signed definition ordinal overflow"
                    ))?,
                    span: span.span,
                },
                bytes,
                self.limits,
                self.staged.source.request_work(),
            )
            .map_err(|problem| super::diagnostic(crate::source::Stage::Check, problem)));
            if lexical_body != span.body_span {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed body differs from original module",
                ));
            }
            let mut meter = crate::Meter::pooled(self.limits, self.staged.source.request_work());
            attempt!(meter
                .charge(
                    attempt!(
                        crate::index(module.source.len(), definition.tree.span).map_err(
                            |problem| super::diagnostic(crate::source::Stage::Check, problem)
                        )
                    ),
                    definition.tree.span,
                )
                .map_err(|problem| super::diagnostic(crate::source::Stage::Check, problem)));
            let original_scheme = attempt!(register::original_signature(
                &self.staged,
                module,
                &definition.name,
                self.limits,
            ));
            let saved_scheme = definition.signature.as_ref().ok_or_else(|| {
                super::error(
                    crate::source::Stage::Check,
                    "signed definition has no retained signature",
                )
            })?;
            if !same_scheme(&original_scheme, saved_scheme) {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed definition signature differs from original module",
                ));
            }
            let source_slot = attempt!(crate::index(source_index, definition.tree.span)
                .map_err(|problem| super::diagnostic(crate::source::Stage::Check, problem)));
            let mut is_matched_specialization = false;
            for specialization in submission
                .definitions
                .iter()
                .filter(|specialization| specialization.owner_slot == Some(source_slot))
            {
                attempt!(meter
                    .charge(
                        attempt!(
                            crate::index(module.source.len(), definition.tree.span).map_err(
                                |problem| super::diagnostic(crate::source::Stage::Check, problem)
                            )
                        ),
                        definition.tree.span,
                    )
                    .map_err(|problem| super::diagnostic(crate::source::Stage::Check, problem)));
                if specialization.source_identity != definition.identity
                    || submission
                        .environment
                        .definition_owners
                        .get(usize::try_from(specialization.definition.0).unwrap_or(usize::MAX))
                        != Some(&Some(owner))
                    || submission
                        .environment
                        .scheme(specialization.definition)
                        .is_none_or(|scheme| !same_scheme(&original_scheme, scheme))
                {
                    return Err(super::error(
                        crate::source::Stage::Check,
                        "signed specialization has a foreign source, signature or effect ceiling",
                    ));
                }
                is_matched_specialization = true;
            }
            if !is_matched_specialization {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed definition has no exact specialized owner",
                ));
            }
            let (mut original, name) =
                attempt!(crate::source::parsing::parse_declared(bytes, &mut meter)
                    .map_err(|problem| super::diagnostic(crate::source::Stage::Check, problem)));
            if self
                .staged
                .source
                .declared
                .as_ref()
                .is_some_and(|context| context.declared_text)
            {
                attempt!(crate::source::locals::lower(&mut original, &mut meter)
                    .map_err(|problem| super::diagnostic(crate::source::Stage::Check, problem)));
            }
            if name.as_deref() != Some(definition.name.as_str())
                || original.body != definition.tree.body
                || original.nodes.len() != definition.tree.nodes.len()
            {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed tree differs from original definition",
                ));
            }
            let mut signed_count = 0usize;
            for (node_index, (saved, parsed)) in definition
                .tree
                .nodes
                .iter()
                .zip(&original.nodes)
                .enumerate()
            {
                if saved.span != parsed.span {
                    return Err(super::error(
                        crate::source::Stage::Check,
                        "signed node moved from original source",
                    ));
                }
                match (&saved.kind, &parsed.kind) {
                    (
                        crate::source::Kind::Call(crate::source::Target::SelfOwner),
                        crate::source::Kind::Word(word),
                    ) if word == definition.name.as_bytes() => {
                        let index = u32::try_from(node_index).map_err(|_| {
                            super::error(
                                crate::source::Stage::Check,
                                "signed lexical node index overflow",
                            )
                        })?;
                        if relation.lexical_nodes.binary_search(&index).is_err() {
                            return Err(super::error(
                                crate::source::Stage::Check,
                                "signed occurrence was not checked in its original quotation path",
                            ));
                        }
                        signed_count += 1;
                    }
                    (crate::source::Kind::Call(_), crate::source::Kind::Word(word))
                        if word != definition.name.as_bytes() => {}
                    (
                        crate::source::Kind::Call(crate::source::Target::Builtin(saved)),
                        crate::source::Kind::Call(crate::source::Target::Builtin(parsed)),
                    ) if saved == parsed => {}
                    (crate::source::Kind::Literal(a), crate::source::Kind::Literal(b))
                        if a == b => {}
                    (crate::source::Kind::Text(a), crate::source::Kind::Text(b)) if a == b => {}
                    (crate::source::Kind::Quotation(a), crate::source::Kind::Quotation(b))
                        if a == b => {}
                    _ => {
                        return Err(super::error(
                            crate::source::Stage::Check,
                            "signed lexical occurrence differs from original source",
                        ))
                    }
                }
            }
            if signed_count != relation.lexical_nodes.len() || definition.signature.is_none() {
                return Err(super::error(
                    crate::source::Stage::Check,
                    "signed owner lacks lexical self or signature",
                ));
            }
            attempt!(crate::source::preparation::symbolic::verify(
                &definition.tree,
                &original,
                relation,
                &definition.name,
                bytes,
                &mut meter,
            )
            .map_err(|problem| super::diagnostic(crate::source::Stage::Check, problem)));
            checked_slots.push(source_index);
        }
        if requested_slots
            .iter()
            .any(|slot| !checked_slots.contains(slot))
        {
            return Err(super::error(
                crate::source::Stage::Check,
                "signed call has no authenticated exact source definition",
            ));
        }
        if submission
            .body
            .candidate
            .nodes
            .iter()
            .any(|node| matches!(node, noble_kernel::untrusted::Node::SignedSelf { .. }))
        {
            return Err(super::error(
                crate::source::Stage::Check,
                "signed self in executable root is not an owner definition",
            ));
        }
        Ok(())
    }

    /// Bind a checked named invocation to its original pure interface. A
    /// candidate-reported owner or effect ceiling cannot create this token.
    pub fn checked_program_interface(
        &self,
        owner: &CheckedProgramOwner,
    ) -> Option<CheckedProgramInterface> {
        if !self.staged.bindings.is_empty() {
            return None;
        }
        let (submission, definition, _) = self.checked_program_invocation(owner)?;
        if !submission.request.expected.allowed_effects.is_empty()
            || !definition.expected.allowed_effects.is_empty()
            || definition.identity != owner.definition_identity
        {
            return None;
        }
        Some(CheckedProgramInterface {
            input: submission.request.expected.stack_in.clone(),
            output: self.output.clone(),
            effects: noble_kernel::types::EffSet::empty(),
        })
    }

    /// Source-checked interface for an independently selected bound effect
    /// binding. This type token does not authorize a host operation.
    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; each compound guard states one ordered admission rule verbatim; extracting its operands would change short-circuit evaluation."
    )]
    pub fn checked_program_interface_for_binding(
        &self,
        owner: &CheckedProgramOwner,
        binding: &super::BoundOperation,
    ) -> Option<CheckedProgramInterface> {
        let (submission, definition, module) = self.checked_program_invocation(owner)?;
        let adapter = self.checked_bound_adapter(module, binding)?;
        let closure = self
            .checked_program_closure_source(module, owner, &[])
            .ok()?;
        if !self.closure_invokes_bound(&closure, adapter.definition)
            || submission.request.expected.allowed_effects != adapter.effects
            || definition.expected.allowed_effects != adapter.effects
            || submission
                .environment
                .bound_adapters
                .iter()
                .filter(|actual| {
                    actual.definition == adapter.definition
                        && actual.adapter_identity == binding.adapter_identity
                        && actual.adapter_slot == binding.adapter_slot
                        && actual.input == binding.input
                        && actual.output == binding.output
                        && actual.effects == adapter.effects
                        && (matches!(submission.environment.kind(actual.definition),
                        Some(noble_kernel::contracts::Behavior::BoundEmit(slot))
                            if slot == binding.adapter_slot
                                && binding.operation == "test.emit")
                            || matches!(submission.environment.kind(actual.definition),
                        Some(noble_kernel::contracts::Behavior::BoundCodeExport(slot))
                            if slot == binding.adapter_slot
                                && binding.operation == "test.code-export")
                            || matches!(submission.environment.kind(actual.definition),
                        Some(noble_kernel::contracts::Behavior::BoundSend(slot))
                            if slot == binding.adapter_slot
                                && binding.operation == "test.send"))
                })
                .count()
                != 1
        {
            return None;
        }
        Some(CheckedProgramInterface {
            input: submission.request.expected.stack_in.clone(),
            output: self.output.clone(),
            effects: adapter.effects.clone(),
        })
    }

    #[expect(
        tigerstyle::compound_condition,
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-contracts; each compound guard states one ordered admission rule verbatim; extracting its operands would change short-circuit evaluation; usize is at least 32 bits on every supported target, so the u32-to-usize widening is lossless."
    )]
    fn checked_program_invocation(
        &self,
        owner: &CheckedProgramOwner,
    ) -> Option<(
        &noble_kernel::execution::Submission,
        &noble_kernel::execution::Definition,
        &super::Module,
    )> {
        if self.kind != ModuleKind::Expression || self.pending.is_some() {
            return None;
        }
        let module = self.staged.modules.iter().find(|module| {
            module.identity == owner.module_identity && module.version == owner.module_version
        })?;
        let ordinal = usize::try_from(owner.definition_ordinal).ok()?;
        let entry = module.definition_spans.get(ordinal)?;
        let source = self.checked_program_owner_source(module, &entry.name)?;
        if source.module_identity != owner.module_identity
            || source.module_version != owner.module_version
            || source.definition_ordinal != owner.definition_ordinal
            || source.definition_identity != owner.definition_identity
            || source.definition_slot != owner.definition_slot
        {
            return None;
        }
        let submission = self.submission.as_ref()?;
        if submission.request.expected.stack_out != self.output {
            return None;
        }
        let mut matched = submission.definitions.iter().filter(|definition| {
            definition.owner_slot == Some(owner.definition_slot)
                && definition.source_identity == owner.definition_identity
                && submission
                    .environment
                    .definition_owners
                    .get(definition.definition.0 as usize)
                    == Some(&Some(owner.module_identity))
        });
        let definition = matched.next()?;
        if matched.next().is_some() {
            return None;
        }
        // This token describes the named Program itself, not an arbitrary
        // expression that happens to call that name once before doing other
        // work. Only a single exact root invocation has that provenance.
        let [root] = submission.body.candidate.body.as_slice() else {
            return None;
        };
        let root = usize::try_from(root.0).ok()?;
        if !matches!(
            submission.body.candidate.nodes.get(root),
            Some(noble_kernel::untrusted::Node::Invocation { def, .. })
                if *def == definition.definition
        ) {
            return None;
        }
        Some((submission, definition, module))
    }

    pub const fn kind(&self) -> ModuleKind {
        self.kind
    }
    pub const fn submission(&self) -> Option<&noble_kernel::execution::Submission> {
        self.submission.as_ref()
    }
    pub const fn output(&self) -> &[noble_kernel::types::Ty] {
        self.output.as_slice()
    }
    /// Uncommitted source obligations, never evidence of proof acceptance.
    pub const fn proof_obligations(&self) -> Option<&crate::intrinsic::ProofBatch> {
        self.pending.as_ref()
    }
    /// Size-change evidence for every self-recursive definition this unit
    /// installed (issue #101). A definition with no lexical self call carries
    /// none; a definition outside the bounded classifier carries an
    /// `Unsupported` verdict rather than an assumed one.
    pub fn size_change_evidence(&self) -> alloc::vec::Vec<crate::source::Evidence> {
        let definitions = &self.staged.source.definitions;
        let start = self.previous_source_definitions_len.min(definitions.len());
        let environment = match self.staged.source.declared.as_ref() {
            Some(context) => &context.environment,
            None => return alloc::vec::Vec::new(),
        };
        let recursive_count = definitions[start..]
            .iter()
            .filter(|definition| crate::source::sizechange::calls_self(definition))
            .count();
        let mut evidence = alloc::vec::Vec::with_capacity(recursive_count);
        for definition in &definitions[start..] {
            if crate::source::sizechange::calls_self(definition) {
                evidence.push(crate::source::sizechange::evidence(definition, environment));
            }
        }
        evidence
    }
    /// Typed, non-executable contract metadata for the newly staged module,
    /// including modules with no proof; this is not a proof receipt.
    pub fn contract_goals(&self) -> impl Iterator<Item = &crate::intrinsic::ContractGoal> {
        self.staged
            .modules
            .last()
            .filter(|_| self.kind == ModuleKind::Module)
            .into_iter()
            .flat_map(|module| module.contracts.iter().map(|entry| &entry.goal))
    }
    pub fn bindings(&self) -> &[super::BoundOperation] {
        &self.staged.bindings
    }
    /// The final checked alias introduced by this import, not a source
    /// spelling re-parsed by a consumer or an alias from an earlier session.
    pub fn resolved_import_alias(&self) -> Option<&str> {
        if self.kind != ModuleKind::Import {
            return None;
        }
        self.staged
            .aliases
            .last()
            .filter(|alias| !alias.pending)
            .map(|alias| alias.spelling.as_str())
    }
    /// A source-checked transitive rebind preview, not an alias-word refresh.
    /// Publication still requires `ModuleSession::commit` on its original
    /// namespace snapshot; saved old Program values retain prior identities.
    pub fn rebuilt_import_definitions(&self) -> Option<&[RebuiltImportDefinition]> {
        self.rebuilt_import_definitions.as_deref()
    }
    pub fn resolved_module(&self) -> Option<(u64, alloc::string::String, u32)> {
        let module = match self.kind {
            ModuleKind::Module => present!(self.staged.modules.last()),
            ModuleKind::Import => {
                let alias = present!(self.staged.aliases.last());
                if alias.pending {
                    return None;
                }
                let module = present!(self.staged.modules.get(alias.module));
                module
            }
            ModuleKind::Definition | ModuleKind::Expression => {
                let submission = present!(self.submission.as_ref());
                let mut owner = None;
                let mut definition_at = 0;
                let mut is_invalid_owner = false;
                while definition_at < submission.definitions.len() {
                    let definition = &submission.definitions[definition_at];
                    let Ok(index) = usize::try_from(definition.definition.0) else {
                        is_invalid_owner = true;
                        break;
                    };
                    if let Some(id) = submission
                        .environment
                        .definition_owners
                        .get(index)
                        .copied()
                        .flatten()
                    {
                        if owner.is_some_and(|previous| previous != id) {
                            is_invalid_owner = true;
                            break;
                        }
                        owner = Some(id);
                    }
                    definition_at += 1;
                }
                if is_invalid_owner {
                    return None;
                }
                let mut found = None;
                let mut module_at = 0usize;
                while module_at < self.staged.modules.len() {
                    let module = &self.staged.modules[module_at];
                    if Some(module.identity) == owner {
                        found = Some(module);
                        break;
                    }
                    module_at += 1;
                }
                present!(found)
            }
        };
        Some((module.identity, module.name.clone(), module.version))
    }
    pub fn linked_binding(&self) -> Option<&super::BoundOperation> {
        if let Some(submission) = self.submission.as_ref() {
            let mut selected = None;
            let mut definition_at = 0;
            let mut is_ambiguous = false;
            while definition_at < submission.definitions.len() && !is_ambiguous {
                let definition = &submission.definitions[definition_at];
                let mut node_at = 0;
                while node_at < definition.body.candidate.nodes.len() {
                    let node = &definition.body.candidate.nodes[node_at];
                    if let noble_kernel::untrusted::Node::Invocation { def, .. } = node {
                        if let Some(
                            noble_kernel::contracts::Behavior::BoundEmit(slot)
                            | noble_kernel::contracts::Behavior::BoundClock(slot)
                            | noble_kernel::contracts::Behavior::BoundCodeExport(slot)
                            | noble_kernel::contracts::Behavior::BoundSend(slot),
                        ) = submission.environment.kind(*def)
                        {
                            if selected.is_some_and(|previous| previous != slot) {
                                is_ambiguous = true;
                                break;
                            }
                            selected = Some(slot);
                        }
                    }
                    node_at += 1;
                }
                definition_at += 1;
            }
            if is_ambiguous {
                return None;
            }
            if let Some(slot) = selected {
                return self.binding_for_slot(slot);
            }
        }
        let (_, module, version) = present!(self.resolved_module());
        let mut linked = None;
        let mut module_at = 0usize;
        while module_at < self.staged.modules.len() {
            let entry = &self.staged.modules[module_at];
            if entry.name == module && entry.version == version {
                linked = Some(entry);
                break;
            }
            module_at += 1;
        }
        let linked = present!(linked);
        let slot = present!(linked.adapter_slot);
        self.binding_for_slot(slot)
    }

    fn binding_for_slot(&self, slot: u32) -> Option<&super::BoundOperation> {
        let mut at = 0usize;
        let mut matched_at = None;
        while at < self.staged.bindings.len() && matched_at.is_none() {
            if self.staged.bindings[at].adapter_slot == slot {
                matched_at = Some(at);
            }
            at += 1;
        }
        match matched_at {
            Some(index) => self.staged.bindings.get(index),
            None => None,
        }
    }
}

#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
impl ModuleSession {
    /// Host-owned bindings are exact, immutable adapter declarations, not authorizations.
    pub fn new(
        bindings: &[super::BoundOperation],
        retained_source_limit: crate::source::RetainedSourceLimit,
    ) -> Result<Self, crate::source::Error> {
        Self::initialize(bindings, false, false, false, retained_source_limit.bytes())
    }

    /// Select the signed source grammar. Valid lexical ownership, kernel
    /// structure and a private compiler ingress are still required.
    pub fn new_signed_self(
        bindings: &[super::BoundOperation],
        retained_source_limit: crate::source::RetainedSourceLimit,
    ) -> Result<Self, crate::source::Error> {
        Self::initialize(bindings, true, false, false, retained_source_limit.bytes())
    }

    /// Selected pure declared text profile; its signed self extension is explicit.
    /// Even a denied host adapter cannot be admitted here.
    pub fn new_declared_text(
        bindings: &[super::BoundOperation],
        signed_self: bool,
        retained_source_limit: crate::source::RetainedSourceLimit,
    ) -> Result<Self, crate::source::Error> {
        if !bindings.is_empty() {
            return Err(super::error(
                crate::source::Stage::Link,
                "declared text does not accept host bindings",
            ));
        }
        Self::initialize(
            bindings,
            signed_self,
            true,
            false,
            retained_source_limit.bytes(),
        )
    }

    /// Calculator modules are separately selected and never receive adapters.
    pub fn new_calculator() -> Result<Self, crate::source::Error> {
        Self::initialize(&[], false, false, true, CALCULATOR_RETAINED_SOURCE_BYTES)
    }

    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape."
    )]
    fn initialize(
        bindings: &[super::BoundOperation],
        signed_self: bool,
        declared_text: bool,
        calculator: bool,
        retained_source_limit_bytes: u32,
    ) -> Result<Self, crate::source::Error> {
        if bindings.len() > super::MODULE_CAP {
            return Err(super::error(
                crate::source::Stage::Check,
                "too many bound operations",
            ));
        }
        let mut index = 0;
        let mut is_invalid_binding = false;
        while index < bindings.len() {
            if !super::links::valid_binding(&bindings[index]) {
                is_invalid_binding = true;
                break;
            }
            let mut prior = 0usize;
            while prior < index {
                if super::links::duplicate_binding(&bindings[index], &bindings[prior]) {
                    is_invalid_binding = true;
                    break;
                }
                prior += 1;
            }
            if is_invalid_binding {
                break;
            }
            index += 1;
        }
        if is_invalid_binding {
            return Err(super::error(
                crate::source::Stage::Link,
                "invalid, duplicate or mismatched adapter binding",
            ));
        }
        let mut environment = attempt!((if calculator {
            noble_kernel::contracts::calculator_environment()
        } else if declared_text {
            noble_kernel::contracts::declared_text_environment()
        } else {
            noble_kernel::contracts::environment()
        })
        .map_err(|_| super::error(crate::source::Stage::Check, "invalid kernel environment")));
        environment.declared_modules = true;
        let mut source_session = crate::source::Session::without_test_hosts();
        source_session.calculator = calculator;
        source_session.declared = Some(super::Context {
            environment,
            words: alloc::vec::Vec::new(),
            aliases: alloc::vec::Vec::new(),
            owner: None,
            signed_self,
            declared_text,
            request_work: None,
        });
        Ok(Self {
            source: source_session,
            bindings: bindings.to_vec(),
            modules: alloc::vec::Vec::new(),
            aliases: alloc::vec::Vec::new(),
            next_module_identity: 1,
            generation: 0,
            retained_source_limit: retained_source_limit_bytes,
        })
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn declared_text(&self) -> bool {
        self.source
            .declared
            .as_ref()
            .is_some_and(|context| context.declared_text)
    }

    /// An independent copy of a committed calculator namespace, so the
    /// calculator host can restore it after a failed submission. The private
    /// snapshot omits test hosts, the text cursor, live selections and
    /// component bindings, which a calculator session never has; any other
    /// session is refused rather than copied incompletely.
    pub fn checkpoint(&self) -> Option<Self> {
        let source = &self.source;
        let is_exact = source.calculator
            && !source.hosts
            && !source.text_cursor
            && source.live_selected.is_none()
            && source.live_slots.is_none()
            && source.bindings.is_none();
        is_exact.then(|| self.snapshot())
    }

    /// Copy this checked namespace for an unpublished external transaction.
    /// A proof recheck or worker proposal can commit to the copy without
    /// changing the retained session; its owner must revalidate its external
    /// safepoint before replacing the retained session with this successor.
    /// Share one host-selected request budget across every checking stage
    /// this session and the sessions staged from it perform: each internal
    /// meter then draws from the same remaining work instead of restarting
    /// from its own limit, which only caps that meter's share.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; the only mutable input is the K-CHECK-07 request budget, a request-local counter debited by the meters of this checking stage; no source, published compiler state or caller value is mutated."
    )]
    pub fn share_request_work(&mut self, work: u32) {
        if let Some(context) = self.source.declared.as_mut() {
            context.request_work = Some(alloc::rc::Rc::new(core::cell::Cell::new(work)));
        }
    }

    /// Remaining shared request work, if the host opted in.
    pub fn remaining_request_work(&self) -> Option<u32> {
        self.source.request_work().map(|remaining| remaining.get())
    }

    pub fn snapshot(&self) -> Self {
        Self {
            source: crate::source::Session {
                definitions: self.source.definitions.clone(),
                history: self.source.history.clone(),
                generation: self.source.generation,
                hosts: false,
                text_cursor: self.source.text_cursor,
                live_selected: None,
                live_slots: None,
                calculator: self.source.calculator,
                checked_i64: self.source.checked_i64,
                bindings: None,
                declared: self.source.declared.clone(),
            },
            bindings: self.bindings.clone(),
            modules: self.modules.clone(),
            aliases: self.aliases.clone(),
            next_module_identity: self.next_module_identity,
            generation: self.generation,
            retained_source_limit: self.retained_source_limit,
        }
    }
}

/// Whether a definition contains a lexical self call. Only a signed owner
/// can resolve one; the closure walk separately requires it to be its own.
fn calls_self(definition: &crate::source::Named) -> bool {
    definition.tree.nodes.iter().any(|node| {
        matches!(
            node.kind,
            crate::source::Kind::Call(crate::source::Target::SelfOwner)
        )
    })
}

/// The root module or the selected dependency module owning a definition.
fn closure_owner<'module>(
    module: &'module super::Module,
    dependencies: &[&'module super::Module],
    owner: Option<u64>,
) -> Option<&'module super::Module> {
    let owner = owner?;
    if owner == module.identity {
        return Some(module);
    }
    dependencies
        .iter()
        .copied()
        .find(|dependency| dependency.identity == owner)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; equality on the owned scheme vectors and effect set is PartialEq, which is not const."
)]
fn same_scheme(left: &noble_kernel::words::Scheme, right: &noble_kernel::words::Scheme) -> bool {
    left.var_kinds == right.var_kinds
        && left.stack_in == right.stack_in
        && left.stack_out == right.stack_out
        && left.effects == right.effects
}

// Test harness only: scenario-named tests over checked fixtures.
#[allow(
    tigerstyle::assertion_density,
    tigerstyle::function_length,
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; test harness: descriptive scenario names, fixture imports and long setup legs are the readable form."
)]
#[cfg(test)]
mod symbolic_owner_tests {
    use super::{ModuleKind, ModulePrepared, ModuleSession, RebuiltImportDefinition};
    use crate::source::Target;

    fn prepared_owner() -> ModulePrepared {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        let retained = crate::source::RetainedSourceLimit::new(limits.bytes)
            .expect("valid historical retained source bound");
        let session = ModuleSession::new_signed_self(&[], retained).expect("signed source session");
        let module = b"module nested@1 [ signature helper forall<S:stack> [ S -- S ! pure ] def helper [ 1 drop ] signature callable forall<S:stack> [ S -- S ! pure ] export callable def callable [ [ callable ] drop helper 1 drop ] ]";
        let staged = session
            .prepare(module, &[], limits)
            .expect("signed source module");
        let (session, outcome) = session.commit(staged);
        outcome.expect("signed module commit");
        session
            .prepare(b"nested@1.callable", &[], limits)
            .expect("signed expression")
    }

    #[test]
    fn symbolic_owner_certificate_rejects_missing_edge_forged_target_and_foreign_owner() {
        let mut prepared = prepared_owner();
        prepared
            .signed_source_owner()
            .expect("source owner")
            .rederive()
            .expect("retained complete nested source graph");
        let relation = prepared
            .staged
            .source
            .definitions
            .iter_mut()
            .find_map(|definition| definition.signed.as_mut())
            .expect("nested recursive owner relation");
        relation.symbolic_nodes.pop();
        assert!(
            prepared
                .signed_source_owner()
                .expect("source owner")
                .rederive()
                .is_err(),
            "a missing original node must not pass sealed signed ingress"
        );

        let mut forged = prepared_owner();
        let relation = forged
            .staged
            .source
            .definitions
            .iter_mut()
            .find_map(|definition| definition.signed.as_mut())
            .expect("nested recursive owner relation");
        let self_node = relation
            .symbolic_nodes
            .iter_mut()
            .filter_map(Option::as_mut)
            .find(|node| node.target == Some(Target::SelfOwner))
            .expect("nested owner edge");
        self_node.target = Some(Target::Builtin(0));
        assert!(
            forged
                .signed_source_owner()
                .expect("source owner")
                .rederive()
                .is_err(),
            "forging a coherent declaration target must not replace the lexical owner"
        );

        let mut foreign = prepared_owner();
        let definition = foreign
            .staged
            .source
            .definitions
            .iter_mut()
            .find(|definition| definition.signed.is_some())
            .expect("nested recursive owner definition");
        definition.owner = Some(u64::MAX);
        assert!(
            foreign
                .signed_source_owner()
                .expect("source owner")
                .rederive()
                .is_err(),
            "a foreign source owner must not authenticate the submitted signed edge"
        );
    }

    #[test]
    fn signed_namespace_alias_stays_inert_until_each_owner_has_a_concrete_certificate() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        let retained = crate::source::RetainedSourceLimit::new(limits.bytes)
            .expect("valid historical retained source bound");
        let session = ModuleSession::new_signed_self(&[], retained).expect("signed source session");
        let source = b"module dual@1 [ signature left forall<S:stack> [ S -- S ! pure ] export left def left [ left ] signature right forall<S:stack> [ S I64 -- S I64 ! pure ] export right def right [ right ] ]";
        let staged = session
            .prepare(source, &[], limits)
            .expect("two signed source owners");
        let (session, result) = session.commit(staged);
        result.expect("signed source-only module");
        let generation = session.generation();
        assert!(
            matches!(session.prepare(b"def capture [ dual@1.left ]", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Resolve)
        );
        assert!(matches!(session.prepare(
            b"module capture@1 [ export call def call [ dual@1.left ] ]", &[], limits,
        ), Err(error) if error.stage() == crate::source::Stage::Resolve));
        assert_eq!(
            session.generation(),
            generation,
            "an unverified versioned signed name must not publish a captured definition"
        );
        let pending = session
            .prepare(b"import dual@1 as d", &[], limits)
            .expect("pending inert import");
        assert!(
            pending.resolved_module().is_none(),
            "inert import cannot expose module"
        );
        let (session, result) = session.commit(pending);
        result.expect("pending inert import commit");
        assert!(
            matches!(session.prepare(b"def captured_alias [ d.left ]", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Resolve)
        );
        assert!(matches!(session.prepare(b"d.left", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Check
                || error.stage() == crate::source::Stage::Resolve));

        let checked_left = session
            .prepare(b"dual@1.left", &[], limits)
            .expect("concrete left owner certificate");
        let (session, result) = session.commit(checked_left);
        result.expect("left owner commit");
        assert!(
            session.prepare(b"d.left", &[], limits).is_err(),
            "one certificate must not activate an alias containing a second signed owner"
        );
        assert!(
            matches!(session.prepare(b"def capture [ dual@1.right ]", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Resolve)
        );
        assert!(matches!(session.prepare(b"1 d.right", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Resolve));

        let checked_right = session
            .prepare(b"1 dual@1.right", &[], limits)
            .expect("concrete right owner certificate");
        let (session, result) = session.commit(checked_right);
        result.expect("right owner commit");
        session
            .prepare(b"d.left", &[], limits)
            .expect("activated alias rechecks its own instance");
        session
            .prepare(b"1 d.right", &[], limits)
            .expect("second owner's alias checks its instance");
        session
            .prepare(b"def capture [ 1 dual@1.right drop ]", &[], limits)
            .expect("qualified capture is admitted only after both concrete certificates");
    }

    #[test]
    fn sealed_ingress_pairs_each_source_owner_with_its_original_signature() {
        let mut prepared = prepared_owner();
        let definition = prepared
            .staged
            .source
            .definitions
            .iter_mut()
            .find(|definition| definition.signed.is_some())
            .expect("retained signed source definition");
        definition
            .signature
            .as_mut()
            .expect("signed owner signature")
            .effects
            .push(noble_kernel::shapes::EffectSlot::Effect(
                noble_kernel::contracts::TEST_EMIT,
            ));
        assert!(
            prepared
                .signed_source_owner()
                .expect("sealed owner")
                .rederive()
                .is_err(),
            "a changed effect ceiling must not claim the original signed signature"
        );

        let mut forged = prepared_owner();
        let entry = forged
            .submission
            .as_mut()
            .expect("submitted signed definition")
            .definitions
            .iter_mut()
            .find(|definition| definition.owner_inst.is_some())
            .expect("signed owner specialization");
        entry.source_identity = u64::MAX;
        assert!(
            forged
                .signed_source_owner()
                .expect("sealed owner")
                .rederive()
                .is_err(),
            "same-module signed owner may not borrow another source identity"
        );
    }

    #[test]
    fn same_cap_foreign_profile_cannot_publish_a_prepared_module() {
        let limits = crate::Limits::default();
        let retained = crate::source::RetainedSourceLimit::new(limits.bytes)
            .expect("valid historical retained source bound");
        let ordinary = ModuleSession::new(&[], retained).expect("ordinary declared session");
        let signed_text = ModuleSession::new_declared_text(&[], true, retained)
            .expect("signed declared text session");
        let prepared = signed_text
            .prepare(
                b"module foreign@1 [ export value def value [ 1 ] ]",
                &[],
                limits,
            )
            .expect("foreign module proposal");
        let (ordinary, outcome) = ordinary.commit(prepared);
        assert!(matches!(outcome, Err(error)
            if error.stage() == crate::source::Stage::Acceptance));
        assert_eq!(ordinary.generation(), 0);
        assert!(!ordinary.declared_text());
        assert!(matches!(ordinary.prepare(b"foreign@1.value", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Resolve));
        assert!(matches!(ordinary.prepare(b"1 2 <", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Resolve));
    }

    #[test]
    fn same_profile_foreign_history_cannot_replace_captured_owner_graph() {
        let limits = crate::Limits::default();
        let retained = crate::source::RetainedSourceLimit::new(limits.bytes)
            .expect("valid historical retained source bound");
        let mut prior = ModuleSession::new(&[], retained).expect("first declared session");
        let mut foreign = ModuleSession::new(&[], retained).expect("second declared session");
        let publish = |session: ModuleSession, source: &[u8]| {
            let prepared = session.prepare(source, &[], limits).expect("checked unit");
            let (next, result) = session.commit(prepared);
            result.expect("publish checked unit");
            next
        };
        for source in [
            b"module m@1 [ opaque Item I64 public export Item export f def f [ 1 ] ]".as_slice(),
            b"module m@2 [ opaque Item I64 public export Item export f def f [ 2 ] ]".as_slice(),
        ] {
            prior = publish(prior, source);
            foreign = publish(foreign, source);
        }
        prior = publish(prior, b"import m@1 as a");
        prior = publish(prior, b"def d [ a.f ]");
        prior = publish(prior, b"import m@2 as a");
        foreign = publish(foreign, b"import m@2 as a");
        foreign = publish(foreign, b"def d [ a.f ]");
        foreign = publish(foreign, b"def d [ a.f ]");

        assert_eq!(prior.generation(), foreign.generation());
        assert_eq!(prior.modules, foreign.modules);
        assert_eq!(prior.aliases, foreign.aliases);
        assert_eq!(prior.source.history, foreign.source.history);
        assert_ne!(prior.source.definitions, foreign.source.definitions);
        let prior_old = prior
            .source
            .definitions
            .iter()
            .find(|definition| definition.name == "d")
            .and_then(|definition| definition.alias_uses.first())
            .expect("original old-version alias owner")
            .owner;
        let foreign_old = foreign
            .source
            .definitions
            .iter()
            .find(|definition| definition.name == "d")
            .and_then(|definition| definition.alias_uses.first())
            .expect("foreign new-version alias owner")
            .owner;
        assert_eq!(prior_old, 1);
        assert_eq!(foreign_old, 2);

        let baseline = prior.snapshot();
        let prepared = foreign
            .prepare(b"module extra@1 [ export f def f [ 3 ] ]", &[], limits)
            .expect("same-profile foreign module proposal");
        let (prior, outcome) = prior.commit(prepared);
        assert!(matches!(outcome, Err(error)
            if error.stage() == crate::source::Stage::Acceptance));
        assert_eq!(prior.generation(), baseline.generation());
        assert_eq!(prior.source.definitions, baseline.source.definitions);
        assert_eq!(prior.source.history, baseline.source.history);
        assert_eq!(
            prior
                .source
                .definitions
                .iter()
                .find(|definition| definition.name == "d")
                .and_then(|definition| definition.alias_uses.first())
                .expect("preserved original historical owner")
                .owner,
            1
        );
        assert_eq!(
            prior.resolve_type("m@1.Item").expect("old versioned owner"),
            baseline
                .resolve_type("m@1.Item")
                .expect("original old owner")
        );
        assert_eq!(
            prior
                .resolve_type("a.Item")
                .expect("current new-version alias"),
            baseline
                .resolve_type("m@2.Item")
                .expect("original new owner")
        );
        prior
            .prepare(b"m@1.f a.f", &[], limits)
            .expect("both original version-qualified owners remain checked");
        assert!(matches!(prior.prepare(b"extra@1.f", &[], limits),
            Err(error) if error.stage() == crate::source::Stage::Resolve));
    }

    #[test]
    fn certified_signed_rebind_preserves_original_sources_and_captured_old_program() {
        let limits = crate::Limits::default();
        let retained =
            crate::source::RetainedSourceLimit::new(crate::source::RetainedSourceLimit::MAX_BYTES)
                .expect("valid opt-in bound");
        let mut session =
            ModuleSession::new_declared_text(&[], true, retained).expect("signed declared session");
        let publish = |session: ModuleSession, source: &[u8]| {
            let prepared = session
                .prepare(source, &[], limits)
                .expect("checked source");
            if prepared.kind() == ModuleKind::Expression {
                prepared
                    .signed_source_owner()
                    .expect("sealed source")
                    .rederive()
                    .expect("original signed self source");
            }
            let (next, result) = session.commit(prepared);
            result.expect("published checked unit");
            next
        };
        let first = b"module m@1 [ signature addone forall<S:stack> [ S I64 -- S I64 ! pure ] export addone def addone [ dup 0 = [ 1 + ] [ 1 - addone 1 + ] if ] ]";
        let second = b"module m@2 [ signature addone forall<S:stack> [ S I64 -- S I64 ! pure ] export addone def addone [ dup 0 = [ 2 + ] [ 1 - addone 1 + ] if ] ]";
        let once = b"def once [ let x x m.addone ]";
        let twice = b"def twice [ once once ]";
        session = publish(session, first);
        session = publish(session, b"0 m@1.addone drop");
        session = publish(session, second);
        session = publish(session, b"0 m@2.addone drop");
        assert!(!session.modules[0].verified_signed_slots.is_empty());
        assert!(!session.modules[1].verified_signed_slots.is_empty());
        session = publish(session, b"import m@1 as m");
        session = publish(session, b"import m@1 as other");
        session = publish(session, once);
        session = publish(session, twice);
        let old_program = session
            .prepare(b"[ once ]", &[], limits)
            .expect("captured old checked Program");
        old_program
            .signed_source_owner()
            .expect("signed old Program")
            .rederive()
            .expect("original old module signed body");
        session = publish(session, b"[ once ]");
        let before = session.snapshot();
        let mut original_history = alloc::vec::Vec::new();
        for source in [once.as_slice(), twice.as_slice()] {
            original_history.extend_from_slice(
                &u32::try_from(source.len())
                    .expect("bounded definition")
                    .to_le_bytes(),
            );
            original_history.extend_from_slice(source);
        }
        assert_eq!(before.modules[0].source.as_slice(), first);
        assert_eq!(before.modules[1].source.as_slice(), second);
        assert_eq!(before.source.history, original_history);
        assert_eq!(
            before
                .source
                .definitions
                .iter()
                .find(|definition| definition.name == "once")
                .and_then(|definition| definition.alias_uses.first())
                .expect("old resolved alias")
                .owner,
            1
        );

        let prepared = session
            .prepare(b"import m@2 as m", &[], limits)
            .expect("transitively checked alias rebind");
        assert_eq!(
            prepared
                .rebuilt_import_definitions()
                .expect("complete dependent replay")
                .iter()
                .map(RebuiltImportDefinition::name)
                .collect::<alloc::vec::Vec<_>>(),
            ["once", "twice"]
        );
        let (after, result) = session.commit(prepared);
        result.expect("publish complete rebind");
        assert_eq!(after.modules[0].source.as_slice(), first);
        assert_eq!(after.modules[1].source.as_slice(), second);
        let mut replayed_history = original_history.clone();
        replayed_history.extend_from_slice(&original_history);
        assert_eq!(
            after.source.history, replayed_history,
            "both exact original framed bodies remain alongside exact charged replay copies"
        );
        assert_eq!(
            after
                .source
                .definitions
                .get(..before.source.definitions.len()),
            Some(before.source.definitions.as_slice()),
            "older resolved owner graph must remain immutable"
        );
        assert_eq!(
            after
                .source
                .definitions
                .iter()
                .rev()
                .find(|definition| definition.name == "once")
                .and_then(|definition| definition.alias_uses.first())
                .expect("new resolved alias")
                .owner,
            2
        );
        assert_eq!(
            after
                .aliases
                .iter()
                .find(|alias| alias.spelling == "other")
                .expect("preserved old alias")
                .module,
            0
        );
        assert_eq!(
            after
                .aliases
                .iter()
                .find(|alias| alias.spelling == "m")
                .expect("new current alias")
                .module,
            1
        );
        let sealed_replay = old_program
            .signed_source_owner()
            .expect("captured signed Program")
            .rederive()
            .expect("old checked Program retains original signed owner");
        assert_eq!(
            sealed_replay.environment.definition_owners,
            old_program
                .submission()
                .expect("original checked Program")
                .environment
                .definition_owners
        );
        after
            .prepare(b"m@1.addone", &[noble_kernel::types::Ty::I64], limits)
            .expect("old version-qualified owner still checked");
        after
            .prepare(b"m.addone", &[noble_kernel::types::Ty::I64], limits)
            .expect("new current alias checked");
    }
}
