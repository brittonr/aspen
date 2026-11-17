//! The checking environment: one scheme per definition, its behavior kind,
//! its external dependency data, the user-declared schemas, and the
//! supplied effect-identity table.
//!
//! Contracts are rank-1 schemes; the bootstrap table lives in `bootstrap`.

pub mod bootstrap;
mod clone_budget;
mod live;
mod nominal;
pub use clone_budget::{
    CloneBudget, CALCULATOR_ENVIRONMENT_ROW_LIMIT, CALCULATOR_ENV_CLONE_BYTES_LIMIT,
    CALCULATOR_ENV_CLONE_FANOUT_LIMIT,
};

/// Identity of one environment definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Definition(pub u32);

fn empty_dependency_list() -> alloc::vec::Vec<Definition> {
    alloc::vec::Vec::new()
}

/// The reserved identity of the resource-free `test.emit` effect.
pub const TEST_EMIT: crate::types::EffId = crate::types::EffId(0);
/// The reserved identity of the opt-in scripted `test.clock` effect.
pub const TEST_CLOCK: crate::types::EffId = crate::types::EffId(2);
/// The reserved identity of the required, deny-only `test.code-export` effect.
pub const TEST_CODE_EXPORT: crate::types::EffId = crate::types::EffId(6);
/// Host-selected identity of the deny-only WORKER-12 source adapter.
pub const TEST_CODE_EXPORT_ADAPTER_IDENTITY: &str = "worker-test-code-export@1.0.0";
/// The reserved identity of the required, host-observed `test.send` effect.
pub const TEST_SEND: crate::types::EffId = crate::types::EffId(7);
/// Host-selected identity of the WORKER-10 observed-send source adapter.
pub const TEST_SEND_ADAPTER_IDENTITY: &str = "worker-test-send@1.0.0";
/// The reserved, opt-in effect of selecting a preadmitted live program.
pub const LIVE_DISPATCH: crate::types::EffId = crate::types::EffId(5);

/// The reserved resource kind used by negative eligibility fixtures.
pub const FIXTURE_RESOURCE: crate::types::ResourceKind = crate::types::ResourceKind(0);

/// Identity of one user-declared schema in external environment data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemaId(pub u32);

/// What a definition's contract constrains beyond its scheme.
///
/// The extracted constructors carry a `Behavior` suffix: `Unit` would
/// otherwise shadow Lean's own `Unit` inside every declaration named
/// `contracts.Behavior.*`, where this type's instances live.
#[charon::variants_suffix("Behavior")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    /// `dup`: requires `Data` on its value variable.
    Dup,
    /// `drop`: requires `Data` on its value variable.
    Drop,
    /// `swap`.
    Swap,
    /// `dip`.
    Dip,
    /// `+`, `-`, or `*`; wrapping `I64`.
    Arith,
    /// `=`: `I64` equality yielding `Bool`.
    Equals,
    /// `quote`: requires `Data` on its captured variable.
    Quote,
    /// `compose`.
    Compose,
    /// `run`.
    Run,
    /// `reflect`.
    Reflect,
    /// `unit`.
    Unit,
    /// `pair`.
    Pair,
    /// `unpair`.
    Unpair,
    /// `inl`.
    Inl,
    /// `inr`.
    Inr,
    /// `case`.
    Case,
    /// `if`.
    If,
    /// `nil`.
    Nil,
    /// `cons`.
    Cons,
    /// `list.case`.
    ListCase,

    /// The supplied `test.emit` operation.
    TestEmit,
    /// Pure byte-offset inspection in standalone cursor or calculator.
    TextByte,
    /// Pure signed comparison, visible only to the combined declared-text profile.
    I64Less,
    /// Checked signed addition in the opt-in checked-I64 profile.
    CheckedI64Add,
    /// Checked signed subtraction in the opt-in checked-I64 profile.
    CheckedI64Sub,
    /// Checked signed multiplication in the opt-in checked-I64 profile.
    CheckedI64Mul,
    /// Checked signed negation in the opt-in checked-I64 profile.
    CheckedI64Neg,
    /// Checked signed absolute value in the opt-in checked-I64 profile.
    CheckedI64Abs,
    /// Pure signed comparison in the opt-in checked-I64 profile.
    CheckedI64Less,
    /// Checked truncating signed quotient and remainder in the opt-in profile.
    CheckedI64DivRem,
    /// Pure, budget-metered guest iteration in the opt-in calculator profile.
    CalculatorIterate,
    /// Signed comparison returning -1, 0 or 1.
    CalculatorCompare,
    /// Checked nonnegative integer quotient and remainder.
    CalculatorDivRem,
    /// One-pass checked ASCII construction from a list of bytes.
    CalculatorTextFromAscii,
    /// Constant-time immutable Text byte length for guest budget preflight.
    CalculatorTextLength,
    /// Non-callable slots keeping builtin identities disjoint between profiles.
    Reserved,
    /// A required `test.emit` binding, retaining the immutable adapter slot.
    BoundEmit(u32),
    /// A required scripted `test.clock` binding, retaining its immutable slot.
    BoundClock(u32),
    /// A required `test.code-export` binding, retaining its immutable slot.
    BoundCodeExport(u32),
    /// A required `test.send` binding, retaining its immutable slot.
    BoundSend(u32),
    /// Explicit opaque construction and conversion.
    NominalNew(crate::types::NominalTypeId),
    NominalInto(crate::types::NominalTypeId),
    /// Ordered variant constructors and exhaustive elimination.
    NominalLeft(crate::types::NominalTypeId),
    NominalRight(crate::types::NominalTypeId),
    NominalMatch(crate::types::NominalTypeId),
    /// Source-declared rank-1 variant family operations, shared by every
    /// concrete instantiation of the family.
    GenericLeft(crate::types::NominalTypeId),
    GenericRight(crate::types::NominalTypeId),
    GenericMatch(crate::types::NominalTypeId),
    /// An environment-supplied named definition with no extra constraint.
    Named,
}

/// One user-declared schema in external environment data.
///
/// A declaration that names itself — a recursive schema — is unsupported
/// (B-CHECK-02) and rejected during preflight; a non-recursive declaration
/// is still validated as a scheme before any candidate body is checked.
#[derive(Clone, Debug)]
pub struct SchemaDecl {
    /// The declared schema's identity.
    pub id: SchemaId,
    /// The declared contract.
    pub scheme: crate::words::Scheme,
    /// Whether the declaration names itself.
    pub recursive: bool,
}

/// Checked descriptor independently supplied as part of a module interface.
#[derive(Clone, Debug)]
pub struct NominalDecl {
    pub id: crate::types::NominalTypeId,
    pub shape: crate::types::NominalShape,
    /// Export of the type itself, distinct from constructor visibility.
    pub exported: bool,
    /// Opaque: `new`/`into`. Variant: left/right constructors, in order.
    pub public: [bool; 2],
}

/// A source-declared family with exactly two type parameters. Each arm's
/// payload is the indicated parameter (0 or 1), in source-declared order.
#[derive(Clone, Debug)]
pub struct GenericVariantDecl {
    pub id: crate::types::NominalTypeId,
    pub payload_params: [u8; 2],
    pub exported: bool,
    pub public: [bool; 2],
}

/// Definitions assigned by `declare_nominal` in the environment.
#[derive(Clone, Copy, Debug)]
pub struct NominalOps {
    pub new: Option<Definition>,
    pub into: Option<Definition>,
    pub left: Option<Definition>,
    pub right: Option<Definition>,
    pub matcher: Option<Definition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NominalError {
    DuplicateIdentity,
    InvalidRepresentation,
    TooManyDefinitions,
}

/// Trusted manifest mapping for one required host operation.
#[derive(Clone, Debug)]
pub struct BoundAdapter {
    pub definition: Definition,
    pub adapter_identity: alloc::string::String,
    pub adapter_slot: u32,
    pub input: alloc::vec::Vec<crate::types::Ty>,
    pub output: alloc::vec::Vec<crate::types::Ty>,
    pub effects: crate::types::EffSet,
}

/// Pending required host operation, before a definition identity is assigned.
pub struct BoundEmitRegistration {
    pub adapter_identity: alloc::string::String,
    pub adapter_slot: u32,
    pub owner: u64,
    pub input: alloc::vec::Vec<crate::types::Ty>,
    pub output: alloc::vec::Vec<crate::types::Ty>,
    pub effects: crate::types::EffSet,
}

/// Pending scripted clock binding; not authority to consult a real clock.
pub struct BoundClockRegistration {
    pub adapter_identity: alloc::string::String,
    pub adapter_slot: u32,
    pub owner: u64,
    pub input: alloc::vec::Vec<crate::types::Ty>,
    pub output: alloc::vec::Vec<crate::types::Ty>,
    pub effects: crate::types::EffSet,
}

/// Pending code-export request; its checked effect is not export authority.
pub struct BoundCodeExportRegistration {
    pub adapter_identity: alloc::string::String,
    pub adapter_slot: u32,
    pub owner: u64,
    pub input: alloc::vec::Vec<crate::types::Ty>,
    pub output: alloc::vec::Vec<crate::types::Ty>,
    pub effects: crate::types::EffSet,
}

/// Pending send request; its checked effect is not send authority.
pub struct BoundSendRegistration {
    pub adapter_identity: alloc::string::String,
    pub adapter_slot: u32,
    pub owner: u64,
    pub input: alloc::vec::Vec<crate::types::Ty>,
    pub output: alloc::vec::Vec<crate::types::Ty>,
    pub effects: crate::types::EffSet,
}

/// One precomputed, source-independent test-host dispatch decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockDecision {
    Value(i64),
    Denied,
    ScriptExhausted,
    UnexpectedOperation,
}

/// A finite, explicitly typed test clock, not an ambient clock or authority.
pub struct ClockPlan<'a> {
    pub operation: &'a str,
    pub adapter_identity: &'a str,
    pub input: &'a [crate::types::Ty],
    pub output: &'a [crate::types::Ty],
    pub effects: &'a [crate::types::EffId],
    pub allowed: bool,
    pub script: &'a [i64],
}

impl ClockPlan<'_> {
    /// Check the exact binding anew and choose one dispatch result. The shell
    /// receives a finite table of these decisions; it never consults a real clock.
    #[expect(
        tigerstyle::usize_in_public_api,
        reason = "the index is a bounded table position already typed usize by the caller; changing it would break the kernel API"
    )]
    pub fn decide(&self, requested: &str, index: usize) -> Result<ClockDecision, NominalError> {
        let is_test_clock = self.operation == "test.clock";
        let has_adapter = !self.adapter_identity.is_empty();
        let has_input = !self.input.is_empty();
        let is_i64_returning = self.output == [crate::types::Ty::I64];
        let has_test_clock_effect = self.effects == [TEST_CLOCK];
        let is_short_script = self.script.len() <= 16;
        let is_clock_shape = is_test_clock && has_adapter && !has_input;
        let is_clock_signature = is_i64_returning && has_test_clock_effect && is_short_script;
        if !(is_clock_shape && is_clock_signature) {
            return Err(NominalError::InvalidRepresentation);
        }
        if requested != self.operation {
            return Ok(ClockDecision::UnexpectedOperation);
        }
        if !self.allowed {
            return Ok(ClockDecision::Denied);
        }
        Ok(match self.script.get(index) {
            Some(value) => ClockDecision::Value(*value),
            None => ClockDecision::ScriptExhausted,
        })
    }
}

/// The checking environment. `Default` is an empty staging replacement;
/// checking uses the validated environment returned by `environment`.
#[derive(Clone, Debug, Default)]
pub struct Env {
    /// Schemes indexed by `Definition`.
    pub defs: alloc::vec::Vec<crate::words::Scheme>,
    /// Behavior kinds indexed by `Definition`.
    pub kinds: alloc::vec::Vec<Behavior>,
    /// Per-definition dependency lists from external environment data; the
    /// bootstrap table declares none, and a missing entry means none.
    pub deps: alloc::vec::Vec<alloc::vec::Vec<Definition>>,
    /// User-declared schemas from external environment data.
    pub schemas: alloc::vec::Vec<SchemaDecl>,
    /// Canonical nonrecursive descriptors, addressed by resolved identities.
    pub nominals: alloc::vec::Vec<NominalDecl>,
    /// Source-declared two-parameter variants; never expanded per instance.
    pub generic_variants: alloc::vec::Vec<GenericVariantDecl>,
    /// Kinds actually admitted by the host; the fixture is explicit.
    pub resource_kinds: alloc::vec::Vec<crate::types::ResourceKind>,
    /// Host-registered, versioned opaque resource identities without guest
    /// constructors. Each has a distinct kind in `resource_kinds`.
    pub live_resource_nominals: alloc::vec::Vec<crate::types::NominalTypeId>,
    /// Independently selected live-slot profile; absent in Core and draft.
    pub live_slots: bool,
    /// Host-selected permission to typecheck the bootstrap test emitter in
    /// live-slot source. This flag is not a runtime effect authorization.
    pub live_test_hosts: bool,
    /// Caller selected by the host for this independent acceptance run.
    pub caller_module: Option<u64>,
    /// Explicit opt-in profile selected outside the untrusted candidate.
    /// Module registration also enables it; it is required for declarations
    /// with no nominal types or bound operations.
    pub declared_modules: bool,
    /// Selected standalone Text cursor profile, not part of Core or declared modules.
    pub text_cursor: bool,
    /// Selected calculator-only guest primitives and declared numeric modules.
    pub calculator: bool,
    /// Owner of each definition, supplied independently of a candidate.
    pub definition_owners: alloc::vec::Vec<Option<u64>>,
    /// Host-confirmed immutable dispatch identities for required operations.
    pub bound_adapters: alloc::vec::Vec<BoundAdapter>,
    /// Effect identities this environment provides; `TEST_EMIT` is first.
    pub effects: alloc::vec::Vec<crate::types::EffId>,
}

impl Env {
    /// Select live-slot checking outside the untrusted candidate.
    pub fn enable_live_slots(mut self) -> Self {
        if !self.live_slots {
            self.live_slots = true;
            self.declared_modules = true;
            self.effects.push(LIVE_DISPATCH);
        }
        self
    }

    /// The host must select this independently of source text and authorize
    /// each emitted effect separately at the execution boundary.
    pub fn enable_live_test_hosts(mut self) -> Self {
        self.live_test_hosts = true;
        self
    }

    pub(crate) fn ambient_test_emit_visible(&self) -> bool {
        !self.text_cursor
            && !self.calculator
            && !self.has_checked_i64()
            && self.caller_module.is_none()
            && self.bound_adapters.is_empty()
            && ((!self.declared_modules
                && self.nominals.is_empty()
                && self.generic_variants.is_empty())
                || (self.live_slots && self.live_test_hosts))
    }

    /// The checked-I64 profile is identified by its host-selected builtin
    /// table, keeping legacy environment debug identities unchanged.
    pub fn has_checked_i64(&self) -> bool {
        self.kinds.contains(&Behavior::CheckedI64Add)
    }

    /// Register a host-owned versioned nominal backed by a unique opaque
    /// resource kind. No guest-accessible `new` or `into` operation is made.
    pub fn register_live_resource(self, decl: NominalDecl) -> Result<Self, NominalError> {
        live::register_resource(self, decl)
    }

    /// Validate a live reference's full ordered interface, including nested
    /// borrowed inputs but never borrowed outputs or captured values.
    pub fn valid_live_ref(&self, ty: &crate::types::Ty) -> bool {
        live::valid_ref(self, ty)
    }

    /// Find one resolved nominal descriptor, without treating an alias as an identity.
    pub fn nominal(&self, id: crate::types::NominalTypeId) -> Option<&NominalDecl> {
        let mut index = 0;
        while index < self.nominals.len() && self.nominals[index].id != id {
            index += 1;
        }
        self.nominals.get(index)
    }

    /// Find a generic family by its resolved declaration identity.
    pub fn generic_variant(&self, id: crate::types::NominalTypeId) -> Option<&GenericVariantDecl> {
        let mut index = 0;
        while index < self.generic_variants.len() && self.generic_variants[index].id != id {
            index += 1;
        }
        self.generic_variants.get(index)
    }

    /// Construct a checked canonical instance, retaining argument order and
    /// the exact descriptor derived from its family declaration.
    pub fn generic_instance(
        &self,
        id: crate::types::NominalTypeId,
        args: [crate::types::Ty; 2],
    ) -> Option<crate::types::Ty> {
        let decl = self.generic_variant(id)?;
        let is_visible = decl.exported || self.caller_module == Some(id.module);
        let has_valid_argument_types =
            args[0].valid_generic_argument() && args[1].valid_generic_argument();
        let has_admitted_payloads =
            self.valid_nominal_payload(&args[0], 512) && self.valid_nominal_payload(&args[1], 512);
        let has_admitted_types = self.valid_type(&args[0], 512) && self.valid_type(&args[1], 512);
        let has_admitted_arguments = has_valid_argument_types && has_admitted_payloads;
        if !is_visible || !has_admitted_arguments || !has_admitted_types {
            return None;
        }
        let shape = nominal::generic_shape(decl, &args)?;
        Some(crate::types::Ty::GenericNominal(
            id,
            alloc::boxed::Box::new(args),
            alloc::boxed::Box::new(shape),
        ))
    }

    /// Borrowed exact-instance check for consumers of an independently
    /// checked family; no per-instance declaration is registered.
    pub fn generic_instance_matches(
        &self,
        ty: &crate::types::Ty,
        id: crate::types::NominalTypeId,
    ) -> bool {
        let crate::types::Ty::GenericNominal(actual_id, _, _) = ty else {
            return false;
        };
        if *actual_id != id {
            return false;
        }
        self.generic_variant(id).is_some() && self.valid_type(ty, 512)
    }

    /// Check a borrowed instance without a separately supplied family id.
    pub fn valid_generic_instance(&self, ty: &crate::types::Ty) -> bool {
        match ty {
            crate::types::Ty::GenericNominal(id, _, _) => self.generic_instance_matches(ty, *id),
            _ => false,
        }
    }

    /// Register a validated nonrecursive type and its typed operations.
    /// Ownership transfers into the updated environment on success.
    pub fn declare_nominal(self, decl: NominalDecl) -> Result<(Self, NominalOps), NominalError> {
        let is_resource_payload = match &decl.shape {
            crate::types::NominalShape::Opaque(inner) => !inner.is_data(),
            crate::types::NominalShape::Variant(left, right) => !left.is_data() || !right.is_data(),
        };
        if self.live_slots && is_resource_payload {
            return Err(NominalError::InvalidRepresentation);
        }
        nominal::register(self, decl)
    }

    /// Register one validated generic variant and its three rank-1
    /// operations, without generating definitions per concrete instance.
    pub fn declare_generic_variant(
        self,
        decl: GenericVariantDecl,
    ) -> Result<(Self, NominalOps), NominalError> {
        nominal::register_generic_variant(self, decl)
    }

    /// Register an immutable required operation, exactly `Text -- ! test.emit`.
    /// Its operation name and requirement are resolved by the module frontend.
    /// Ownership transfers into the updated environment on success.
    pub fn declare_bound_emit(
        self,
        registration: BoundEmitRegistration,
    ) -> Result<(Self, Definition), NominalError> {
        nominal::register_bound_emit(self, registration)
    }

    /// Register only the exact `[] -- I64 ! test.clock` scripted host contract.
    /// The separately supplied host policy and finite script grant no authority here.
    pub fn declare_bound_clock(
        self,
        registration: BoundClockRegistration,
    ) -> Result<(Self, Definition), NominalError> {
        nominal::register_bound_clock(self, registration)
    }

    /// Register only `Text Text -- ! test.code-export`. This checks a request,
    /// never grants permission to serialize or export either Text.
    pub fn declare_bound_code_export(
        self,
        registration: BoundCodeExportRegistration,
    ) -> Result<(Self, Definition), NominalError> {
        nominal::register_bound_code_export(self, registration)
    }

    /// Register only `Text -- ! test.send`. This checks a request; the host
    /// alone authorizes, admits and observes each attempt.
    pub fn declare_bound_send(
        self,
        registration: BoundSendRegistration,
    ) -> Result<(Self, Definition), NominalError> {
        nominal::register_bound_send(self, registration)
    }

    /// The scheme of a definition.
    pub fn scheme(&self, def: Definition) -> Option<&crate::words::Scheme> {
        match index(def) {
            Some(index) => self.defs.get(index),
            None => None,
        }
    }

    /// The behavior of a definition.
    pub fn kind(&self, def: Definition) -> Option<Behavior> {
        match index(def) {
            Some(index) => self.kinds.get(index).copied(),
            None => None,
        }
    }

    /// Whether the environment provides this effect identity.
    pub fn knows_effect(&self, id: crate::types::EffId) -> bool {
        let mut index = 0;
        while index < self.effects.len() {
            if self.effects[index] == id {
                return true;
            }
            index += 1;
        }
        false
    }

    /// Number of definitions.
    pub fn len(&self) -> Option<u64> {
        u64::try_from(self.defs.len()).ok()
    }

    /// Number of effect identities this environment provides.
    ///
    /// This is the bound one instantiation's effect set must fit inside; it is
    /// deliberately independent of the definition count, which says nothing
    /// about how many effect identities exist.
    pub fn effect_count(&self) -> Option<u64> {
        u64::try_from(self.effects.len()).ok()
    }

    /// Whether the environment has no definitions.
    pub fn is_empty(&self) -> bool {
        self.defs.len() == 0
    }
}

fn index(def: Definition) -> Option<usize> {
    usize::try_from(def.0).ok()
}

/// Build the bootstrap environment with the fixed v1 contract table.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; environment validates every bootstrap scheme before constructing its parallel definition/kind/dependency tables and returns the first Defect; assertions would change this fallible construction contract."
)]
pub fn environment() -> Result<Env, crate::shapes::Defect> {
    let table = bootstrap::data::table();
    let mut defs: alloc::vec::Vec<crate::words::Scheme> = alloc::vec::Vec::with_capacity(32);
    let mut kinds: alloc::vec::Vec<Behavior> = alloc::vec::Vec::with_capacity(32);
    let mut deps: alloc::vec::Vec<alloc::vec::Vec<Definition>> = alloc::vec::Vec::with_capacity(32);
    let mut index = 0;
    while index < table.len() {
        match table[index].1.validate() {
            Ok(()) => {
                kinds.push(table[index].0);
                index += 1;
            }
            Err(problem) => return Err(problem),
        }
    }
    let mut table_index = 0;
    while table_index < table.len() {
        defs.push(table[table_index].1.clone());
        #[expect(
            tigerstyle::allocation_in_loop,
            reason = "Owner: noble-maintainers; Vec::new allocates no storage and records the bootstrap definition's empty dependency list; reassess if this changes to a nonzero-capacity constructor."
        )]
        deps.push(alloc::vec::Vec::new());
        table_index += 1;
    }
    Ok(Env {
        defs,
        kinds,
        deps,
        schemas: alloc::vec::Vec::new(),
        nominals: alloc::vec::Vec::new(),
        generic_variants: alloc::vec::Vec::new(),
        resource_kinds: alloc::vec![FIXTURE_RESOURCE],
        live_resource_nominals: alloc::vec::Vec::new(),
        live_slots: false,
        live_test_hosts: false,
        caller_module: None,
        declared_modules: false,
        text_cursor: false,
        calculator: false,
        definition_owners: alloc::vec![None; table.len()],
        bound_adapters: alloc::vec::Vec::new(),
        effects: alloc::vec![TEST_EMIT],
    })
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
/// Extend the fixed bootstrap table with private reserved rows and one pure
/// Text byte-at operation, without making it available in other profiles.
pub fn text_cursor_environment() -> Result<Env, crate::shapes::Defect> {
    let mut env = environment()?;
    let reserved = crate::words::Scheme {
        var_kinds: alloc::vec![],
        stack_in: alloc::vec![],
        stack_out: alloc::vec![],
        effects: alloc::vec![],
    };
    reserved.validate()?;
    let reserved_rows = 3;
    env.defs
        .extend(core::iter::repeat_n(reserved.clone(), reserved_rows));
    env.kinds
        .extend(core::iter::repeat_n(Behavior::Reserved, reserved_rows));
    // Aeneas 1238879f rejects generic Vec::new function items with bound regions (SymbolicToPureTypes.ml:411).
    env.deps
        .extend(core::iter::repeat_with(empty_dependency_list).take(reserved_rows));
    env.definition_owners
        .extend(core::iter::repeat_n(None, reserved_rows));
    let tail = crate::shapes::Pattern::StackVar(crate::words::Variable(0));
    let scheme = crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![
            tail.clone(),
            crate::shapes::Pattern::Text,
            crate::shapes::Pattern::I64
        ],
        stack_out: alloc::vec![
            tail,
            crate::shapes::Pattern::Text,
            crate::shapes::Pattern::I64
        ],
        effects: alloc::vec![],
    };
    scheme.validate()?;
    env.defs.push(scheme);
    env.kinds.push(Behavior::TextByte);
    env.deps.push(alloc::vec::Vec::new());
    env.definition_owners.push(None);
    env.text_cursor = true;
    Ok(env)
}

/// The opt-in checked-I64 profile preserves the bootstrap prefix and appends
/// seven pure operations at fixed identities 23..=29.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; each appended builtin scheme is independently validated before its parallel environment tables are extended"
)]
pub fn checked_i64_environment() -> Result<Env, crate::shapes::Defect> {
    let mut env = environment()?;
    let tail = crate::shapes::Pattern::StackVar(crate::words::Variable(0));
    let result = crate::shapes::Pattern::Sum(
        alloc::boxed::Box::new(crate::shapes::Pattern::I64),
        alloc::boxed::Box::new(crate::shapes::Pattern::I64),
    );
    let div_rem_result = crate::shapes::Pattern::Sum(
        alloc::boxed::Box::new(crate::shapes::Pattern::Pair(
            alloc::boxed::Box::new(crate::shapes::Pattern::I64),
            alloc::boxed::Box::new(crate::shapes::Pattern::I64),
        )),
        alloc::boxed::Box::new(crate::shapes::Pattern::I64),
    );
    let binary = |output: crate::shapes::Pattern| crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![
            tail.clone(),
            crate::shapes::Pattern::I64,
            crate::shapes::Pattern::I64
        ],
        stack_out: alloc::vec![tail.clone(), output],
        effects: alloc::vec![],
    };
    let unary = |output: crate::shapes::Pattern| crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![tail.clone(), crate::shapes::Pattern::I64],
        stack_out: alloc::vec![tail.clone(), output],
        effects: alloc::vec![],
    };
    let operations = [
        (Behavior::CheckedI64Add, binary(result.clone())),
        (Behavior::CheckedI64Sub, binary(result.clone())),
        (Behavior::CheckedI64Mul, binary(result.clone())),
        (Behavior::CheckedI64Neg, unary(result.clone())),
        (Behavior::CheckedI64Abs, unary(result)),
        (
            Behavior::CheckedI64Less,
            binary(crate::shapes::Pattern::Bool),
        ),
        (Behavior::CheckedI64DivRem, binary(div_rem_result)),
    ];
    env.deps
        .extend(alloc::vec![alloc::vec::Vec::new(); operations.len()]);
    for (behavior, scheme) in operations {
        scheme.validate()?;
        env.defs.push(scheme);
        env.kinds.push(behavior);
        env.definition_owners.push(None);
    }
    Ok(env)
}

#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-kernel; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
/// The calculator has its own opt-in prefix. Neither standalone cursor nor
/// ordinary declared-module admission inherits these extra pure operations.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
pub fn calculator_environment() -> Result<Env, crate::shapes::Defect> {
    let mut env = text_cursor_environment()?;
    env.text_cursor = false;
    env.calculator = true;
    env.declared_modules = true;
    let row = crate::shapes::Pattern::StackVar(crate::words::Variable(0));
    let state = crate::shapes::Pattern::Var(crate::words::Variable(1));
    let loop_step = crate::words::Scheme {
        var_kinds: alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value
        ],
        stack_in: alloc::vec![
            row.clone(),
            state.clone(),
            crate::shapes::Pattern::program(
                alloc::vec![row.clone(), state.clone()],
                alloc::vec![row.clone(), state.clone(), crate::shapes::Pattern::Bool],
                alloc::vec![],
            ),
        ],
        stack_out: alloc::vec![row.clone(), state],
        effects: alloc::vec![],
    };
    let compare = crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![
            row.clone(),
            crate::shapes::Pattern::I64,
            crate::shapes::Pattern::I64
        ],
        stack_out: alloc::vec![row.clone(), crate::shapes::Pattern::I64],
        effects: alloc::vec![],
    };
    let divide = crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![
            row.clone(),
            crate::shapes::Pattern::I64,
            crate::shapes::Pattern::I64
        ],
        stack_out: alloc::vec![
            row.clone(),
            crate::shapes::Pattern::I64,
            crate::shapes::Pattern::I64,
            crate::shapes::Pattern::Bool
        ],
        effects: alloc::vec![],
    };
    let text = crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![
            row.clone(),
            crate::shapes::Pattern::List(alloc::boxed::Box::new(crate::shapes::Pattern::I64))
        ],
        stack_out: alloc::vec![
            row.clone(),
            crate::shapes::Pattern::Text,
            crate::shapes::Pattern::Bool
        ],
        effects: alloc::vec![],
    };
    let text_length = crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![row.clone(), crate::shapes::Pattern::Text],
        stack_out: alloc::vec![
            row,
            crate::shapes::Pattern::Text,
            crate::shapes::Pattern::I64
        ],
        effects: alloc::vec![],
    };
    let table = [
        (Behavior::CalculatorIterate, loop_step),
        (Behavior::CalculatorCompare, compare),
        (Behavior::CalculatorDivRem, divide),
        (Behavior::CalculatorTextFromAscii, text),
        (Behavior::CalculatorTextLength, text_length),
    ];
    let rows = table.len();
    for (behavior, scheme) in table {
        scheme.validate()?;
        env.defs.push(scheme);
        env.kinds.push(behavior);
    }
    // Aeneas 1238879f rejects generic Vec::new function items with bound regions (SymbolicToPureTypes.ml:411).
    env.deps
        .extend(core::iter::repeat_with(empty_dependency_list).take(rows));
    env.definition_owners
        .extend(core::iter::repeat_n(None, rows));
    Ok(env)
}

/// Pure declared text retains the immutable cursor at 26 and adds signed
/// comparison at 27. Core, standalone cursor, ordinary modules and the
/// separately selected calculator profile gain neither.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the environment validates the added signed-comparison scheme before extending its parallel definition/kind/dependency/owner tables and returns the first Defect; assertions would change this fallible construction contract."
)]
pub fn declared_text_environment() -> Result<Env, crate::shapes::Defect> {
    let mut env = text_cursor_environment()?;
    let tail = crate::shapes::Pattern::StackVar(crate::words::Variable(0));
    let less = crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![
            tail.clone(),
            crate::shapes::Pattern::I64,
            crate::shapes::Pattern::I64
        ],
        stack_out: alloc::vec![tail, crate::shapes::Pattern::Bool],
        effects: alloc::vec![],
    };
    less.validate()?;
    env.defs.push(less);
    env.kinds.push(Behavior::I64Less);
    env.deps.push(alloc::vec::Vec::new());
    env.definition_owners.push(None);
    env.declared_modules = true;
    Ok(env)
}

#[cfg(test)]
mod checked_i64_tests {
    #[test]
    fn concrete_aeneas_factory_keeps_each_reserved_dependency_list_empty() {
        let base = super::environment()
            .expect("bootstrap environment")
            .defs
            .len();
        let cursor = super::text_cursor_environment().expect("cursor environment");
        assert_eq!(cursor.deps.len(), cursor.defs.len());
        assert_eq!(
            cursor.deps.len(),
            base.checked_add(4).expect("cursor row bound")
        );
        assert!(
            cursor.deps[base..base.checked_add(3).expect("cursor row bound")]
                .iter()
                .all(alloc::vec::Vec::is_empty)
        );

        let mut calculator = super::calculator_environment().expect("calculator environment");
        assert_eq!(calculator.deps.len(), calculator.defs.len());
        assert_eq!(
            calculator.deps.len(),
            cursor
                .deps
                .len()
                .checked_add(5)
                .expect("calculator row bound")
        );
        assert!(calculator.deps[base..]
            .iter()
            .all(alloc::vec::Vec::is_empty));
        calculator.deps[base].push(super::Definition(0));
        assert!(
            calculator.deps[base.checked_add(1).expect("calculator row bound")..]
                .iter()
                .all(alloc::vec::Vec::is_empty)
        );
    }

    #[test]
    fn profile_appends_the_fixed_pure_operation_table() {
        let ordinary = super::environment().unwrap_or_else(|_| panic!("ordinary environment"));
        let checked =
            super::checked_i64_environment().unwrap_or_else(|_| panic!("checked-I64 environment"));
        assert!(!ordinary.has_checked_i64());
        assert!(checked.has_checked_i64());
        assert!(!checked.calculator);
        assert!(!checked.text_cursor);
        assert!(!checked.declared_modules);
        assert_eq!(checked.defs.len(), 30);
        assert_eq!(checked.kinds.len(), 30);
        assert_eq!(checked.deps.len(), 30);
        assert_eq!(checked.definition_owners.len(), 30);
        assert_eq!(
            checked.kinds[23..],
            [
                super::Behavior::CheckedI64Add,
                super::Behavior::CheckedI64Sub,
                super::Behavior::CheckedI64Mul,
                super::Behavior::CheckedI64Neg,
                super::Behavior::CheckedI64Abs,
                super::Behavior::CheckedI64Less,
                super::Behavior::CheckedI64DivRem,
            ]
        );
        assert!(checked.deps[23..].iter().all(alloc::vec::Vec::is_empty));
        assert!(checked.definition_owners[23..].iter().all(Option::is_none));
        assert_eq!(
            checked
                .scheme(super::Definition(23))
                .unwrap()
                .stack_in
                .len(),
            3
        );
        assert_eq!(
            checked
                .scheme(super::Definition(26))
                .unwrap()
                .stack_in
                .len(),
            2
        );
    }

    #[test]
    fn div_rem_result_has_pair_success_or_error_alternative() {
        let checked =
            super::checked_i64_environment().unwrap_or_else(|_| panic!("checked-I64 environment"));
        let scheme = checked.scheme(super::Definition(29)).unwrap();
        assert!(matches!(
            scheme.stack_out.as_slice(),
            [
                crate::shapes::Pattern::StackVar(crate::words::Variable(0)),
                crate::shapes::Pattern::Sum(success, error)
            ] if matches!(success.as_ref(), crate::shapes::Pattern::Pair(q, r)
                if matches!(q.as_ref(), crate::shapes::Pattern::I64)
                    && matches!(r.as_ref(), crate::shapes::Pattern::I64))
                && matches!(error.as_ref(), crate::shapes::Pattern::I64)
        ));
    }
}
