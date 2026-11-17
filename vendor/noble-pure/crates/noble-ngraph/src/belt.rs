//! Bounded, checked sequential-belt backend for the total `I64`/`Bool` fragment.
//!
//! The lowerer is untrusted. [`check`] independently validates the encoded
//! instruction stream against the checked graph, and [`execute_checked`]
//! rechecks artifact binding before interpreting it. This is finite Rust
//! implementation evidence only; the Lean model and source-to-artifact
//! correspondence remain separate claims.

mod codec;
mod execution;
#[path = "belt/lower.rs"]
mod lowering;
mod support;
pub mod target;
mod types;
mod validate;

pub use codec::{decode, encode, MAX_ARTIFACT_BYTES};
pub use execution::{execute, execute_checked};
pub use lowering::lower;
pub use types::{ArtifactView, CheckedArtifact, Lowered, Refusal};

/// Artifact format and semantic revision.
pub const FORMAT_REVISION: &str = "ngraph-belt/0";
/// Revision of the canonical graph-to-belt lowerer.
pub const LOWERER_REVISION: &str = "ngraph-belt-lowerer/0";
/// Revision of the independent belt checker.
pub const CHECKER_REVISION: &str = "ngraph-belt-checker/0";
/// Smallest belt that can hold both operands of a binary operation.
pub const MIN_CAPACITY: u8 = 2;
/// Nominal belt capacity; callers may choose another admitted capacity per artifact.
pub const NOMINAL_CAPACITY: u8 = 8;
/// Maximum age-addressable belt capacity.
pub const MAX_CAPACITY: u8 = 64;
/// Maximum number of explicit preservation slots in one artifact.
pub const MAX_SAVE_SLOTS: u8 = 8;
/// Maximum number of graph nodes admitted by this backend.
pub const MAX_NODES: usize = 4_096;
/// Maximum number of ordered graph outputs admitted by this backend.
pub const MAX_OUTPUTS: usize = 4_096;
/// Maximum encoded instruction count, including saves, reloads and output reads.
pub const MAX_INSTRUCTIONS: usize = 65_536;

/// Independently validate an encoded belt artifact against `checked`.
///
/// The returned token borrows the exact bytes checked. Its private fields
/// prevent construction without running the checker.
pub fn check<'a>(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    bytes: &'a [u8],
) -> Result<CheckedArtifact<'a>, Refusal> {
    validate::check(ids, checked, bytes)
}

#[cfg(test)]
mod tests {
    #![allow(
        tigerstyle::assertion_density,
        tigerstyle::no_panic,
        tigerstyle::no_unwrap,
        reason = "Owner: noble-maintainers; this internal integrity test forges the backing bytes while retaining a checker-issued digest snapshot, a state safe callers cannot construct."
    )]

    use super::*;

    struct TestIds;

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

    impl crate::identity::IdentityScheme for TestIds {
        fn name(&self) -> &'static str {
            "belt-test"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(TestHasher(0))
        }
    }

    #[test]
    fn execute_checked_rejects_rebound_bytes_after_checking() {
        let ids = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&ids, "belt-post-check-mutation"),
        );
        let input = builder.input(crate::ir::ScalarType::I64);
        let one = builder.i64_const(1);
        let result = builder.op(crate::ir::Op::I64Add(input, one));
        builder.output(result);
        let checked =
            crate::check::check(&ids, builder.finish()).expect("the small I64 graph checks");
        let lowered = lower(&ids, &checked, 2, 1).expect("the graph lowers");
        let accepted = check(&ids, &checked, &lowered.bytes)
            .expect("the lowerer's bytes pass the independent checker");
        let view = decode(&ids, &lowered.bytes).expect("fresh bytes decode");
        let mut changed_body = view.instruction_bytes().to_vec();
        changed_body[6] ^= 1;
        let rebound = encode(
            &ids,
            &checked,
            accepted.belt_slots,
            accepted.save_slots,
            &changed_body,
        )
        .expect("the altered body has a valid structural encoding");
        let changed_token = CheckedArtifact {
            bytes: &rebound,
            graph_id: accepted.graph_id,
            belt_slots: accepted.belt_slots,
            save_slots: accepted.save_slots,
            instruction_digest: accepted.instruction_digest,
            artifact_binding: accepted.artifact_binding,
            instruction_count: accepted.instruction_count,
        };

        assert_eq!(
            execute_checked(&ids, &checked, changed_token, &[crate::sem::Value::I64(1)],),
            Err(Refusal::InstructionDigest)
        );
    }
}
