//! The bounded WIT boundary. WIT is parsed, not replaced by a
//! parallel IDL. Unsupported shapes fail before source or Wasm emission.
mod bindings;
mod context;
mod parser;
mod preparation;

pub const PROFILE: &str = "Component-Sync-Bootstrap";
pub const ASYNC_PROFILE: &str = "Component-Async-Bootstrap";
pub const MAX_OPERATIONS: usize = 62;
pub const MAX_RESOURCES: usize = 32;
pub const MAX_PARAMETERS: usize = 16;

// Declared resources occupy 1..=MAX_RESOURCES, including declared witnesses.
// Live async kinds have fixed identities independent of declaration order.
pub const STREAM_U8_KIND: noble_kernel::types::ResourceKind =
    noble_kernel::types::ResourceKind(u32::MAX.saturating_sub(2));
pub const FUTURE_S64_KIND: noble_kernel::types::ResourceKind =
    noble_kernel::types::ResourceKind(u32::MAX.saturating_sub(1));
pub const FUTURE_RESULT_S64_STRING_KIND: noble_kernel::types::ResourceKind =
    noble_kernel::types::ResourceKind(u32::MAX);

const BINDING_SCHEMA: &str = "WIT-Bounded-v3:bool,s64,string,list<u8>,result<s64,string>,result<list<u8>,string>,own,borrow,stream<u8>,future<s64>,future<result<s64,string>>";
const CHECKED_U64_SCHEMA: &str = "checked-u64-to-I64:0..=9223372036854775807";
const SYNC_ABI: &str = "Canonical-Sync-cm32p2";
const ASYNC_ABI: &str = "Canonical-Async-Legacy-Lower-WaitableSet-LiftStackful-TaskReturn-v1";

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Sync,
    Async,
}

impl Profile {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sync => PROFILE,
            Self::Async => ASYNC_PROFILE,
        }
    }
}

/// The adapter profile is closed: new types require an exact Noble mapping
/// and a separately reviewed Canonical ABI adapter.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Boolean,
    S64,
    /// WIT u64, only in a host-selected, range-checked I64 boundary recipe.
    CheckedU64,
    String,
    Bytes,
    ResultS64String,
    ResultBytesString,
    StreamU8,
    FutureS64,
    FutureResultS64String,
    Own(noble_kernel::types::ResourceKind),
    Borrow(noble_kernel::types::ResourceKind),
}

impl Type {
    /// A type identity, not a live handle or an authorization witness.
    pub const fn resource_kind(self) -> Option<noble_kernel::types::ResourceKind> {
        match self {
            Self::StreamU8 => Some(STREAM_U8_KIND),
            Self::FutureS64 => Some(FUTURE_S64_KIND),
            Self::FutureResultS64String => Some(FUTURE_RESULT_S64_STRING_KIND),
            Self::Own(kind) | Self::Borrow(kind) => Some(kind),
            Self::Boolean
            | Self::S64
            | Self::CheckedU64
            | Self::String
            | Self::Bytes
            | Self::ResultS64String
            | Self::ResultBytesString => None,
        }
    }

    /// Bytes use checked u8 conversions at the boundary, never I64 truncation.
    /// Borrow maps to its guest owner only; no borrow token enters guest types.
    /// Streams and futures use the same move-only discipline as owned resources.
    pub fn noble(self) -> Option<noble_kernel::types::Ty> {
        Some(match self {
            Self::Boolean => noble_kernel::types::Ty::Bool,
            Self::S64 => noble_kernel::types::Ty::I64,
            Self::CheckedU64 => return None,
            Self::String => noble_kernel::types::Ty::Text,
            Self::Bytes => {
                noble_kernel::types::Ty::List(alloc::boxed::Box::new(noble_kernel::types::Ty::I64))
            }
            Self::ResultS64String => noble_kernel::types::Ty::Sum(
                alloc::boxed::Box::new(noble_kernel::types::Ty::I64),
                alloc::boxed::Box::new(noble_kernel::types::Ty::Text),
            ),
            Self::ResultBytesString => noble_kernel::types::Ty::Sum(
                alloc::boxed::Box::new(noble_kernel::types::Ty::List(alloc::boxed::Box::new(
                    noble_kernel::types::Ty::I64,
                ))),
                alloc::boxed::Box::new(noble_kernel::types::Ty::Text),
            ),
            Self::StreamU8 => noble_kernel::types::Ty::Resource(STREAM_U8_KIND),
            Self::FutureS64 => noble_kernel::types::Ty::Resource(FUTURE_S64_KIND),
            Self::FutureResultS64String => {
                noble_kernel::types::Ty::Resource(FUTURE_RESULT_S64_STRING_KIND)
            }
            Self::Own(kind) | Self::Borrow(kind) => noble_kernel::types::Ty::Resource(kind),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Resource {
    pub identity: alloc::string::String,
    pub interface: alloc::string::String,
    pub name: alloc::string::String,
    pub kind: noble_kernel::types::ResourceKind,
}

#[derive(Clone, Debug)]
pub struct Operation {
    /// Exact package/interface/version/function semantic identity.
    pub identity: alloc::string::String,
    /// Generated Noble spelling, e.g. arithmetic.inc or counter.read.
    pub word: alloc::string::String,
    /// Standard core Canonical ABI module and function names.
    pub core_module: alloc::string::String,
    pub core_name: alloc::string::String,
    /// Direct world name, or versioned interface#function for interface exports.
    pub export_name: alloc::string::String,
    pub parameters: alloc::vec::Vec<Type>,
    pub results: alloc::vec::Vec<Type>,
    /// Native suspension preserves the direct-style sequential Noble contract.
    pub asynchronous: bool,
    /// Import-only identities; export contracts do not invent host effects.
    pub effect: Option<noble_kernel::types::EffId>,
    pub definition: Option<noble_kernel::contracts::Definition>,
}

impl Operation {
    pub fn input_types(&self, world: &World) -> Option<alloc::vec::Vec<noble_kernel::types::Ty>> {
        let mut result = alloc::vec::Vec::with_capacity(self.parameters.len());
        let mut at = 0usize;
        while at < self.parameters.len() {
            result.push(world.noble_type(self.parameters[at])?);
            at = at.saturating_add(1);
        }
        Some(result)
    }
    /// Thread borrowed owners before results, in original parameter order.
    /// Owned parameters are NOT returned unless WIT explicitly returns ownership.
    pub fn output_types(&self, world: &World) -> Option<alloc::vec::Vec<noble_kernel::types::Ty>> {
        let mut result = alloc::vec::Vec::with_capacity(
            self.parameters.len().saturating_add(self.results.len()),
        );
        let mut at = 0usize;
        while let Some(ty) = self.parameters.get(at) {
            if let Type::Borrow(kind) = ty {
                result.push(noble_kernel::types::Ty::Resource(*kind));
            }
            at = at.saturating_add(1);
        }
        at = 0;
        while at < self.results.len() {
            result.push(world.noble_type(self.results[at])?);
            at = at.saturating_add(1);
        }
        Some(result)
    }
}

#[derive(Clone, Debug)]
pub struct World {
    identity: alloc::string::String,
    name: alloc::string::String,
    wit: alloc::vec::Vec<u8>,
    imports: alloc::vec::Vec<Operation>,
    exports: alloc::vec::Vec<Operation>,
    resources: alloc::vec::Vec<Resource>,
    asynchronous: bool,
    checked_u64: bool,
    worker_program_kind: Option<noble_kernel::types::ResourceKind>,
    worker_program_effect: Option<noble_kernel::types::EffId>,
    /// Opaque move-only owner for a host-typed transition Program. Unlike the
    /// I64 Program world, the guest sees only this Resource: canonical frame
    /// strings are typed independently by the selected host schema.
    worker_transition_kind: Option<noble_kernel::types::ResourceKind>,
    limits: crate::Limits,
}

impl World {
    pub fn parse(wit: &[u8], world: &str, limits: crate::Limits) -> Result<Self, Error> {
        parser::parse(wit, world, limits, false)
    }
    /// Select the partial, checked u64↔I64 boundary explicitly. WIT values
    /// outside the signed range trap before becoming Noble values or results.
    pub fn parse_checked_u64(
        wit: &[u8],
        world: &str,
        limits: crate::Limits,
    ) -> Result<Self, Error> {
        parser::parse(wit, world, limits, true)
    }
    pub const fn checked_u64(&self) -> bool {
        self.checked_u64
    }
    /// Opt in only with a checked, owner-bound Program type from the source
    /// session. This selects the semantic WIT type, not an executable handle;
    /// a host still has to authenticate and install the actual Program cell.
    pub fn select_worker_program(
        self,
        source: &crate::source::CheckedProgramInterface,
    ) -> Result<Self, Error> {
        if !source.is_pure_i64_endomorphism() {
            return Err(unsupported("Worker WIT requires checked pure I64 Program"));
        }
        self.select_worker_program_shape(None)
    }

    /// Opt in to the separate, source-checked I64 Program bound exactly to
    /// test.emit. This is a WIT type selection, never a host operation grant.
    pub fn select_worker_program_for_test_emit(
        self,
        source: &crate::source::CheckedProgramInterface,
    ) -> Result<Self, Error> {
        if !source.is_test_emit_i64_endomorphism() {
            return Err(unsupported(
                "Worker WIT requires checked I64 Program bound to test.emit",
            ));
        }
        self.select_worker_program_shape(Some(noble_kernel::contracts::TEST_EMIT))
    }

    /// The deny-only captured-Text export request has its own exact checked
    /// source effect. WIT selection alone does not authorize publication.
    pub fn select_worker_program_for_test_code_export(
        self,
        source: &crate::source::CheckedProgramInterface,
    ) -> Result<Self, Error> {
        if !source.is_test_code_export_i64_endomorphism() {
            return Err(unsupported(
                "Worker WIT requires checked test.code-export Program",
            ));
        }
        self.select_worker_program_shape(Some(noble_kernel::contracts::TEST_CODE_EXPORT))
    }

    /// The host-observed send request has its own exact checked source
    /// effect. WIT selection alone authorizes, admits and observes nothing.
    pub fn select_worker_program_for_test_send(
        self,
        source: &crate::source::CheckedProgramInterface,
    ) -> Result<Self, Error> {
        if !source.is_test_send_i64_endomorphism() {
            return Err(unsupported("Worker WIT requires checked test.send Program"));
        }
        self.select_worker_program_shape(Some(noble_kernel::contracts::TEST_SEND))
    }

    /// Every ordered admission rule stays a separate guard so the shape reads
    /// as the exact operation/ownership table rather than one compound test.
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the predicate indexes owned Vec slices and compares owned Strings, which const fn cannot do here."
    )]
    fn has_transition_operation_shape(&self, kind: noble_kernel::types::ResourceKind) -> bool {
        if self.resources.len() != 1 || self.imports.len() != 2 {
            return false;
        }
        if self.exports.len() != 1 {
            return false;
        }
        let admit = &self.imports[0];
        if admit.identity != "noble-test:worker-transition/programs@1.0.0#admit" {
            return false;
        }
        if !admit.parameters.is_empty() || admit.results != [Type::Own(kind)] {
            return false;
        }
        let invoke = &self.imports[1];
        if invoke.identity != "noble-test:worker-transition/programs@1.0.0#invoke" {
            return false;
        }
        if invoke.parameters != [Type::String, Type::String, Type::Own(kind)]
            || invoke.results != [Type::String]
        {
            return false;
        }
        let step = &self.exports[0];
        if step.identity != "noble-test:worker-transition/guest@1.0.0#step" {
            return false;
        }
        step.parameters == [Type::String, Type::String] && step.results == [Type::String]
    }

    /// Select the frame-transport Worker transition world only for the exact
    /// host-checked `State Event -- State List<Action> ! {}` Program. WIT
    /// strings and the opaque owner carry no schema authority; the embedder
    /// validates both frames against `state`, `event` and `action`.
    pub fn select_worker_transition(
        mut self,
        source: &crate::source::CheckedProgramInterface,
        state: &noble_kernel::types::Ty,
        event: &noble_kernel::types::Ty,
        action: &noble_kernel::types::Ty,
    ) -> Result<Self, Error> {
        let expected = noble_kernel::types::Ty::program(
            alloc::vec![state.clone(), event.clone()],
            alloc::vec![
                state.clone(),
                noble_kernel::types::Ty::List(alloc::boxed::Box::new(action.clone())),
            ],
            noble_kernel::types::EffSet::empty(),
        );
        if source.ty() != expected || self.identity != "noble-test:worker-transition/guest@1.0.0" {
            return Err(unsupported(
                "Worker transition WIT requires the checked schema Program",
            ));
        }
        if self.asynchronous || self.checked_u64 {
            return Err(unsupported(
                "Worker transition WIT requires the checked schema Program",
            ));
        }
        let mut matches = self.resources.iter().filter(|resource| {
            resource.identity == "noble-test:worker-transition/programs@1.0.0#program"
        });
        let resource = matches
            .next()
            .ok_or_else(|| unsupported("selected transition Program resource is absent"))?;
        if matches.next().is_some() {
            return Err(invalid("duplicate transition Program resource"));
        }
        let kind = resource.kind;
        if !self.has_transition_operation_shape(kind) {
            return Err(unsupported(
                "Worker transition WIT operation or ownership shape differs",
            ));
        }
        self.worker_transition_kind = Some(kind);
        attempt!(bindings::make(&self));
        Ok(self)
    }

    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; each compound guard states one ordered admission rule verbatim; extracting its operands would change short-circuit evaluation."
    )]
    fn select_worker_program_shape(
        mut self,
        effect: Option<noble_kernel::types::EffId>,
    ) -> Result<Self, Error> {
        if self.identity != "noble-test:worker-program/guest@2.0.0"
            || self.asynchronous
            || self.checked_u64
        {
            return Err(unsupported("Worker Program WIT world differs"));
        }
        let mut matches = self.resources.iter().filter(|resource| {
            resource.identity == "noble-test:worker-program/programs@2.0.0#program"
        });
        let resource = matches
            .next()
            .ok_or_else(|| unsupported("selected WIT Program resource is absent"))?;
        if matches.next().is_some() {
            return Err(invalid("duplicate WIT Program resource"));
        }
        let kind = resource.kind;
        if self.resources.len() != 1
            || self.imports.len() != 3
            || self.exports.len() != 1
            || self.imports[0].identity != "noble-test:worker-program/programs@2.0.0#admit"
            || !self.imports[0].parameters.is_empty()
            || self.imports[0].results != [Type::Own(kind)]
            || self.imports[1].identity != "noble-test:worker-program/programs@2.0.0#invoke"
            || self.imports[1].parameters != [Type::Own(kind), Type::S64]
            || self.imports[1].results != [Type::S64]
            || self.imports[2].identity
                != "noble-test:worker-program/programs@2.0.0#request-code-export"
            || self.imports[2].parameters != [Type::Own(kind), Type::String]
            || self.imports[2].results != [Type::Boolean]
            || self.exports[0].identity != "noble-test:worker-program/guest@2.0.0#compute"
            || self.exports[0].parameters != [Type::S64]
            || self.exports[0].results != [Type::S64]
        {
            return Err(unsupported(
                "Worker Program WIT operation or ownership shape differs",
            ));
        }
        self.worker_program_kind = Some(kind);
        self.worker_program_effect = effect;
        attempt!(bindings::make(&self));
        Ok(self)
    }

    pub const fn worker_program_kind(&self) -> Option<noble_kernel::types::ResourceKind> {
        self.worker_program_kind
    }
    pub const fn worker_transition_kind(&self) -> Option<noble_kernel::types::ResourceKind> {
        self.worker_transition_kind
    }
    /// Checked unsigned projection and Worker Program typing require explicit
    /// selection; neither installs a host operation or an executable cell.
    pub fn noble_type(&self, ty: Type) -> Option<noble_kernel::types::Ty> {
        match ty {
            Type::CheckedU64 if self.checked_u64 => Some(noble_kernel::types::Ty::I64),
            Type::CheckedU64 => None,
            Type::Own(kind) if self.worker_program_kind == Some(kind) => {
                Some(noble_kernel::types::Ty::program(
                    alloc::vec![noble_kernel::types::Ty::I64],
                    alloc::vec![noble_kernel::types::Ty::I64],
                    match self.worker_program_effect {
                        Some(effect) => noble_kernel::types::EffSet::from_ids(&[effect]),
                        None => noble_kernel::types::EffSet::empty(),
                    },
                ))
            }
            Type::Borrow(kind) if self.worker_program_kind == Some(kind) => None,
            _ => ty.noble(),
        }
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn wit(&self) -> &[u8] {
        &self.wit
    }
    pub fn imports(&self) -> &[Operation] {
        &self.imports
    }
    pub fn exports(&self) -> &[Operation] {
        &self.exports
    }
    pub fn resources(&self) -> &[Resource] {
        &self.resources
    }
    /// Native async ABI is also required by live stream and future values.
    pub const fn is_async(&self) -> bool {
        self.asynchronous
    }
    pub const fn profile(&self) -> Profile {
        if self.asynchronous {
            Profile::Async
        } else {
            Profile::Sync
        }
    }
    pub const fn limits(&self) -> crate::Limits {
        self.limits
    }
    /// Independently reconstruct the exact generated import environment.
    pub fn environment(&self) -> Result<noble_kernel::contracts::Env, Error> {
        Ok(attempt!(bindings::make(self)).environment)
    }
    pub fn session(&self) -> Result<crate::source::Session, Error> {
        Ok(crate::source::Session::with_bindings(attempt!(
            bindings::make(self)
        )))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Bindings {
    pub(crate) environment: noble_kernel::contracts::Env,
    pub(crate) words: alloc::vec::Vec<(alloc::string::String, noble_kernel::contracts::Definition)>,
    /// Only independently imported WIT operations are callable guest effects.
    pub(crate) effects: u64,
    /// The selected Program's latent type effect is not a callable import.
    pub(crate) program_type_effects: u64,
    pub(crate) key: alloc::vec::Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Wit,
    Binding,
    Export,
    Acceptance,
}

#[derive(Clone, Debug)]
pub struct Error {
    pub stage: Stage,
    pub diagnostic: crate::Diagnostic,
}

fn error(stage: Stage, kind: crate::DiagnosticKind, message: &str) -> Error {
    Error {
        stage,
        diagnostic: crate::Diagnostic::new(kind, crate::Span { start: 0, end: 0 }, message),
    }
}
fn invalid(message: &str) -> Error {
    error(Stage::Wit, crate::DiagnosticKind::Invalid, message)
}
fn unsupported(message: &str) -> Error {
    error(Stage::Wit, crate::DiagnosticKind::Unsupported, message)
}
fn exhausted() -> Error {
    error(
        Stage::Wit,
        crate::DiagnosticKind::Exhausted,
        "bounded WIT compiler limit exceeded",
    )
}

/// Only actual source preparation can construct an export. All getters borrow.
pub struct CheckedExport {
    world: alloc::vec::Vec<u8>,
    index: usize,
    source_bytes: alloc::vec::Vec<u8>,
    prepared: crate::source::Prepared,
}
impl CheckedExport {
    pub fn world_context(&self) -> &[u8] {
        &self.world
    }
    #[expect(
        tigerstyle::usize_in_public_api,
        reason = "Owner: noble-maintainers; this retained index selects the same World's borrowed exports slice, is never serialized, and is bounded by MAX_OPERATIONS during parsing; preserve the existing slice-index API."
    )]
    pub const fn index(&self) -> usize {
        self.index
    }
    pub fn source(&self) -> &[u8] {
        &self.source_bytes
    }
    pub fn submission(&self) -> Option<&noble_kernel::execution::Submission> {
        self.prepared.submission()
    }
}
