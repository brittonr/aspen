use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;
use crate::prolly_map::NodeRef;

// r[verify aspen.marble_store.art_index]
#[test]
fn a_canonical_node_ref_round_trips_through_its_fixed_length_digest() {
    let expected = digest("leaf");
    let node_ref = node_ref_of_digest(&expected);
    assert_eq!(digest_of_node_ref(&node_ref), Ok(expected));
    assert_eq!(node_ref_of_digest(&digest_of_node_ref(&node_ref).expect("digest")), node_ref);
    let hex = node_ref.as_str().trim_start_matches("blake3:").to_owned();
    for invalid in [
        format!("blake3:{}", hex.to_uppercase()),
        format!("blake3:{}", &hex[1..]),
        format!("sha256:{hex}"),
        format!("blake3:{hex}0"),
        String::new(),
    ] {
        assert_eq!(digest_of_node_ref(&NodeRef::new(invalid)), Err(MarbleStoreIssue::InvalidNodeRef));
    }
}

#[test]
fn a_block_frame_round_trips_and_malformed_frames_are_refused() {
    let digest = digest("block");
    let frame = encode_block_frame(&digest, b"payload", MAX_OBJECT).expect("frame");
    assert_eq!(
        decode_block_frame(&frame, MAX_OBJECT),
        Ok(BlockFrame {
            digest,
            payload: b"payload"
        })
    );
    assert_eq!(decode_block_frame(&frame[..20], MAX_OBJECT), Err(MarbleStoreIssue::TruncatedFrame));
    assert_eq!(decode_block_frame(&[], MAX_OBJECT), Err(MarbleStoreIssue::TruncatedFrame));
    let mut unknown = frame.clone();
    unknown[0] = 9;
    assert_eq!(decode_block_frame(&unknown, MAX_OBJECT), Err(MarbleStoreIssue::UnknownFrameVersion(9)));
    let oversized = vec![0_u8; usize::try_from(MAX_OBJECT).expect("bound")];
    assert_eq!(encode_block_frame(&digest, &oversized, MAX_OBJECT), Err(MarbleStoreIssue::PayloadTooLarge));
    assert_eq!(decode_block_frame(&frame, 8), Err(MarbleStoreIssue::PayloadTooLarge));
}

fn catalog() -> MarbleCatalog {
    MarbleCatalog {
        heads: BTreeMap::from([
            ("alpha".to_owned(), b"head-a".to_vec()),
            ("beta".to_owned(), b"head-b".to_vec()),
        ]),
        pins: BTreeSet::from([digest("pin-1"), digest("pin-2")]),
        retention_generation: 7,
    }
}

#[test]
fn a_catalog_round_trips_and_non_canonical_encodings_are_refused() {
    let encoded = encode_catalog(&catalog()).expect("encode");
    assert_eq!(decode_catalog(&encoded), Ok(catalog()));
    assert_eq!(
        decode_catalog(&encode_catalog(&MarbleCatalog::default()).expect("empty")),
        Ok(MarbleCatalog::default())
    );

    let mut trailing = encoded.clone();
    trailing.push(0);
    assert_eq!(decode_catalog(&trailing), Err(MarbleStoreIssue::InvalidCatalog("trailing-bytes")));
    assert_eq!(decode_catalog(&encoded[..encoded.len() - 1]), Err(MarbleStoreIssue::InvalidCatalog("truncated")));

    let pins_start = encoded.len() - 2 * 32;
    let mut reordered = encoded[..pins_start].to_vec();
    reordered.extend_from_slice(&encoded[pins_start + 32..]);
    reordered.extend_from_slice(&encoded[pins_start..pins_start + 32]);
    assert_eq!(decode_catalog(&reordered), Err(MarbleStoreIssue::InvalidCatalog("pin-order")));

    let mut oversized = catalog();
    oversized.heads.insert("x".repeat(97), Vec::new());
    assert_eq!(encode_catalog(&oversized), Err(MarbleStoreIssue::InvalidCatalog("map-id-length")));
}
