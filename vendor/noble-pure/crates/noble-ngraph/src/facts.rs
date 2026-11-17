//! D9 checked facts: the checker's own transfer rules (`derived`), the
//! refinement applied by input validation and guards, and the runtime fact
//! test used by input validation and `guard` nodes.
//!
//! Rules (fact-rule revision `ngraph-facts/0`): intervals propagate through
//! `rn` by evaluating the same rounded operation on the endpoints (rounding is
//! monotone); any NaN-capable result becomes unknown; `step` yields `bit`;
//! `select` of two bits yields `bit`; integral ± / × integral with every
//! rounded bound `< 2^24` in magnitude yields `integral`. These are the
//! checker's rules; NG-PO-E (fact-transfer soundness) remains open.

/// 2^24 as `f32`.
pub const TWO_POW_24: f32 = 16_777_216.0;

/// Abstract summary of one `F32` value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbsF32 {
    /// Numeric lower bound (`-0` and `+0` equal).
    pub lo: f32,
    /// Numeric upper bound.
    pub hi: f32,
    /// May be NaN.
    pub nan: bool,
    /// Certainly a finite integer.
    pub integral: bool,
    /// Certainly `+0` or `1` (bitwise).
    pub bit: bool,
}

impl AbsF32 {
    /// No information.
    pub const UNKNOWN: Self = Self {
        lo: f32::NEG_INFINITY,
        hi: f32::INFINITY,
        nan: true,
        integral: false,
        bit: false,
    };

    /// Exactly `v`.
    #[must_use]
    pub fn exact(v: f32) -> Self {
        if v.is_nan() {
            return Self::UNKNOWN;
        }
        Self {
            lo: v,
            hi: v,
            nan: false,
            integral: v.is_finite() && crate::sem::soft::trunc_f32(v) == v,
            bit: crate::sem::is_bit(v.to_bits()),
        }
    }

    fn interval(lo: f32, hi: f32, nan: bool) -> Self {
        if lo.is_nan() || hi.is_nan() {
            return Self::UNKNOWN;
        }
        Self {
            lo,
            hi,
            nan,
            integral: false,
            bit: false,
        }
    }

    /// Certainly finite.
    #[must_use]
    pub fn finite(&self) -> bool {
        !self.nan && self.lo.is_finite() && self.hi.is_finite()
    }

    fn may_inf(&self) -> bool {
        self.lo.is_infinite() || self.hi.is_infinite()
    }

    fn contains_zero(&self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
    }

    fn small_integral(&self) -> bool {
        self.integral && self.lo.abs() < TWO_POW_24 && self.hi.abs() < TWO_POW_24
    }

    /// `rn(a + b)`.
    #[must_use]
    pub fn radd(a: Self, b: Self) -> Self {
        let is_nan = a.nan || b.nan || (a.may_inf() && b.may_inf());
        let mut r = Self::interval(a.lo + b.lo, a.hi + b.hi, is_nan);
        r.integral = a.small_integral() && b.small_integral() && r.finite() && r.small_bounds();
        r
    }

    /// `rn(a - b)`.
    #[must_use]
    pub fn rsub(a: Self, b: Self) -> Self {
        let is_nan = a.nan || b.nan || (a.may_inf() && b.may_inf());
        let mut r = Self::interval(a.lo - b.hi, a.hi - b.lo, is_nan);
        r.integral = a.small_integral() && b.small_integral() && r.finite() && r.small_bounds();
        r
    }

    /// `rn(a × b)`.
    #[must_use]
    pub fn rmul(a: Self, b: Self) -> Self {
        let is_nan = a.nan
            || b.nan
            || (a.may_inf() && b.contains_zero())
            || (b.may_inf() && a.contains_zero());
        let p = [a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi];
        if p.iter().any(|x| x.is_nan()) {
            return Self::UNKNOWN;
        }
        let lo = p.iter().copied().fold(f32::INFINITY, f32::min);
        let hi = p.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut r = Self::interval(lo, hi, is_nan);
        r.integral = a.small_integral() && b.small_integral() && r.finite() && r.small_bounds();
        r
    }

    fn small_bounds(&self) -> bool {
        self.lo.abs() < TWO_POW_24 && self.hi.abs() < TWO_POW_24
    }

    /// `relu`.
    #[must_use]
    pub fn relu(x: Self) -> Self {
        let lo = if x.lo > 0.0 { x.lo } else { 0.0 };
        let hi = if x.hi > 0.0 { x.hi } else { 0.0 };
        let mut r = Self::interval(lo, hi, x.nan);
        r.integral = x.integral;
        r.bit = x.bit;
        r
    }

    /// `step` (on its checked range).
    #[must_use]
    pub const fn step() -> Self {
        Self {
            lo: 0.0,
            hi: 1.0,
            nan: false,
            integral: true,
            bit: true,
        }
    }

    /// Hull of two summaries (`select`, index candidates).
    #[must_use]
    pub fn hull(a: Self, b: Self) -> Self {
        Self {
            lo: a.lo.min(b.lo),
            hi: a.hi.max(b.hi),
            nan: a.nan || b.nan,
            integral: a.integral && b.integral,
            bit: a.bit && b.bit,
        }
    }

    /// Refines with a fact known to hold.
    #[must_use]
    #[expect(
        tigerstyle::no_recursion,
        reason = "Owner: noble-maintainers; refinement follows one fact implication chain bounded by the fact variant set"
    )]
    pub fn refine(mut self, fact: &crate::ir::Fact) -> Self {
        match fact {
            crate::ir::Fact::Range { lo, hi } => {
                if let (crate::ir::Literal::F32(l), crate::ir::Literal::F32(h)) = (lo, hi) {
                    self.lo = self.lo.max(f32::from_bits(*l));
                    self.hi = self.hi.min(f32::from_bits(*h));
                    self.nan = false;
                }
            }
            crate::ir::Fact::Bit => {
                self.lo = self.lo.max(0.0);
                self.hi = self.hi.min(1.0);
                self.nan = false;
                self.integral = true;
                self.bit = true;
            }
            crate::ir::Fact::OneHot { .. } => {
                return self.refine(&crate::ir::Fact::Bit);
            }
            crate::ir::Fact::Integral => {
                self.nan = false;
                self.lo = self.lo.max(f32::MIN);
                self.hi = self.hi.min(f32::MAX);
                self.integral = true;
            }
            crate::ir::Fact::Finite => {
                self.nan = false;
                self.lo = self.lo.max(f32::MIN);
                self.hi = self.hi.min(f32::MAX);
            }
            crate::ir::Fact::Grid { log2 } => {
                self.nan = false;
                self.lo = self.lo.max(f32::MIN);
                self.hi = self.hi.min(f32::MAX);
                if *log2 >= 0 {
                    self.integral = true;
                }
            }
            crate::ir::Fact::Shape(_) => {}
        }
        self
    }

    /// Whether this summary implies `fact` (the `derived` check).
    #[must_use]
    pub fn implies(&self, fact: &crate::ir::Fact) -> bool {
        match fact {
            crate::ir::Fact::Range { lo, hi } => match (lo, hi) {
                (crate::ir::Literal::F32(l), crate::ir::Literal::F32(h)) => {
                    !self.nan && self.lo >= f32::from_bits(*l) && self.hi <= f32::from_bits(*h)
                }
                _ => false,
            },
            crate::ir::Fact::Bit => self.bit,
            crate::ir::Fact::Integral => self.integral,
            crate::ir::Fact::Finite => self.finite(),
            crate::ir::Fact::Grid { log2 } => {
                (*log2 <= 0 && self.integral)
                    || (self.lo == self.hi && !self.nan && grid_holds_f32(self.lo, *log2))
            }
            crate::ir::Fact::OneHot { .. } | crate::ir::Fact::Shape(_) => false,
        }
    }
}

/// Abstract summary of one `I64` value (non-wrapping interval).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbsI64 {
    /// Lower bound.
    pub lo: i64,
    /// Upper bound.
    pub hi: i64,
}

impl AbsI64 {
    /// No information.
    pub const UNKNOWN: Self = Self {
        lo: i64::MIN,
        hi: i64::MAX,
    };

    fn from_i128(lo: i128, hi: i128) -> Self {
        match (i64::try_from(lo), i64::try_from(hi)) {
            (Ok(lo), Ok(hi)) => Self { lo, hi },
            _ => Self::UNKNOWN,
        }
    }

    /// Wrapping `a + b` (unknown if it may wrap).
    #[must_use]
    pub fn radd(a: Self, b: Self) -> Self {
        Self::from_i128(
            i128::from(a.lo) + i128::from(b.lo),
            i128::from(a.hi) + i128::from(b.hi),
        )
    }

    /// Wrapping `a - b`.
    #[must_use]
    pub fn rsub(a: Self, b: Self) -> Self {
        Self::from_i128(
            i128::from(a.lo) - i128::from(b.hi),
            i128::from(a.hi) - i128::from(b.lo),
        )
    }

    /// Wrapping `a × b`.
    #[must_use]
    pub fn rmul(a: Self, b: Self) -> Self {
        let p = [
            i128::from(a.lo) * i128::from(b.lo),
            i128::from(a.lo) * i128::from(b.hi),
            i128::from(a.hi) * i128::from(b.lo),
            i128::from(a.hi) * i128::from(b.hi),
        ];
        let lo = p[0].min(p[1]).min(p[2]).min(p[3]);
        let hi = p[0].max(p[1]).max(p[2]).max(p[3]);
        Self::from_i128(lo, hi)
    }

    /// Refines with a fact known to hold.
    #[must_use]
    pub fn refine(mut self, fact: &crate::ir::Fact) -> Self {
        match fact {
            crate::ir::Fact::Range {
                lo: crate::ir::Literal::I64(l),
                hi: crate::ir::Literal::I64(h),
            } => {
                self.lo = self.lo.max(*l);
                self.hi = self.hi.min(*h);
            }
            crate::ir::Fact::Bit | crate::ir::Fact::OneHot { .. } => {
                self.lo = self.lo.max(0);
                self.hi = self.hi.min(1);
            }
            _ => {}
        }
        self
    }

    /// Whether this summary implies `fact`.
    #[must_use]
    pub fn implies(&self, fact: &crate::ir::Fact) -> bool {
        match fact {
            crate::ir::Fact::Range {
                lo: crate::ir::Literal::I64(l),
                hi: crate::ir::Literal::I64(h),
            } => self.lo >= *l && self.hi <= *h,
            crate::ir::Fact::Bit => self.lo >= 0 && self.hi <= 1,
            crate::ir::Fact::Integral | crate::ir::Fact::Finite => true,
            crate::ir::Fact::Grid { log2 } => {
                *log2 <= 0 || (self.lo == self.hi && grid_holds_i64(self.lo, *log2))
            }
            _ => false,
        }
    }
}

/// Abstract summary of any value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Abs {
    /// `F32`.
    F32(AbsF32),
    /// `I64`.
    I64(AbsI64),
    /// `Bool` (no information tracked).
    Bool,
    /// Guard root (no value).
    None,
}

impl Abs {
    /// Refines with a fact known to hold.
    #[must_use]
    pub fn refine(self, fact: &crate::ir::Fact) -> Self {
        match self {
            Self::F32(a) => Self::F32(a.refine(fact)),
            Self::I64(a) => Self::I64(a.refine(fact)),
            other => other,
        }
    }

    /// Whether the summary implies `fact`.
    #[must_use]
    pub fn implies(&self, fact: &crate::ir::Fact) -> bool {
        match self {
            Self::F32(a) => a.implies(fact),
            Self::I64(a) => a.implies(fact),
            Self::Bool => matches!(
                fact,
                crate::ir::Fact::Bit | crate::ir::Fact::Finite | crate::ir::Fact::Integral
            ),
            Self::None => false,
        }
    }
}

fn grid_holds_f32(v: f32, log2: i32) -> bool {
    if !v.is_finite() {
        return false;
    }
    // Every finite binary32 value is an integer multiple of 2^-149, the smallest
    // subnormal step, so a grid at or below that step holds for every finite value.
    if log2 <= -149 {
        return true;
    }
    // f64 scaling by 2^-k is exact for every binary32 value and k ≤ 300; finer than
    // that no nonzero value is a multiple.
    if log2 > 300 {
        return v == 0.0;
    }
    let scaled = f64::from(v) * crate::sem::soft::pow2_f64(-log2);
    crate::sem::soft::trunc_f64(scaled) == scaled
}

fn grid_holds_i64(v: i64, log2: i32) -> bool {
    if log2 <= 0 {
        return true;
    }
    if log2 >= 64 {
        return v == 0;
    }
    v.trailing_zeros() >= log2.unsigned_abs() || v == 0
}

/// Whether a fact payload is well-formed for a value of type `ty`.
#[must_use]
pub fn payload_ok(fact: &crate::ir::Fact, ty: crate::ir::ScalarType) -> bool {
    match (fact, ty) {
        (crate::ir::Fact::Range { lo, hi }, crate::ir::ScalarType::F32) => match (lo, hi) {
            (crate::ir::Literal::F32(l), crate::ir::Literal::F32(h)) => {
                let (l, h) = (f32::from_bits(*l), f32::from_bits(*h));
                !l.is_nan() && !h.is_nan() && l <= h
            }
            _ => false,
        },
        (crate::ir::Fact::Range { lo, hi }, crate::ir::ScalarType::I64) => {
            matches!((lo, hi), (crate::ir::Literal::I64(l), crate::ir::Literal::I64(h)) if l <= h)
        }
        (
            crate::ir::Fact::Range { .. } | crate::ir::Fact::Grid { .. },
            crate::ir::ScalarType::Bool,
        ) => false,
        (crate::ir::Fact::OneHot { group, .. }, _) => !group.is_empty(),
        _ => true,
    }
}

/// Runtime test of `fact` on `v` (input validation and `guard`). `group`
/// supplies the values of a one-hot group in order; `shape` the structural
/// table length of the subject node, if any.
#[must_use]
pub fn holds(
    fact: &crate::ir::Fact,
    v: crate::sem::Value,
    group: &[crate::sem::Value],
    shape: Option<u32>,
) -> bool {
    match (fact, v) {
        (crate::ir::Fact::Range { lo, hi }, crate::sem::Value::F32(b)) => match (lo, hi) {
            (crate::ir::Literal::F32(l), crate::ir::Literal::F32(h)) => {
                let x = f32::from_bits(b);
                !x.is_nan() && x >= f32::from_bits(*l) && x <= f32::from_bits(*h)
            }
            _ => false,
        },
        (crate::ir::Fact::Range { lo, hi }, crate::sem::Value::I64(x)) => match (lo, hi) {
            (crate::ir::Literal::I64(l), crate::ir::Literal::I64(h)) => *l <= x && x <= *h,
            _ => false,
        },
        (crate::ir::Fact::Grid { log2 }, crate::sem::Value::F32(b)) => {
            grid_holds_f32(f32::from_bits(b), *log2)
        }
        (crate::ir::Fact::Grid { log2 }, crate::sem::Value::I64(x)) => grid_holds_i64(x, *log2),
        (crate::ir::Fact::Bit, crate::sem::Value::F32(b)) => crate::sem::is_bit(b),
        (crate::ir::Fact::Bit, crate::sem::Value::I64(x)) => x == 0 || x == 1,
        (crate::ir::Fact::Bit, crate::sem::Value::Bool(_))
        | (
            crate::ir::Fact::Finite | crate::ir::Fact::Integral,
            crate::sem::Value::I64(_) | crate::sem::Value::Bool(_),
        ) => true,
        (crate::ir::Fact::Finite, crate::sem::Value::F32(b)) => f32::from_bits(b).is_finite(),
        (crate::ir::Fact::Integral, crate::sem::Value::F32(b)) => {
            let x = f32::from_bits(b);
            x.is_finite() && crate::sem::soft::trunc_f32(x) == x
        }
        (crate::ir::Fact::OneHot { allow_zero, .. }, _) => {
            let mut ones = 0usize;
            for g in group {
                let is_one = match *g {
                    crate::sem::Value::F32(b) if crate::sem::is_bit(b) => b == crate::sem::ONE,
                    crate::sem::Value::I64(x) if x == 0 || x == 1 => x == 1,
                    crate::sem::Value::Bool(x) => x,
                    _ => return false,
                };
                ones += usize::from(is_one);
            }
            ones == 1 || (*allow_zero && ones == 0)
        }
        (crate::ir::Fact::Shape(n), _) => shape == Some(*n),
        _ => false,
    }
}

/// The facts `step`, `select` and `index` require of their operand (D2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// `step`: `s ∈ [0,1]`.
    UnitRange,
    /// `select`: exact bit.
    Bit,
    /// `index`: integral and `|i| < 2^24`.
    Index,
}

impl Requirement {
    /// Whether the summary satisfies the requirement.
    #[must_use]
    pub fn satisfied_by(self, a: &AbsF32) -> bool {
        match self {
            Self::UnitRange => !a.nan && a.lo >= 0.0 && a.hi <= 1.0,
            Self::Bit => a.bit,
            Self::Index => a.integral && !a.nan && a.lo > -TWO_POW_24 && a.hi < TWO_POW_24,
        }
    }

    /// The canonical fact reported when the requirement is unmet.
    #[must_use]
    pub const fn fact(self) -> crate::ir::Fact {
        match self {
            Self::UnitRange => crate::ir::Fact::Range {
                lo: crate::ir::Literal::F32(0),
                hi: crate::ir::Literal::F32(crate::sem::ONE),
            },
            Self::Bit => crate::ir::Fact::Bit,
            Self::Index => crate::ir::Fact::Integral,
        }
    }
}

/// Group values of a one-hot fact (helper for evaluators).
#[must_use]
pub fn group_values(
    fact: &crate::ir::Fact,
    value_of: impl Fn(crate::ir::NodeId) -> crate::sem::Value,
) -> alloc::vec::Vec<crate::sem::Value> {
    match fact {
        crate::ir::Fact::OneHot { group, .. } => group.iter().map(|g| value_of(*g)).collect(),
        _ => alloc::vec::Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    struct TestHasher(u64);

    impl crate::identity::IdentityHasher for TestHasher {
        fn update(&mut self, bytes: &[u8]) {
            for &byte in bytes {
                self.0 = self.0.wrapping_mul(31).wrapping_add(u64::from(byte));
            }
        }

        fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
            let mut digest = [0u8; 32];
            digest[..8].copy_from_slice(&self.0.to_le_bytes());
            digest
        }
    }

    struct TestIds;

    impl crate::identity::IdentityScheme for TestIds {
        fn name(&self) -> &'static str {
            "test"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(TestHasher(0))
        }
    }

    fn grid(log2: i32) -> crate::ir::Fact {
        crate::ir::Fact::Grid { log2 }
    }

    #[test]
    fn a_grid_at_or_below_the_smallest_binary32_step_holds_for_every_finite_value() {
        // Every finite binary32 value is an integer multiple of 2^-149.
        assert!(super::grid_holds_f32(1.0, -301));
        assert!(super::grid_holds_f32(f32::MIN_POSITIVE, -149));
        assert!(super::grid_holds_f32(-f32::MIN_POSITIVE, -149));
        assert!(super::grid_holds_f32(f32::MAX, -149));
        assert!(super::grid_holds_f32(f32::from_bits(1), -149));
        assert!(super::AbsF32::exact(1.0).implies(&grid(-301)));
        assert!(super::holds(
            &grid(-301),
            crate::sem::Value::F32(1.0f32.to_bits()),
            &[],
            None
        ));
        // The coarse direction is unchanged: no nonzero value is a multiple of 2^301,
        // and the smallest subnormal is not a multiple of 2^-148.
        assert!(!super::grid_holds_f32(f32::INFINITY, -301));
        assert!(!super::grid_holds_f32(1.0, 301));
        assert!(super::grid_holds_f32(0.0, 301));
        assert!(!super::grid_holds_f32(f32::from_bits(1), -148));
    }

    #[test]
    fn a_grid_below_the_smallest_binary32_step_passes_a_guard() {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::F32,
            crate::ir::Subject::authored(&scheme, "facts-grid-guard"),
        );
        let one = builder.f32_const(1.0);
        let _guard = builder.guard(grid(-301), one);
        builder.output(one);
        let checked = crate::check::check(&scheme, builder.finish())
            .expect("the guarded constant graph is admitted");
        let mut control = crate::eval::Control {
            fuel: None,
            cancel: None,
            scratch: None,
            forward_scratch: None,
            evaluated_nodes: None,
        };
        let report = crate::eval::reference(&checked, &[], None, &mut control);
        assert_eq!(
            report.obs,
            crate::eval::Obs::Normal(alloc::vec![crate::sem::Value::F32(1.0f32.to_bits())]),
            "the runtime fact test agrees with the static implication"
        );
    }
}
