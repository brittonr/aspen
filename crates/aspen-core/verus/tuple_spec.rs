//! Tuple Layer State Machine Model
//!
//! Abstract state model for formal verification of tuple encoding operations.
//!
//! # State Model
//!
//! The `TupleSpec` captures:
//! - Sequence of elements (ints, bytes, strings, nested tuples)
//! - Encoding/decoding functions
//!
//! # Key Invariants
//!
//! 1. **TUPLE-1: Order Preservation**: Encoded bytes preserve tuple ordering
//! 2. **TUPLE-2: Roundtrip Correctness**: decode(encode(t)) == t
//! 3. **TUPLE-3: Prefix Property**: Tuple prefixes encode to byte prefixes
//! 4. **TUPLE-4: Null Escaping**: Null bytes properly escaped
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-core/verus/tuple_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Element Types
    // ========================================================================

    /// Abstract tuple element type
    pub enum ElementSpec {
        /// Null/none value
        Null,
        /// Signed integer (-2^63 to 2^63-1)
        Int(i64),
        /// Byte array
        Bytes(Seq<u8>),
        /// UTF-8 string
        String(Seq<u8>), // UTF-8 encoded
        /// Nested tuple
        Tuple(TupleSpec),
    }

    /// Abstract tuple structure
    pub struct TupleSpec {
        /// Ordered sequence of elements
        pub elements: Seq<ElementSpec>,
    }

    // ========================================================================
    // Size Functions for Termination Checking
    // ========================================================================
    //
    // These functions define the size measure for ElementSpec and TupleSpec.
    // well-foundedness measure used by comparison functions. The functions
    // are structurally recursive on the algebraic data types and trivially
    // terminate.

    /// Size of a tuple (sum of element sizes + 1)
    pub open spec fn tuple_size(t: TupleSpec) -> nat
        decreases t,
    {
        1 + elements_size(t.elements)
    }

    /// Size of a sequence of elements
    pub open spec fn elements_size(elems: Seq<ElementSpec>) -> nat
        decreases elems,
    {
        if elems.len() == 0 {
            0
        } else {
            element_size(elems.first()) + elements_size(elems.skip(1))
        }
    }

    /// Size of an element (1 for primitives, recursive for tuples)
    pub open spec fn element_size(e: ElementSpec) -> nat
        decreases e,
    {
        match e {
            ElementSpec::Null => 1,
            ElementSpec::Int(_) => 1,
            ElementSpec::Bytes(_) => 1,
            ElementSpec::String(_) => 1,
            ElementSpec::Tuple(t) => 1 + tuple_size(t),
        }
    }

    // ========================================================================
    // Element Ordering
    // ========================================================================

    /// Ordering of element types (matches FoundationDB/tuple layer convention)
    /// Null < Bytes < String < Nested < Int (negative) < Int (0) < Int (positive)
    pub open spec fn element_type_order(e: ElementSpec) -> int {
        match e {
            ElementSpec::Null => 0,
            ElementSpec::Bytes(_) => 1,
            ElementSpec::String(_) => 2,
            ElementSpec::Tuple(_) => 3,
            ElementSpec::Int(n) => {
                if n < 0 { 4 }
                else if n == 0 { 5 }
                else { 6 }
            }
        }
    }

    /// Compare two elements for ordering using an explicit fuel budget.
    pub open spec fn element_less_than_with_fuel(
        a: ElementSpec,
        b: ElementSpec,
        fuel: nat,
    ) -> bool
        decreases fuel
    {
        if fuel == 0 {
            false
        } else {
            let type_a = element_type_order(a);
            let type_b = element_type_order(b);
            if type_a < type_b {
                true
            } else if type_a > type_b {
                false
            } else {
                match (a, b) {
                    (ElementSpec::Null, ElementSpec::Null) => false,
                    (ElementSpec::Int(na), ElementSpec::Int(nb)) => na < nb,
                    (ElementSpec::Bytes(ba), ElementSpec::Bytes(bb)) => seq_less_than(ba, bb),
                    (ElementSpec::String(sa), ElementSpec::String(sb)) => seq_less_than(sa, sb),
                    (ElementSpec::Tuple(ta), ElementSpec::Tuple(tb)) => {
                        tuple_less_than_with_fuel(ta, tb, (fuel - 1) as nat)
                    },
                    (_, _) => false,
                }
            }
        }
    }

    /// Compare two elements for ordering.
    pub open spec fn element_less_than(a: ElementSpec, b: ElementSpec) -> bool {
        element_less_than_with_fuel(a, b, element_size(a) + element_size(b) + 1)
    }

    /// Lexicographic comparison of byte sequences
    pub open spec fn seq_less_than(a: Seq<u8>, b: Seq<u8>) -> bool
        decreases a.len() + b.len()
    {
        if a.len() == 0 && b.len() == 0 {
            false // equal
        } else if a.len() == 0 {
            true // empty < non-empty
        } else if b.len() == 0 {
            false // non-empty > empty
        } else if a.first() < b.first() {
            true
        } else if a.first() > b.first() {
            false
        } else {
            seq_less_than(a.skip(1), b.skip(1))
        }
    }

    pub proof fn seq_less_than_irreflexive(bytes: Seq<u8>)
        ensures !seq_less_than(bytes, bytes)
        decreases bytes.len()
    {
        if bytes.len() == 0 {
        } else {
            seq_less_than_irreflexive(bytes.skip(1));
        }
    }

    pub proof fn seq_less_than_antisymmetric(a: Seq<u8>, b: Seq<u8>)
        requires seq_less_than(a, b)
        ensures !seq_less_than(b, a)
        decreases a.len() + b.len()
    {
        if a.len() == 0 {
            if b.len() == 0 {
            }
        } else if b.len() == 0 {
        } else if a.first() < b.first() {
        } else if a.first() > b.first() {
        } else {
            seq_less_than_antisymmetric(a.skip(1), b.skip(1));
        }
    }

    pub proof fn seq_less_than_right_equivalent(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
        requires
            seq_less_than(a, b),
            !seq_less_than(c, b),
        ensures seq_less_than(a, c)
        decreases a.len() + b.len() + c.len()
    {
        if a.len() == 0 {
            if c.len() == 0 {
                seq_less_than_antisymmetric(b, c);
            }
        } else if b.len() == 0 {
        } else if c.len() == 0 {
        } else if a.first() < b.first() {
        } else if a.first() > b.first() {
        } else {
            if c.first() < b.first() {
            } else if c.first() > b.first() {
            } else {
                seq_less_than_right_equivalent(a.skip(1), b.skip(1), c.skip(1));
            }
        }
    }

    pub proof fn seq_less_than_left_equivalent(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
        requires
            !seq_less_than(b, a),
            seq_less_than(b, c),
        ensures seq_less_than(a, c)
        decreases a.len() + b.len() + c.len()
    {
        if a.len() == 0 {
            if c.len() == 0 {
                seq_less_than_antisymmetric(b, c);
            }
        } else if b.len() == 0 {
        } else if c.len() == 0 {
        } else if b.first() < c.first() {
        } else if b.first() > c.first() {
        } else {
            if b.first() < a.first() {
            } else if b.first() > a.first() {
            } else {
                seq_less_than_left_equivalent(a.skip(1), b.skip(1), c.skip(1));
            }
        }
    }

    pub proof fn seq_less_than_transitive(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
        requires
            seq_less_than(a, b),
            seq_less_than(b, c),
        ensures seq_less_than(a, c)
        decreases a.len() + b.len() + c.len()
    {
        if a.len() == 0 {
            if c.len() == 0 {
                seq_less_than_antisymmetric(b, c);
            }
        } else if b.len() == 0 {
        } else if c.len() == 0 {
        } else if a.first() < b.first() {
            if b.first() < c.first() {
            } else if b.first() > c.first() {
            } else {
            }
        } else if a.first() > b.first() {
        } else {
            if b.first() < c.first() {
            } else if b.first() > c.first() {
            } else {
                seq_less_than_transitive(a.skip(1), b.skip(1), c.skip(1));
            }
        }
    }

    /// Compare two tuples lexicographically by elements using an explicit fuel
    /// budget shared with nested element comparison.
    pub open spec fn tuple_less_than_with_fuel(
        a: TupleSpec,
        b: TupleSpec,
        fuel: nat,
    ) -> bool
        decreases fuel
    {
        if fuel == 0 {
            false
        } else if a.elements.len() == 0 && b.elements.len() == 0 {
            false
        } else if a.elements.len() == 0 {
            true
        } else if b.elements.len() == 0 {
            false
        } else {
            let ea = a.elements.first();
            let eb = b.elements.first();
            if element_less_than_with_fuel(ea, eb, (fuel - 1) as nat) {
                true
            } else if element_less_than_with_fuel(eb, ea, (fuel - 1) as nat) {
                false
            } else {
                tuple_less_than_with_fuel(
                    TupleSpec { elements: a.elements.skip(1) },
                    TupleSpec { elements: b.elements.skip(1) },
                    (fuel - 1) as nat,
                )
            }
        }
    }

    /// Compare two tuples lexicographically by elements.
    pub open spec fn tuple_less_than(a: TupleSpec, b: TupleSpec) -> bool {
        tuple_less_than_with_fuel(a, b, tuple_size(a) + tuple_size(b) + 1)
    }

    /// Check if two tuples are equal
    pub open spec fn tuple_equal(a: TupleSpec, b: TupleSpec) -> bool {
        a.elements =~= b.elements
    }

    // ========================================================================
    // Invariant 1: Order Preservation
    // ========================================================================

    /// Abstract pack function (produces byte sequence). Empty tuple encoding is
    /// structural; non-empty encoding remains an opaque runtime encoder model.
    pub uninterp spec fn pack_nonempty(t: TupleSpec) -> Seq<u8>;

    pub open spec fn pack(t: TupleSpec) -> Seq<u8> {
        if t.elements.len() == 0 {
            Seq::empty()
        } else {
            pack_nonempty(t)
        }
    }

    /// TUPLE-1: Order Preservation
    ///
    /// If tuple a < tuple b, then pack(a) < pack(b) lexicographically
    pub open spec fn tuple_order_preservation(a: TupleSpec, b: TupleSpec) -> bool {
        tuple_less_than(a, b) ==> seq_less_than(pack(a), pack(b))
    }

    /// Trusted encoding axiom: order preservation holds for the production
    /// FoundationDB-style tuple encoder (`crates/aspen-layer/src/tuple/`).
    /// Verus models `pack` as uninterpreted, so this is an explicit encoding
    /// trust boundary backed by tuple runtime coverage rather than a local proof
    /// of the encoder implementation. Runtime evidence: `cargo test -p
    /// aspen-layer` exercises `test_trusted_tuple_boundary_runtime_evidence`,
    /// `prop_tuple_ordering`, `prop_int_ordering`, `prop_string_ordering`,
    /// and `prop_bytes_ordering`.
    #[verifier(external_body)]
    pub proof fn order_preservation_holds(a: TupleSpec, b: TupleSpec)
        ensures tuple_order_preservation(a, b)
    {
        // The encoding is designed to preserve lexicographic order:
        // 1. Type codes are ordered (null < bytes < string < int)
        // 2. Within bytes/strings: null escape + lexicographic
        // 3. Within ints: sign-magnitude encoding preserves order
    }

    // ========================================================================
    // Invariant 2: Roundtrip Correctness
    // ========================================================================

    /// Abstract unpack function
    pub uninterp spec fn unpack(bytes: Seq<u8>) -> Option<TupleSpec>;

    /// TUPLE-2: Roundtrip Correctness
    ///
    /// For any tuple t: unpack(pack(t)) == Some(t)
    pub open spec fn tuple_roundtrip(t: TupleSpec) -> bool {
        unpack(pack(t)) == Some(t)
    }

    /// Trusted encoding axiom: production tuple pack/unpack roundtrips all
    /// represented elements. The Verus model keeps `pack`/`unpack`
    /// uninterpreted, so this marker names the external encoder/decoder
    /// correctness boundary. Runtime evidence: `cargo test -p aspen-layer`
    /// exercises `test_trusted_tuple_boundary_runtime_evidence` and
    /// `prop_roundtrip`.
    #[verifier(external_body)]
    pub proof fn roundtrip_holds(t: TupleSpec)
        ensures tuple_roundtrip(t)
    {
        // The encoding is bijective:
        // 1. Each element type has unique type code
        // 2. Length-prefixed or null-terminated encoding is unambiguous
        // 3. Null escaping makes embedded nulls recoverable
    }

    // ========================================================================
    // Invariant 3: Prefix Property
    // ========================================================================

    /// Get prefix of a tuple (first n elements)
    pub open spec fn tuple_prefix(t: TupleSpec, n: int) -> TupleSpec {
        TupleSpec { elements: t.elements.take(n) }
    }

    /// Check if bytes are a prefix of other bytes
    pub open spec fn is_byte_prefix(prefix: Seq<u8>, full: Seq<u8>) -> bool {
        prefix.len() <= full.len() &&
        forall |i: int| 0 <= i < prefix.len() ==> prefix[i] == full[i]
    }

    /// TUPLE-3: Prefix Property
    ///
    /// If tuple p is a prefix of tuple t, then pack(p) is a byte prefix of pack(t)
    pub open spec fn tuple_prefix_property(p: TupleSpec, t: TupleSpec, n: int) -> bool {
        (p == tuple_prefix(t, n)) ==> is_byte_prefix(pack(p), pack(t))
    }

    /// Trusted encoding axiom: encoded tuple prefixes are byte prefixes of the
    /// full encoded tuple under the production tuple encoder. This depends on
    /// the uninterpreted `pack` boundary, not on local structural reasoning.
    /// Runtime evidence: `cargo test -p aspen-layer` exercises
    /// `test_trusted_tuple_boundary_runtime_evidence`,
    /// `prop_prefix_stability`, and `prop_range_captures_prefix`.
    #[verifier(external_body)]
    pub proof fn prefix_property_holds(t: TupleSpec, n: int)
        requires 0 <= n <= t.elements.len()
        ensures tuple_prefix_property(tuple_prefix(t, n), t, n)
    {
        // The encoding is designed so that:
        // 1. Each element is encoded independently
        // 2. Concatenation of element encodings = full tuple encoding
        // 3. Therefore prefix elements = prefix of encoded bytes
    }

    // ========================================================================
    // Invariant 4: Null Escaping
    // ========================================================================

    /// TUPLE-4: Null Escaping
    ///
    /// Null bytes (0x00) in bytes/strings are escaped so they don't
    /// interfere with element boundary markers.
    ///
    /// The encoding uses 0x00 as element terminator. To allow 0x00 within data:
    /// - 0x00 in data is escaped as 0x00 0xFF
    /// - A bare 0x00 (not followed by 0xFF) marks end of element
    ///
    /// This spec verifies that the escaping preserves the original data through roundtrip.
    pub open spec fn null_escaping_correct(bytes: Seq<u8>) -> bool {
        // The encoding must roundtrip correctly for bytes containing nulls:
        // For any byte sequence (including those with embedded nulls),
        // pack then unpack must return the original sequence.
        let t = TupleSpec { elements: seq![ElementSpec::Bytes(bytes)] };
        tuple_roundtrip(t)
    }

    /// Check if a byte sequence contains null bytes
    pub open spec fn contains_null(bytes: Seq<u8>) -> bool {
        exists |i: int| 0 <= i < bytes.len() && bytes[i] == 0u8
    }

    /// Trusted encoding axiom: production byte/string tuple encoding escapes
    /// embedded NUL bytes and decodes them back. This marker is intentionally
    /// tied to the runtime tuple encoder's NUL-escaping behavior. Runtime
    /// evidence: `cargo test -p aspen-layer` exercises
    /// `test_trusted_tuple_boundary_runtime_evidence`, `prop_special_strings`,
    /// `prop_string_ordering`, and `prop_bytes_ordering`.
    #[verifier(external_body)]
    pub proof fn null_bytes_roundtrip(bytes: Seq<u8>)
        ensures ({
            let t = TupleSpec { elements: seq![ElementSpec::Bytes(bytes)] };
            tuple_roundtrip(t)
        })
    {
        // The encoding escapes 0x00 as 0x00 0xFF
        // Decoding reverses this transformation
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined tuple invariant
    pub open spec fn tuple_invariant(t: TupleSpec) -> bool {
        // Roundtrip correctness is the primary correctness property
        tuple_roundtrip(t)
    }

    // ========================================================================
    // Constructor Operations
    // ========================================================================

    /// Empty tuple
    pub open spec fn empty_tuple() -> TupleSpec {
        TupleSpec { elements: Seq::empty() }
    }

    /// Push an element onto a tuple
    pub open spec fn push_element(t: TupleSpec, e: ElementSpec) -> TupleSpec {
        TupleSpec { elements: t.elements.push(e) }
    }

    /// Get tuple length
    pub open spec fn tuple_len(t: TupleSpec) -> int {
        t.elements.len() as int
    }

    /// Get element at index
    pub open spec fn get_element(t: TupleSpec, i: int) -> ElementSpec {
        t.elements[i]
    }

    // ========================================================================
    // Proofs
    // ========================================================================

    /// Empty tuple encoding follows from the structural `pack` model.
    pub proof fn empty_tuple_pack()
        ensures pack(empty_tuple()).len() == 0
    {
        // Empty tuple has no elements, packs to empty bytes
    }

    /// Proof: Push increases tuple length
    pub proof fn push_increases_length(t: TupleSpec, e: ElementSpec)
        ensures tuple_len(push_element(t, e)) == tuple_len(t) + 1
    {
        // Direct from Seq::push specification
    }

    // ========================================================================
    // Lexicographic Ordering Lemmas
    // ========================================================================

    pub proof fn element_less_than_with_fuel_irreflexive(e: ElementSpec, fuel: nat)
        ensures !element_less_than_with_fuel(e, e, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else {
            match e {
                ElementSpec::Bytes(bytes) => {
                    seq_less_than_irreflexive(bytes);
                },
                ElementSpec::String(bytes) => {
                    seq_less_than_irreflexive(bytes);
                },
                ElementSpec::Tuple(t) => {
                    tuple_less_than_with_fuel_irreflexive(t, (fuel - 1) as nat);
                },
                _ => {},
            }
        }
    }

    pub proof fn tuple_less_than_with_fuel_irreflexive(t: TupleSpec, fuel: nat)
        ensures !tuple_less_than_with_fuel(t, t, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else if t.elements.len() == 0 {
        } else {
            let first = t.elements.first();
            element_less_than_with_fuel_irreflexive(first, (fuel - 1) as nat);
            tuple_less_than_with_fuel_irreflexive(
                TupleSpec { elements: t.elements.skip(1) },
                (fuel - 1) as nat,
            );
        }
    }

    pub proof fn element_less_than_with_fuel_antisymmetric(
        a: ElementSpec,
        b: ElementSpec,
        fuel: nat,
    )
        requires element_less_than_with_fuel(a, b, fuel)
        ensures !element_less_than_with_fuel(b, a, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else {
            match (a, b) {
                (ElementSpec::Bytes(ba), ElementSpec::Bytes(bb)) => {
                    seq_less_than_antisymmetric(ba, bb);
                },
                (ElementSpec::String(sa), ElementSpec::String(sb)) => {
                    seq_less_than_antisymmetric(sa, sb);
                },
                (ElementSpec::Tuple(ta), ElementSpec::Tuple(tb)) => {
                    tuple_less_than_with_fuel_antisymmetric(ta, tb, (fuel - 1) as nat);
                },
                (_, _) => {},
            }
        }
    }

    pub proof fn tuple_less_than_with_fuel_antisymmetric(
        a: TupleSpec,
        b: TupleSpec,
        fuel: nat,
    )
        requires tuple_less_than_with_fuel(a, b, fuel)
        ensures !tuple_less_than_with_fuel(b, a, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else if a.elements.len() == 0 {
        } else if b.elements.len() == 0 {
        } else {
            let ea = a.elements.first();
            let eb = b.elements.first();
            if element_less_than_with_fuel(ea, eb, (fuel - 1) as nat) {
                element_less_than_with_fuel_antisymmetric(ea, eb, (fuel - 1) as nat);
            } else if element_less_than_with_fuel(eb, ea, (fuel - 1) as nat) {
            } else {
                tuple_less_than_with_fuel_antisymmetric(
                    TupleSpec { elements: a.elements.skip(1) },
                    TupleSpec { elements: b.elements.skip(1) },
                    (fuel - 1) as nat,
                );
            }
        }
    }

    pub proof fn element_less_than_with_fuel_transitive(
        a: ElementSpec,
        b: ElementSpec,
        c: ElementSpec,
        fuel: nat,
    )
        requires
            element_less_than_with_fuel(a, b, fuel),
            element_less_than_with_fuel(b, c, fuel),
        ensures element_less_than_with_fuel(a, c, fuel)
        decreases fuel
    {
        element_less_than_with_fuel_antisymmetric(b, c, fuel);
        element_less_than_with_fuel_right_equivalent(a, b, c, fuel);
    }

    pub proof fn element_less_than_with_fuel_right_equivalent(
        a: ElementSpec,
        b: ElementSpec,
        c: ElementSpec,
        fuel: nat,
    )
        requires
            element_less_than_with_fuel(a, b, fuel),
            !element_less_than_with_fuel(c, b, fuel),
        ensures element_less_than_with_fuel(a, c, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else {
            let type_a = element_type_order(a);
            let type_b = element_type_order(b);
            let type_c = element_type_order(c);
            if type_a < type_b {
            } else if type_a > type_b {
            } else {
                if type_c < type_b {
                } else if type_c > type_b {
                } else {
                    match (a, b, c) {
                        (ElementSpec::Bytes(ba), ElementSpec::Bytes(bb), ElementSpec::Bytes(bc)) => {
                            seq_less_than_right_equivalent(ba, bb, bc);
                        },
                        (ElementSpec::String(sa), ElementSpec::String(sb), ElementSpec::String(sc)) => {
                            seq_less_than_right_equivalent(sa, sb, sc);
                        },
                        (ElementSpec::Tuple(ta), ElementSpec::Tuple(tb), ElementSpec::Tuple(tc)) => {
                            tuple_less_than_with_fuel_right_equivalent(
                                ta,
                                tb,
                                tc,
                                (fuel - 1) as nat,
                            );
                        },
                        (_, _, _) => {},
                    }
                }
            }
        }
    }

    pub proof fn element_less_than_with_fuel_left_equivalent(
        a: ElementSpec,
        b: ElementSpec,
        c: ElementSpec,
        fuel: nat,
    )
        requires
            !element_less_than_with_fuel(b, a, fuel),
            element_less_than_with_fuel(b, c, fuel),
        ensures element_less_than_with_fuel(a, c, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else {
            let type_a = element_type_order(a);
            let type_b = element_type_order(b);
            let type_c = element_type_order(c);
            if type_b < type_c {
            } else if type_b > type_c {
            } else {
                if type_b < type_a {
                } else if type_b > type_a {
                } else {
                    match (a, b, c) {
                        (ElementSpec::Bytes(ba), ElementSpec::Bytes(bb), ElementSpec::Bytes(bc)) => {
                            seq_less_than_left_equivalent(ba, bb, bc);
                        },
                        (ElementSpec::String(sa), ElementSpec::String(sb), ElementSpec::String(sc)) => {
                            seq_less_than_left_equivalent(sa, sb, sc);
                        },
                        (ElementSpec::Tuple(ta), ElementSpec::Tuple(tb), ElementSpec::Tuple(tc)) => {
                            tuple_less_than_with_fuel_left_equivalent(
                                ta,
                                tb,
                                tc,
                                (fuel - 1) as nat,
                            );
                        },
                        (_, _, _) => {},
                    }
                }
            }
        }
    }

    pub proof fn tuple_less_than_with_fuel_transitive(
        a: TupleSpec,
        b: TupleSpec,
        c: TupleSpec,
        fuel: nat,
    )
        requires
            tuple_less_than_with_fuel(a, b, fuel),
            tuple_less_than_with_fuel(b, c, fuel),
        ensures tuple_less_than_with_fuel(a, c, fuel)
        decreases fuel
    {
        tuple_less_than_with_fuel_antisymmetric(b, c, fuel);
        tuple_less_than_with_fuel_right_equivalent(a, b, c, fuel);
    }

    pub proof fn tuple_less_than_with_fuel_right_equivalent(
        a: TupleSpec,
        b: TupleSpec,
        c: TupleSpec,
        fuel: nat,
    )
        requires
            tuple_less_than_with_fuel(a, b, fuel),
            !tuple_less_than_with_fuel(c, b, fuel),
        ensures tuple_less_than_with_fuel(a, c, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else if a.elements.len() == 0 {
            if c.elements.len() == 0 {
                tuple_less_than_with_fuel_antisymmetric(b, c, fuel);
            }
        } else if b.elements.len() == 0 {
            assert(false);
        } else if c.elements.len() == 0 {
            assert(tuple_less_than_with_fuel(c, b, fuel));
            assert(false);
        } else {
            let ea = a.elements.first();
            let eb = b.elements.first();
            let ec = c.elements.first();
            let next_fuel = (fuel - 1) as nat;
            if element_less_than_with_fuel(ea, eb, next_fuel) {
                if element_less_than_with_fuel(ec, eb, next_fuel) {
                    assert(tuple_less_than_with_fuel(c, b, fuel));
                    assert(false);
                } else {
                    element_less_than_with_fuel_right_equivalent(ea, eb, ec, next_fuel);
                    assert(element_less_than_with_fuel(ea, ec, next_fuel));
                    assert(tuple_less_than_with_fuel(a, c, fuel));
                }
            } else if element_less_than_with_fuel(eb, ea, next_fuel) {
            } else {
                if element_less_than_with_fuel(ec, eb, next_fuel) {
                    assert(tuple_less_than_with_fuel(c, b, fuel));
                    assert(false);
                } else if element_less_than_with_fuel(eb, ec, next_fuel) {
                    element_less_than_with_fuel_left_equivalent(ea, eb, ec, next_fuel);
                    assert(element_less_than_with_fuel(ea, ec, next_fuel));
                    assert(tuple_less_than_with_fuel(a, c, fuel));
                } else {
                    assert(tuple_less_than_with_fuel(
                        TupleSpec { elements: a.elements.skip(1) },
                        TupleSpec { elements: b.elements.skip(1) },
                        next_fuel,
                    ));
                    assert(!tuple_less_than_with_fuel(
                        TupleSpec { elements: c.elements.skip(1) },
                        TupleSpec { elements: b.elements.skip(1) },
                        next_fuel,
                    ));
                    tuple_less_than_with_fuel_right_equivalent(
                        TupleSpec { elements: a.elements.skip(1) },
                        TupleSpec { elements: b.elements.skip(1) },
                        TupleSpec { elements: c.elements.skip(1) },
                        next_fuel,
                    );
                    assert(tuple_less_than_with_fuel(
                        TupleSpec { elements: a.elements.skip(1) },
                        TupleSpec { elements: c.elements.skip(1) },
                        next_fuel,
                    ));
                    if element_less_than_with_fuel(ea, ec, next_fuel) {
                        element_less_than_with_fuel_right_equivalent(ea, ec, eb, next_fuel);
                        assert(false);
                    }
                    if element_less_than_with_fuel(ec, ea, next_fuel) {
                        element_less_than_with_fuel_right_equivalent(ec, ea, eb, next_fuel);
                        assert(false);
                    }
                    assert(!element_less_than_with_fuel(ea, ec, next_fuel));
                    assert(!element_less_than_with_fuel(ec, ea, next_fuel));
                    assert(tuple_less_than_with_fuel(a, c, fuel));
                }
            }
        }
    }

    pub proof fn tuple_less_than_with_fuel_left_equivalent(
        a: TupleSpec,
        b: TupleSpec,
        c: TupleSpec,
        fuel: nat,
    )
        requires
            !tuple_less_than_with_fuel(b, a, fuel),
            tuple_less_than_with_fuel(b, c, fuel),
        ensures tuple_less_than_with_fuel(a, c, fuel)
        decreases fuel
    {
        if fuel == 0 {
        } else if a.elements.len() == 0 {
            if c.elements.len() == 0 {
                tuple_less_than_with_fuel_antisymmetric(b, c, fuel);
            }
        } else if b.elements.len() == 0 {
            assert(false);
        } else if c.elements.len() == 0 {
            assert(false);
        } else {
            let ea = a.elements.first();
            let eb = b.elements.first();
            let ec = c.elements.first();
            let next_fuel = (fuel - 1) as nat;
            if element_less_than_with_fuel(eb, ec, next_fuel) {
                if element_less_than_with_fuel(eb, ea, next_fuel) {
                    assert(tuple_less_than_with_fuel(b, a, fuel));
                    assert(false);
                } else {
                    element_less_than_with_fuel_left_equivalent(ea, eb, ec, next_fuel);
                    assert(element_less_than_with_fuel(ea, ec, next_fuel));
                    assert(tuple_less_than_with_fuel(a, c, fuel));
                }
            } else if element_less_than_with_fuel(ec, eb, next_fuel) {
            } else {
                if element_less_than_with_fuel(eb, ea, next_fuel) {
                    assert(tuple_less_than_with_fuel(b, a, fuel));
                    assert(false);
                } else if element_less_than_with_fuel(ea, eb, next_fuel) {
                    element_less_than_with_fuel_right_equivalent(ea, eb, ec, next_fuel);
                    assert(element_less_than_with_fuel(ea, ec, next_fuel));
                    assert(tuple_less_than_with_fuel(a, c, fuel));
                } else {
                    assert(!tuple_less_than_with_fuel(
                        TupleSpec { elements: b.elements.skip(1) },
                        TupleSpec { elements: a.elements.skip(1) },
                        next_fuel,
                    ));
                    assert(tuple_less_than_with_fuel(
                        TupleSpec { elements: b.elements.skip(1) },
                        TupleSpec { elements: c.elements.skip(1) },
                        next_fuel,
                    ));
                    tuple_less_than_with_fuel_left_equivalent(
                        TupleSpec { elements: a.elements.skip(1) },
                        TupleSpec { elements: b.elements.skip(1) },
                        TupleSpec { elements: c.elements.skip(1) },
                        next_fuel,
                    );
                    assert(tuple_less_than_with_fuel(
                        TupleSpec { elements: a.elements.skip(1) },
                        TupleSpec { elements: c.elements.skip(1) },
                        next_fuel,
                    ));
                    if element_less_than_with_fuel(ea, ec, next_fuel) {
                        element_less_than_with_fuel_right_equivalent(ea, ec, eb, next_fuel);
                        assert(false);
                    }
                    if element_less_than_with_fuel(ec, ea, next_fuel) {
                        element_less_than_with_fuel_right_equivalent(ec, ea, eb, next_fuel);
                        assert(false);
                    }
                    assert(!element_less_than_with_fuel(ea, ec, next_fuel));
                    assert(!element_less_than_with_fuel(ec, ea, next_fuel));
                    assert(tuple_less_than_with_fuel(a, c, fuel));
                }
            }
        }
    }

    pub proof fn element_less_than_with_fuel_stable(
        a: ElementSpec,
        b: ElementSpec,
        fuel: nat,
    )
        requires fuel >= element_size(a) + element_size(b) + 1
        ensures element_less_than_with_fuel(a, b, fuel) == element_less_than(a, b)
        decreases fuel
    {
        if fuel == element_size(a) + element_size(b) + 1 {
            assert(element_less_than_with_fuel(a, b, fuel) == element_less_than(a, b));
        } else {
            match (a, b) {
                (ElementSpec::Tuple(ta), ElementSpec::Tuple(tb)) => {
                    assert((fuel - 1) as nat >= tuple_size(ta) + tuple_size(tb) + 1);
                    tuple_less_than_with_fuel_stable(ta, tb, (fuel - 1) as nat);
                    assert(element_size(a) + element_size(b) >= tuple_size(ta) + tuple_size(tb) + 1);
                    tuple_less_than_with_fuel_stable(ta, tb, element_size(a) + element_size(b));
                    assert(element_less_than_with_fuel(a, b, fuel) == element_less_than(a, b));
                },
                (_, _) => {
                    assert(element_less_than_with_fuel(a, b, fuel) == element_less_than(a, b));
                },
            }
        }
    }

    pub proof fn tuple_less_than_with_fuel_stable(
        a: TupleSpec,
        b: TupleSpec,
        fuel: nat,
    )
        requires fuel >= tuple_size(a) + tuple_size(b) + 1
        ensures tuple_less_than_with_fuel(a, b, fuel) == tuple_less_than(a, b)
        decreases fuel
    {
        if fuel == tuple_size(a) + tuple_size(b) + 1 {
            assert(tuple_less_than_with_fuel(a, b, fuel) == tuple_less_than(a, b));
        } else if a.elements.len() == 0 || b.elements.len() == 0 {
        } else {
            let ea = a.elements.first();
            let eb = b.elements.first();
            let next_fuel = (fuel - 1) as nat;
            assert(next_fuel >= element_size(ea) + element_size(eb) + 1);
            element_less_than_with_fuel_stable(ea, eb, next_fuel);
            element_less_than_with_fuel_stable(eb, ea, next_fuel);
            assert(tuple_size(a) + tuple_size(b) >= element_size(ea) + element_size(eb) + 1);
            element_less_than_with_fuel_stable(ea, eb, tuple_size(a) + tuple_size(b));
            element_less_than_with_fuel_stable(eb, ea, tuple_size(a) + tuple_size(b));
            assert(next_fuel >= tuple_size(TupleSpec { elements: a.elements.skip(1) })
                + tuple_size(TupleSpec { elements: b.elements.skip(1) }) + 1);
            tuple_less_than_with_fuel_stable(
                TupleSpec { elements: a.elements.skip(1) },
                TupleSpec { elements: b.elements.skip(1) },
                next_fuel,
            );
            assert(tuple_size(a) + tuple_size(b) >= tuple_size(TupleSpec { elements: a.elements.skip(1) })
                + tuple_size(TupleSpec { elements: b.elements.skip(1) }) + 1);
            tuple_less_than_with_fuel_stable(
                TupleSpec { elements: a.elements.skip(1) },
                TupleSpec { elements: b.elements.skip(1) },
                tuple_size(a) + tuple_size(b),
            );
            assert(tuple_less_than_with_fuel(a, b, fuel) == tuple_less_than(a, b));
        }
    }

    // ========================================================================
    // Lexicographic Ordering Axioms
    // ========================================================================
    //
    // The following proofs are marked as external_body because they establish
    // fundamental properties of lexicographic ordering that follow from the
    // mathematical definition of lexicographic comparison. These are trusted
    // axioms based on standard mathematical results (see ProofWiki:
    // "Lexicographic Order is Ordering").
    //
    // Axiom justification:
    // 1. Transitivity: If a < b and b < c, then at some position i either:
    //    - a[i] < b[i] and b[j] < c[j] for j <= i, implying a[k] < c[k] for some k
    //    - The induction follows the recursive structure of tuple_less_than
    //
    // 2. Anti-symmetry: If a < b, then at position i where they first differ,
    //    a[i] < b[i]. For b < a we'd need b[i] < a[i], contradicting a[i] < b[i].
    //
    // 3. Irreflexivity: For a < a, we'd need a[i] < a[i] for some i, which
    //    contradicts the irreflexivity of the underlying element comparison.

    /// Axiom: Tuple comparison is transitive
    ///
    /// If tuple a < tuple b and tuple b < tuple c, then tuple a < tuple c.
    /// This follows from the fuel-bounded lexicographic comparison lemma.
    pub proof fn axiom_tuple_comparison_transitive(
        a: TupleSpec,
        b: TupleSpec,
        c: TupleSpec,
    )
        requires
            tuple_less_than(a, b),
            tuple_less_than(b, c),
        ensures tuple_less_than(a, c)
    {
        let fuel = tuple_size(a) + tuple_size(b) + tuple_size(c) + 1;
        assert(fuel >= tuple_size(a) + tuple_size(b) + 1);
        assert(fuel >= tuple_size(b) + tuple_size(c) + 1);
        assert(fuel >= tuple_size(a) + tuple_size(c) + 1);
        tuple_less_than_with_fuel_stable(a, b, fuel);
        tuple_less_than_with_fuel_stable(b, c, fuel);
        tuple_less_than_with_fuel_transitive(a, b, c, fuel);
        tuple_less_than_with_fuel_stable(a, c, fuel);
    }

    /// Axiom: Tuple comparison is anti-symmetric
    ///
    /// If tuple a < tuple b, then it is NOT the case that tuple b < tuple a.
    /// This follows from the fuel-bounded lexicographic comparison lemma.
    pub proof fn axiom_tuple_comparison_antisymmetric(a: TupleSpec, b: TupleSpec)
        requires tuple_less_than(a, b)
        ensures !tuple_less_than(b, a)
    {
        tuple_less_than_with_fuel_antisymmetric(a, b, tuple_size(a) + tuple_size(b) + 1);
    }

    /// Axiom: Tuple comparison is irreflexive
    ///
    /// A tuple is never less than itself: NOT (a < a) for any tuple a.
    /// This follows from the fuel-bounded lexicographic comparison lemma.
    pub proof fn axiom_tuple_comparison_irreflexive(a: TupleSpec)
        ensures !tuple_less_than(a, a)
    {
        tuple_less_than_with_fuel_irreflexive(a, tuple_size(a) + tuple_size(a) + 1);
    }

    // ========================================================================
    // Backwards-compatible aliases
    // ========================================================================
    // Keep old names for API compatibility, delegating to axiom_ prefixed versions

    /// Proof: Tuple comparison is transitive (alias for axiom_tuple_comparison_transitive)
    pub proof fn tuple_comparison_transitive(
        a: TupleSpec,
        b: TupleSpec,
        c: TupleSpec,
    )
        requires
            tuple_less_than(a, b),
            tuple_less_than(b, c),
        ensures tuple_less_than(a, c)
    {
        axiom_tuple_comparison_transitive(a, b, c);
    }

    /// Proof: Tuple comparison is anti-symmetric (alias for axiom_tuple_comparison_antisymmetric)
    pub proof fn tuple_comparison_antisymmetric(a: TupleSpec, b: TupleSpec)
        requires tuple_less_than(a, b)
        ensures !tuple_less_than(b, a)
    {
        axiom_tuple_comparison_antisymmetric(a, b);
    }

    /// Proof: Tuple comparison is irreflexive (alias for axiom_tuple_comparison_irreflexive)
    pub proof fn tuple_comparison_irreflexive(a: TupleSpec)
        ensures !tuple_less_than(a, a)
    {
        axiom_tuple_comparison_irreflexive(a);
    }

    // ========================================================================
    // Integer Encoding Properties
    // ========================================================================

    /// Trusted encoding axiom: integer byte encoding preserves numeric order in
    /// the production tuple encoder. This depends on the uninterpreted `pack`
    /// function and is intentionally retained as an encoder boundary. Runtime
    /// evidence: `cargo test -p aspen-layer` exercises
    /// `test_trusted_tuple_boundary_runtime_evidence`, `prop_int_ordering`,
    /// and `prop_int_boundaries`.
    #[verifier(external_body)]
    pub proof fn int_encoding_preserves_order(a: i64, b: i64)
        requires a < b
        ensures ({
            let ta = TupleSpec { elements: seq![ElementSpec::Int(a)] };
            let tb = TupleSpec { elements: seq![ElementSpec::Int(b)] };
            seq_less_than(pack(ta), pack(tb))
        })
    {
        // The integer encoding uses:
        // - Negative: 0x13-0x14 prefix with inverted bytes
        // - Zero: 0x14 prefix
        // - Positive: 0x15-0x1C prefix with big-endian bytes
        // This ensures lexicographic order matches numeric order
    }

    /// Trusted encoding axiom: byte-array escaping and terminators preserve
    /// lexicographic order in the production tuple encoder. This depends on the
    /// uninterpreted `pack` function and is intentionally retained. Runtime
    /// evidence: `cargo test -p aspen-layer` exercises
    /// `test_trusted_tuple_boundary_runtime_evidence` and
    /// `prop_bytes_ordering`.
    #[verifier(external_body)]
    pub proof fn bytes_encoding_preserves_order(a: Seq<u8>, b: Seq<u8>)
        requires seq_less_than(a, b)
        ensures ({
            let ta = TupleSpec { elements: seq![ElementSpec::Bytes(a)] };
            let tb = TupleSpec { elements: seq![ElementSpec::Bytes(b)] };
            seq_less_than(pack(ta), pack(tb))
        })
    {
        // Bytes use 0x01 prefix + null-escaped content + 0x00 terminator
        // Null escaping (0x00 -> 0x00 0xFF) preserves lexicographic order
    }
}
