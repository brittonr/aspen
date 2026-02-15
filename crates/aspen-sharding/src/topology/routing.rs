//! Hex-based range boundary generation for initial shard distribution.

/// Generate hex-based range boundaries for initial shard distribution.
///
/// Distributes the keyspace evenly using hex character boundaries.
pub(super) fn generate_hex_boundaries(num_shards: u32) -> Vec<(String, String)> {
    if num_shards == 1 {
        return vec![("".to_string(), "".to_string())];
    }

    // For up to 16 shards, use single hex characters
    // For more, use two hex characters (256 buckets max)
    let hex_chars = if num_shards <= 16 {
        "0123456789abcdef"
    } else {
        "00010203040506070809101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"
    };

    // Simple approach: divide hex space evenly
    let boundaries: Vec<String> = if num_shards <= 16 {
        let step = 16 / num_shards;
        (0..num_shards)
            .map(|i| {
                if i == 0 {
                    "".to_string()
                } else {
                    let idx = (i * step) as usize;
                    hex_chars.chars().nth(idx).unwrap().to_string()
                }
            })
            .collect()
    } else {
        let step = 256 / num_shards;
        (0..num_shards)
            .map(|i| {
                if i == 0 {
                    "".to_string()
                } else {
                    let idx = (i * step) as usize * 2;
                    hex_chars[idx..idx + 2].to_string()
                }
            })
            .collect()
    };

    // Create range tuples
    let mut ranges = Vec::with_capacity(num_shards as usize);
    for i in 0..num_shards as usize {
        let start = boundaries[i].clone();
        let end = if i + 1 < boundaries.len() {
            boundaries[i + 1].clone()
        } else {
            "".to_string() // Last shard extends to end
        };
        ranges.push((start, end));
    }

    ranges
}
