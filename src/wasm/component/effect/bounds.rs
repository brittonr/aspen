// Inspect only packed framing before the upstream recursive decoder builds a
// value tree. Scalar depth is enough: every compound uses the same end tag.
pub(super) fn check(bytes: &[u8]) -> super::super::model::Result<()> {
    let length = u64::try_from(bytes.len()).map_err(|error| super::invalid(error.to_string()))?;
    if length > super::HOST_PAYLOAD_BYTES {
        return Err(limit("component effect carrier exceeds the admitted byte bound"));
    }
    let mut remaining = bytes;
    let mut depth = 0_u64;
    let mut nodes = 0_u64;
    while let Some((&byte, rest)) = remaining.split_first() {
        remaining = rest;
        let tag = preserves::packed::constants::Tag::try_from(byte)
            .map_err(|error| super::invalid(format!("invalid component effect tag: {error:?}")))?;
        if tag == preserves::packed::constants::Tag::End {
            depth = depth.checked_sub(1).ok_or_else(|| super::invalid("unexpected carrier end tag"))?;
        } else {
            if depth > super::HOST_MAX_DEPTH || nodes >= super::HOST_MAX_NODES {
                return Err(limit("component effect carrier exceeds its structural bounds"));
            }
            nodes += 1;
            consume(tag, &mut remaining, &mut depth)?;
        }
        if depth == 0 {
            return if remaining.is_empty() {
                Ok(())
            } else {
                Err(super::invalid("component effect carrier has trailing values"))
            };
        }
    }
    Err(super::invalid("component effect carrier is truncated"))
}

fn consume(
    tag: preserves::packed::constants::Tag,
    remaining: &mut &[u8],
    depth: &mut u64,
) -> super::super::model::Result<()> {
    match tag {
        preserves::packed::constants::Tag::False | preserves::packed::constants::Tag::True => Ok(()),
        preserves::packed::constants::Tag::Ieee754
        | preserves::packed::constants::Tag::SignedInteger
        | preserves::packed::constants::Tag::String
        | preserves::packed::constants::Tag::ByteString
        | preserves::packed::constants::Tag::Symbol => skip_body(remaining),
        preserves::packed::constants::Tag::Record
        | preserves::packed::constants::Tag::Sequence
        | preserves::packed::constants::Tag::Set
        | preserves::packed::constants::Tag::Dictionary => {
            *depth += 1;
            Ok(())
        }
        preserves::packed::constants::Tag::End
        | preserves::packed::constants::Tag::Annotation
        | preserves::packed::constants::Tag::Embedded => {
            Err(super::invalid("component effect carrier requires unannotated, nonembedded values"))
        }
    }
}

fn skip_body(remaining: &mut &[u8]) -> super::super::model::Result<()> {
    let mut length = 0_usize;
    // At most three varint bytes can describe an atom inside a 65,536-byte
    // carrier. Longer encodings are oversized or noncanonical, never admitted.
    for shift in [0, 7, 14] {
        let (&byte, rest) = remaining.split_first().ok_or_else(|| super::invalid("truncated atom length"))?;
        *remaining = rest;
        length |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            *remaining =
                remaining.get(length..).ok_or_else(|| super::invalid("atom exceeds remaining carrier bytes"))?;
            return Ok(());
        }
    }
    Err(super::invalid("atom length is oversized or noncanonical"))
}

fn limit(message: &str) -> super::super::model::Denial {
    super::super::model::Denial::classified(super::super::model::DenialClass::ResourceDenial, message)
}

#[cfg(test)]
mod tests {
    // r[verify molten.wasm_component_adoption.hostcalls]
    #[test]
    fn byte_ceiling_is_inclusive() {
        for (length, is_admitted) in [(65_532, true), (65_533, false)] {
            let value = crate::preserves_rail::string("x".repeat(length));
            let bytes = crate::preserves_rail::canonical_bytes(&value).expect("canonical large string");
            assert_eq!(bytes.len(), length + 4);
            assert_eq!(super::check(&bytes).is_ok(), is_admitted);
        }
    }

    // r[verify molten.wasm_component_adoption.hostcalls]
    #[test]
    fn nesting_ceiling_precedes_recursive_decoding() {
        for (depth, is_admitted) in [(32, true), (33, false)] {
            let mut value = crate::preserves_rail::u64_value(0);
            for _ in 0..depth {
                value = crate::preserves_rail::sequence(vec![value]);
            }
            let bytes = crate::preserves_rail::canonical_bytes(&value).expect("canonical nested value");
            assert_eq!(super::check(&bytes).is_ok(), is_admitted);
        }
    }

    // r[verify molten.wasm_component_adoption.hostcalls]
    #[test]
    fn node_ceiling_counts_root_and_children() {
        for (children, is_admitted) in [(4_095, true), (4_096, false)] {
            let value = crate::preserves_rail::sequence((0..children).map(crate::preserves_rail::u64_value).collect());
            let bytes = crate::preserves_rail::canonical_bytes(&value).expect("canonical wide value");
            assert_eq!(super::check(&bytes).is_ok(), is_admitted);
        }
    }

    // r[verify molten.wasm_component_adoption.hostcalls]
    #[test]
    fn annotations_and_embedded_values_cannot_hide_recursive_work() {
        // Preserves interoperability tags: annotation, embedded, and false.
        for bytes in [&[0x85, 0x80, 0x80][..], &[0x86, 0x80][..]] {
            assert!(super::check(bytes).is_err());
        }
    }

    // r[verify molten.wasm_component_adoption.hostcalls]
    #[test]
    fn oversized_atom_lengths_return_denial_instead_of_panicking() {
        for tag in [0x87, 0xb0, 0xb1, 0xb2, 0xb3] {
            let bytes = [tag, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01];
            assert!(super::super::decode_host_request(&bytes).is_err());
        }
    }
}
