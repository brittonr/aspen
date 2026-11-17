//! Independently checked module definitions, bootstrap rows, and bound adapters.

impl super::super::Env {
    pub(crate) fn valid_generic_declarations(&self) -> bool {
        let mut index = 0;
        let mut is_valid = true;
        while index < self.generic_variants.len() && is_valid {
            let decl = &self.generic_variants[index];
            is_valid =
                matches!(decl.payload_params, [0, 1] | [1, 0]) && self.nominal(decl.id).is_none();
            let mut prior = 0;
            while prior < index && is_valid {
                is_valid = self.generic_variants[prior].id != decl.id;
                prior += 1;
            }
            index += 1;
        }
        is_valid
    }

    pub(crate) fn validate_contracts(&self) -> bool {
        if self.kinds.len() != self.defs.len() || self.bound_adapters.len() > self.defs.len() {
            return false;
        }
        if self.live_test_hosts && (!self.live_slots || !self.declared_modules || self.text_cursor)
        {
            return false;
        }
        if (!self.nominals.is_empty()
            || !self.generic_variants.is_empty()
            || !self.bound_adapters.is_empty())
            && !self.declared_modules
        {
            return false;
        }
        bootstrap_matches(self)
            && (!self.text_cursor || !self.declared_modules || declared_text_prefix_matches(self))
            && (!self.calculator || calculator_prefix_matches(self))
            && super::super::live::catalog_valid(self)
            && (!self.declared_modules || fixed_definitions_match(self))
            && counts_match(self)
            && generic_counts_match(self)
            && definitions_match(self)
            && bound_adapters_unique(self)
    }

    fn matching_bound_rows(&self, def: super::super::Definition, slot: u32) -> u8 {
        let mut matches = 0u8;
        let mut row_index = 0;
        while row_index < self.bound_adapters.len() {
            if bound_row_matches(self, row_index, def, slot) {
                matches = matches.saturating_add(1);
            }
            row_index += 1;
        }
        matches
    }
}

/// The combined declared-text profile is exactly the fixed cursor prefix plus
/// signed comparison at 27; no row from 23 onward may be relabeled or owned.
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::compound_condition,
    reason = "Owner: noble-kernel; the prefix check compares side-effect-free table lengths and rows against the fixed declared-text environment and reports a mismatch as false; callers turn that into a typed rejection."
)]
fn declared_text_prefix_matches(env: &super::super::Env) -> bool {
    let Ok(fixed) = super::super::declared_text_environment() else {
        return false;
    };
    if env.defs.len() < fixed.defs.len()
        || env.kinds.len() < fixed.kinds.len()
        || env.deps.len() < fixed.defs.len()
        || env.definition_owners.len() < fixed.defs.len()
    {
        return false;
    }
    let mut index = 23;
    let mut is_matching = true;
    while index < fixed.defs.len() && is_matching {
        is_matching = env.kinds[index] == fixed.kinds[index]
            && super::schemes::same_scheme(&env.defs[index], &fixed.defs[index])
            && env.deps[index].is_empty()
            && env.definition_owners[index].is_none();
        index += 1;
    }
    is_matching
}

/// The calculator rows 27 through 31 are exactly the fixed calculator prefix.
///
/// Visibility grants those five rows by index alone, so a replaced scheme or
/// relabeled kind would be admitted under the visible row's name while
/// deriving a different interface. Every row the profile can invoke is
/// therefore compared with its canonical scheme, as the bootstrap rows are.
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::compound_condition,
    reason = "Owner: noble-kernel; the prefix check compares side-effect-free table lengths and rows against the fixed calculator environment and reports a mismatch as false; callers turn that into a typed rejection."
)]
fn calculator_prefix_matches(env: &super::super::Env) -> bool {
    let Ok(fixed) = super::super::calculator_environment() else {
        return false;
    };
    if env.defs.len() < fixed.defs.len()
        || env.kinds.len() < fixed.kinds.len()
        || env.deps.len() < fixed.defs.len()
        || env.definition_owners.len() < fixed.defs.len()
    {
        return false;
    }
    let mut index = CALCULATOR_FIRST_ROW;
    let mut is_matching = true;
    while index < fixed.defs.len() && is_matching {
        is_matching = env.kinds[index] == fixed.kinds[index]
            && super::schemes::same_scheme(&env.defs[index], &fixed.defs[index])
            && env.deps[index].is_empty()
            && env.definition_owners[index].is_none();
        index += 1;
    }
    is_matching
}

/// First row the calculator profile adds and visibility grants by index.
const CALCULATOR_FIRST_ROW: usize = 27;

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
fn generic_counts_match(env: &super::super::Env) -> bool {
    if !env.valid_generic_declarations() {
        return false;
    }
    let mut index = 0;
    while index < env.generic_variants.len() {
        let decl = &env.generic_variants[index];
        let mut count = [0u8; 3];
        let mut kind_index = 0;
        while kind_index < env.kinds.len() {
            match env.kinds[kind_index] {
                super::super::Behavior::GenericLeft(id) if id == decl.id => {
                    count[0] = count[0].saturating_add(1);
                }
                super::super::Behavior::GenericRight(id) if id == decl.id => {
                    count[1] = count[1].saturating_add(1);
                }
                super::super::Behavior::GenericMatch(id) if id == decl.id => {
                    count[2] = count[2].saturating_add(1);
                }
                _ => {}
            }
            kind_index += 1;
        }
        if count != [1, 1, 1] {
            return false;
        }
        index += 1;
    }
    true
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject indexing bound_adapters Vec (E0277/E0658) and its runtime BoundAdapter equality checks; reassess when those reads and comparisons become const-capable."
)]
fn bound_row_matches(
    env: &super::super::Env,
    index: usize,
    def: super::super::Definition,
    slot: u32,
) -> bool {
    let row = &env.bound_adapters[index];
    if row.definition != def {
        return false;
    }
    if row.adapter_slot != slot {
        return false;
    }
    bound_row_contract(row)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject input Vec indexing (E0277/E0658) and runtime EffSet::contains used to validate this adapter; reassess when these reads become const-capable."
)]
fn bound_row_contract(row: &super::super::BoundAdapter) -> bool {
    if row.adapter_identity.is_empty() {
        return false;
    }
    (row.input.as_slice() == [crate::types::Ty::Text]
        && row.output.is_empty()
        && row.effects.as_slice() == [super::super::TEST_EMIT])
        || (row.input.is_empty()
            && row.output.as_slice() == [crate::types::Ty::I64]
            && row.effects.as_slice() == [super::super::TEST_CLOCK])
        || (row.adapter_identity == super::super::TEST_CODE_EXPORT_ADAPTER_IDENTITY
            && row.input.as_slice() == [crate::types::Ty::Text, crate::types::Ty::Text]
            && row.output.is_empty()
            && row.effects.as_slice() == [super::super::TEST_CODE_EXPORT])
        || (row.adapter_identity == super::super::TEST_SEND_ADAPTER_IDENTITY
            && row.input.as_slice() == [crate::types::Ty::Text]
            && row.output.is_empty()
            && row.effects.as_slice() == [super::super::TEST_SEND])
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject indexing the resource_kinds and effects Vecs (E0277/E0658) to validate fixture identities; reassess when indexed Vec reads become const-capable."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
fn bootstrap_matches(env: &super::super::Env) -> bool {
    if !env.declared_modules {
        return true;
    }
    let has_clock = env
        .kinds
        .iter()
        .any(|kind| matches!(kind, super::super::Behavior::BoundClock(_)));
    let has_code_export = env
        .kinds
        .iter()
        .any(|kind| matches!(kind, super::super::Behavior::BoundCodeExport(_)));
    let has_send = env
        .kinds
        .iter()
        .any(|kind| matches!(kind, super::super::Behavior::BoundSend(_)));
    env.resource_kinds.len() == 1usize.saturating_add(env.live_resource_nominals.len())
        && env.resource_kinds[0].0 == super::super::FIXTURE_RESOURCE.0
        && env.effects.len()
            == 1usize
                .saturating_add(usize::from(has_clock))
                .saturating_add(usize::from(has_code_export))
                .saturating_add(usize::from(has_send))
                .saturating_add(usize::from(env.live_slots))
        && env.effects[0].0 == super::super::TEST_EMIT.0
        && env.effects.contains(&super::super::TEST_CLOCK) == has_clock
        && env.effects.contains(&super::super::TEST_CODE_EXPORT) == has_code_export
        && env.effects.contains(&super::super::TEST_SEND) == has_send
        && env.effects.contains(&super::super::LIVE_DISPATCH) == env.live_slots
        && env.definition_owners.len() == env.defs.len()
        && env.deps.len() == env.defs.len()
}

fn fixed_definitions_match(env: &super::super::Env) -> bool {
    let fixed = super::super::bootstrap::data::table();
    if env.defs.len() < fixed.len() {
        return false;
    }
    let mut index = 0;
    let mut is_matching = true;
    while index < fixed.len() && is_matching {
        let (behavior, scheme) = &fixed[index];
        is_matching =
            env.kinds[index] == *behavior && super::schemes::same_scheme(&env.defs[index], scheme);
        index += 1;
    }
    is_matching
}

fn counts_match(env: &super::super::Env) -> bool {
    let mut index = 0;
    let mut is_matching = true;
    while index < env.nominals.len() && is_matching {
        is_matching = declaration_counts_match(env, &env.nominals[index]);
        index += 1;
    }
    is_matching
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
fn declaration_counts_match(env: &super::super::Env, decl: &super::super::NominalDecl) -> bool {
    let mut actual = [0u8; 5];
    let mut index = 0;
    while index < env.kinds.len() {
        if let Some(which) = operation_slot(env.kinds[index], decl.id) {
            actual[which] = actual[which].saturating_add(1);
        }
        index += 1;
    }
    let expected = if env.live_resource_nominals.contains(&decl.id) {
        [0, 0, 0, 0, 0]
    } else if matches!(decl.shape, crate::types::NominalShape::Opaque(_)) {
        [1, 1, 0, 0, 0]
    } else if matches!(decl.shape, crate::types::NominalShape::Variant(_, _)) {
        [0, 0, 1, 1, 1]
    } else {
        return false;
    };
    actual == expected
}

const fn operation_slot(
    kind: super::super::Behavior,
    id: crate::types::NominalTypeId,
) -> Option<usize> {
    let Some((found, slot)) = operation_identity_slot(kind) else {
        return None;
    };
    if found.module == id.module && found.ordinal == id.ordinal {
        Some(slot)
    } else {
        None
    }
}

const fn operation_identity_slot(
    kind: super::super::Behavior,
) -> Option<(crate::types::NominalTypeId, usize)> {
    if let super::super::Behavior::NominalNew(id) = kind {
        return Some((id, 0));
    }
    if let super::super::Behavior::NominalInto(id) = kind {
        return Some((id, 1));
    }
    if let super::super::Behavior::NominalLeft(id) = kind {
        return Some((id, 2));
    }
    if let super::super::Behavior::NominalRight(id) = kind {
        return Some((id, 3));
    }
    if let super::super::Behavior::NominalMatch(id) = kind {
        return Some((id, 4));
    }
    None
}

fn definitions_match(env: &super::super::Env) -> bool {
    let mut index = 0;
    let mut is_matching = true;
    while index < env.kinds.len() && is_matching {
        let kind = env.kinds[index];
        let is_matched = kind_matches_definition(env, index, kind);
        is_matching = is_matched;
        index += 1;
    }
    is_matching
}

const fn generic_operation_id(kind: super::super::Behavior) -> Option<crate::types::NominalTypeId> {
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-kernel; the Behavior enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
    )]
    match kind {
        super::super::Behavior::GenericLeft(id)
        | super::super::Behavior::GenericRight(id)
        | super::super::Behavior::GenericMatch(id) => Some(id),
        super::super::Behavior::Dup => None,
        super::super::Behavior::Drop => None,
        super::super::Behavior::Swap => None,
        super::super::Behavior::Dip => None,
        super::super::Behavior::Arith => None,
        super::super::Behavior::Equals => None,
        super::super::Behavior::Quote => None,
        super::super::Behavior::Compose => None,
        super::super::Behavior::Run => None,
        super::super::Behavior::Reflect => None,
        super::super::Behavior::Unit => None,
        super::super::Behavior::Pair => None,
        super::super::Behavior::Unpair => None,
        super::super::Behavior::Inl => None,
        super::super::Behavior::Inr => None,
        super::super::Behavior::Case => None,
        super::super::Behavior::If => None,
        super::super::Behavior::Nil => None,
        super::super::Behavior::Cons => None,
        super::super::Behavior::ListCase => None,
        super::super::Behavior::TestEmit => None,
        super::super::Behavior::TextByte => None,
        super::super::Behavior::I64Less => None,
        super::super::Behavior::CheckedI64Add => None,
        super::super::Behavior::CheckedI64Sub => None,
        super::super::Behavior::CheckedI64Mul => None,
        super::super::Behavior::CheckedI64Neg => None,
        super::super::Behavior::CheckedI64Abs => None,
        super::super::Behavior::CheckedI64Less => None,
        super::super::Behavior::CheckedI64DivRem => None,
        super::super::Behavior::CalculatorIterate => None,
        super::super::Behavior::CalculatorCompare => None,
        super::super::Behavior::CalculatorDivRem => None,
        super::super::Behavior::CalculatorTextFromAscii => None,
        super::super::Behavior::CalculatorTextLength => None,
        super::super::Behavior::Reserved => None,
        super::super::Behavior::BoundEmit(_) => None,
        super::super::Behavior::BoundClock(_) => None,
        super::super::Behavior::BoundCodeExport(_) => None,
        super::super::Behavior::BoundSend(_) => None,
        super::super::Behavior::NominalNew(_) => None,
        super::super::Behavior::NominalInto(_) => None,
        super::super::Behavior::NominalLeft(_) => None,
        super::super::Behavior::NominalRight(_) => None,
        super::super::Behavior::NominalMatch(_) => None,
        super::super::Behavior::Named => None,
    }
}

fn kind_matches_definition(
    env: &super::super::Env,
    index: usize,
    kind: super::super::Behavior,
) -> bool {
    if let Some((slot, effect)) = bound_operation(kind) {
        return bound_definition_matches(env, index, slot, effect);
    }
    if let Some(id) = generic_operation_id(kind) {
        return generic_definition_matches(env, index, kind, id);
    }
    if let Some((id, _)) = operation_identity_slot(kind) {
        return declared_definition_matches(env, index, kind, id);
    }
    true
}

/// The adapter slot and reserved effect of a required bound host operation.
const fn bound_operation(kind: super::super::Behavior) -> Option<(u32, crate::types::EffId)> {
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-kernel; the Behavior enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
    )]
    match kind {
        super::super::Behavior::BoundEmit(slot) => Some((slot, super::super::TEST_EMIT)),
        super::super::Behavior::BoundClock(slot) => Some((slot, super::super::TEST_CLOCK)),
        super::super::Behavior::BoundCodeExport(slot) => {
            Some((slot, super::super::TEST_CODE_EXPORT))
        }
        super::super::Behavior::BoundSend(slot) => Some((slot, super::super::TEST_SEND)),
        super::super::Behavior::Dup => None,
        super::super::Behavior::Drop => None,
        super::super::Behavior::Swap => None,
        super::super::Behavior::Dip => None,
        super::super::Behavior::Arith => None,
        super::super::Behavior::Equals => None,
        super::super::Behavior::Quote => None,
        super::super::Behavior::Compose => None,
        super::super::Behavior::Run => None,
        super::super::Behavior::Reflect => None,
        super::super::Behavior::Unit => None,
        super::super::Behavior::Pair => None,
        super::super::Behavior::Unpair => None,
        super::super::Behavior::Inl => None,
        super::super::Behavior::Inr => None,
        super::super::Behavior::Case => None,
        super::super::Behavior::If => None,
        super::super::Behavior::Nil => None,
        super::super::Behavior::Cons => None,
        super::super::Behavior::ListCase => None,
        super::super::Behavior::TestEmit => None,
        super::super::Behavior::TextByte => None,
        super::super::Behavior::I64Less => None,
        super::super::Behavior::CheckedI64Add => None,
        super::super::Behavior::CheckedI64Sub => None,
        super::super::Behavior::CheckedI64Mul => None,
        super::super::Behavior::CheckedI64Neg => None,
        super::super::Behavior::CheckedI64Abs => None,
        super::super::Behavior::CheckedI64Less => None,
        super::super::Behavior::CheckedI64DivRem => None,
        super::super::Behavior::CalculatorIterate => None,
        super::super::Behavior::CalculatorCompare => None,
        super::super::Behavior::CalculatorDivRem => None,
        super::super::Behavior::CalculatorTextFromAscii => None,
        super::super::Behavior::CalculatorTextLength => None,
        super::super::Behavior::Reserved => None,
        super::super::Behavior::NominalNew(_) => None,
        super::super::Behavior::NominalInto(_) => None,
        super::super::Behavior::NominalLeft(_) => None,
        super::super::Behavior::NominalRight(_) => None,
        super::super::Behavior::NominalMatch(_) => None,
        super::super::Behavior::GenericLeft(_) => None,
        super::super::Behavior::GenericRight(_) => None,
        super::super::Behavior::GenericMatch(_) => None,
        super::super::Behavior::Named => None,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "reads env.defs and the scheme helpers, which are not const"
)]
fn generic_definition_matches(
    env: &super::super::Env,
    index: usize,
    kind: super::super::Behavior,
    id: crate::types::NominalTypeId,
) -> bool {
    if env.definition_owners.get(index) != Some(&Some(id.module)) {
        return false;
    }
    let Some(decl) = env.generic_variant(id) else {
        return false;
    };
    let Some(expected) = super::schemes::expected_generic_scheme(decl, kind) else {
        return false;
    };
    super::schemes::same_scheme(&env.defs[index], &expected)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject owner Vec::get/index (E0277/E0658), Env::nominal lookup, and allocated expected_scheme construction (E0015); reassess when those checks become const-capable."
)]
fn declared_definition_matches(
    env: &super::super::Env,
    index: usize,
    kind: super::super::Behavior,
    id: crate::types::NominalTypeId,
) -> bool {
    if env.definition_owners.get(index) != Some(&Some(id.module)) {
        return false;
    }
    let Some(decl) = env.nominal(id) else {
        return false;
    };
    let Some(scheme) = super::schemes::expected_scheme(env, decl, kind) else {
        return false;
    };
    super::schemes::same_scheme(&env.defs[index], &scheme)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "reads env.defs and the scheme helpers, which are not const"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
fn bound_definition_matches(
    env: &super::super::Env,
    index: usize,
    slot: u32,
    effect: crate::types::EffId,
) -> bool {
    let is_valid_scheme = if effect == super::super::TEST_EMIT {
        super::schemes::valid_emit_scheme(&env.defs[index])
    } else if effect == super::super::TEST_CLOCK {
        super::schemes::valid_clock_scheme(&env.defs[index])
    } else if effect == super::super::TEST_SEND {
        super::schemes::valid_send_scheme(&env.defs[index])
    } else {
        effect == super::super::TEST_CODE_EXPORT
            && super::schemes::valid_code_export_scheme(&env.defs[index])
    };
    if !is_valid_scheme {
        return false;
    }
    let Ok(def) = u32::try_from(index) else {
        return false;
    };
    let Some(row) = env
        .bound_adapters
        .iter()
        .find(|row| row.definition == super::super::Definition(def) && row.adapter_slot == slot)
    else {
        return false;
    };
    if row.effects.as_slice() != [effect] {
        return false;
    }
    matches!(
        env.matching_bound_rows(super::super::Definition(def), slot),
        1
    ) && matches!(env.definition_owners.get(index), Some(Some(_)))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
fn bound_adapters_unique(env: &super::super::Env) -> bool {
    let mut index = 0;
    let mut is_unique = true;
    while index < env.bound_adapters.len() && is_unique {
        let definition = env.bound_adapters[index].definition;
        let slot = env.bound_adapters[index].adapter_slot;
        is_unique = matches!(
            env.kind(definition),
            Some(super::super::Behavior::BoundEmit(actual)
                | super::super::Behavior::BoundClock(actual)
                | super::super::Behavior::BoundCodeExport(actual)
                | super::super::Behavior::BoundSend(actual)) if actual == slot
        );
        let mut prior = 0;
        while prior < index && is_unique {
            let prev = &env.bound_adapters[prior];
            is_unique = prev.adapter_slot != slot && prev.definition != definition;
            prior += 1;
        }
        index += 1;
    }
    is_unique
}

pub(super) fn valid_program_components(
    env: &super::super::Env,
    stack_in: &[crate::types::Ty],
    stack_out: &[crate::types::Ty],
    effects: &crate::types::EffSet,
    max_nodes: u32,
) -> bool {
    let ids = effects.as_slice();
    let mut index = 0;
    let mut is_effect_set_known = true;
    while index < ids.len() && is_effect_set_known {
        is_effect_set_known = env.knows_effect(ids[index]);
        index += 1;
    }
    is_effect_set_known
        && u64::try_from(stack_in.len().saturating_add(stack_out.len()))
            .is_ok_and(|count| count <= u64::from(max_nodes))
}
