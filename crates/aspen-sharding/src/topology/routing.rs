//! Hex-based range boundary generation for initial shard distribution.

const SINGLE_HEX_DIGITS: &str = "0123456789abcdef";
const DOUBLE_HEX_BOUNDARIES: &str =
    "00010203040506070809101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff";
const SINGLE_DIGIT_SHARD_LIMIT: u32 = 16;
const SINGLE_DIGIT_BUCKET_COUNT: u32 = 16;
const DOUBLE_DIGIT_BUCKET_COUNT: u32 = 256;
const HEX_PAIR_WIDTH_BYTES: usize = 2;

#[inline]
fn single_digit_boundary(boundary_index: u32) -> String {
    let mut current_index = 0_u32;
    for digit in SINGLE_HEX_DIGITS.chars() {
        if current_index == boundary_index {
            return digit.to_string();
        }
        current_index = current_index.saturating_add(1);
    }
    String::new()
}

#[inline]
fn double_digit_boundary(boundary_index: u32) -> String {
    let mut current_index = 0_u32;
    for hex_pair_bytes in DOUBLE_HEX_BOUNDARIES.as_bytes().chunks(HEX_PAIR_WIDTH_BYTES) {
        if current_index == boundary_index {
            return String::from_utf8_lossy(hex_pair_bytes).into_owned();
        }
        current_index = current_index.saturating_add(1);
    }
    String::new()
}

#[inline]
fn generate_single_digit_boundaries(num_shards: u32) -> Vec<String> {
    let Some(step) = SINGLE_DIGIT_BUCKET_COUNT.checked_div(num_shards) else {
        return Vec::new();
    };
    (0..num_shards)
        .map(|shard_index| {
            if shard_index == 0 {
                String::new()
            } else {
                single_digit_boundary(shard_index.saturating_mul(step))
            }
        })
        .collect()
}

#[inline]
fn generate_double_digit_boundaries(num_shards: u32) -> Vec<String> {
    let Some(step) = DOUBLE_DIGIT_BUCKET_COUNT.checked_div(num_shards) else {
        return Vec::new();
    };
    (0..num_shards)
        .map(|shard_index| {
            if shard_index == 0 {
                String::new()
            } else {
                double_digit_boundary(shard_index.saturating_mul(step))
            }
        })
        .collect()
}

#[inline]
fn build_ranges(boundaries: &[String]) -> Vec<(String, String)> {
    let mut ranges = Vec::with_capacity(boundaries.len());
    for (shard_index, start) in boundaries.iter().enumerate() {
        let end = if let Some(end_boundary) = boundaries.get(shard_index.saturating_add(1)) {
            end_boundary.clone()
        } else {
            String::new()
        };
        ranges.push((start.clone(), end));
    }
    ranges
}

/// Generate hex-based range boundaries for initial shard distribution.
///
/// Distributes the keyspace evenly using hex character boundaries.
pub(super) fn generate_hex_boundaries(num_shards: u32) -> Vec<(String, String)> {
    if num_shards == 1 {
        return vec![(String::new(), String::new())];
    }

    let boundaries = if num_shards <= SINGLE_DIGIT_SHARD_LIMIT {
        generate_single_digit_boundaries(num_shards)
    } else {
        generate_double_digit_boundaries(num_shards)
    };

    build_ranges(&boundaries)
}
