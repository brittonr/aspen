use super::percentile_stats::PercentileStats;

/// A histogram for tracking the distribution of u64 values using logarithmic bucketing.
///
/// Uses a logarithmic bucketing strategy where smaller values get higher precision.
/// The bucketing algorithm is based on the binary representation of the value:
///
/// - Group 0 (special): [0, 1, 2, 3]
/// - Group 1: [4, 5, 6, 7]
/// - Group 2: [8, 10, 12, 14]
/// - Group 3: [16, 20, 24, 28]
/// - Group 4: [32, 40, 48, 56]
/// - And so on...
///
/// Each group (except group 0) contains 4 buckets determined by the 2 bits
/// after the most significant bit.
///
/// The histogram uses exactly 252 buckets to cover all possible u64 values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Histogram {
    buckets: Vec<u64>,
    bucket_min_values: Vec<u64>,
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

impl Histogram {
    /// The width of the bit pattern used for bucketing (most significant bits).
    ///
    /// Each bucket group uses 3 bits: 1 MSB + 2 offset bits.
    const WIDTH: usize = 3;

    /// Number of offset bits after the group MSB.
    const OFFSET_BIT_COUNT: usize = 2;

    /// Number of exact buckets in group 0.
    const GROUP_ZERO_BUCKET_COUNT: usize = 4;

    /// Bitmask for extracting the offset within a bucket group.
    const OFFSET_MASK: u64 = 0b11;

    /// The MSB bit pattern for bucket groups.
    ///
    /// Sets the most significant bit to 1: 1 << OFFSET_BIT_COUNT = 0b100
    const GROUP_MSB_BIT: usize = 1 << Self::OFFSET_BIT_COUNT;

    /// Number of buckets per group.
    ///
    /// Each group contains GROUP_MSB_BIT buckets.
    /// For WIDTH=3: GROUP_MSB_BIT = 4 buckets per group.
    const GROUP_SIZE: usize = Self::GROUP_MSB_BIT;

    /// Mask for extracting the offset within a bucket group.
    const MASK: u64 = Self::OFFSET_MASK;

    /// The exact number of buckets needed to cover all u64 values with logarithmic precision.
    ///
    /// For WIDTH=3 this is 252 buckets, which equals `bucket_index(u64::MAX) + 1`.
    const BUCKETS_FOR_U64: usize = 252;

    /// Creates a new histogram with 252 buckets, covering all u64 values.
    ///
    /// Memory usage: 252 * 8 bytes = 2,016 bytes per histogram.
    pub(crate) fn new() -> Self {
        let mut bucket_min_values = vec![0u64; Self::BUCKETS_FOR_U64];
        #[allow(clippy::needless_range_loop)]
        for i in 0..Self::BUCKETS_FOR_U64 {
            bucket_min_values[i] = Self::bucket_min_value_for_index(i);
        }

        Self {
            buckets: vec![0; Self::BUCKETS_FOR_U64],
            bucket_min_values,
        }
    }

    /// Records a value to the histogram.
    pub(crate) fn record(&mut self, value: u64) {
        let bucket_index = Self::calculate_bucket(value);
        self.buckets[bucket_index] += 1;
    }

    /// Calculates the bucket index for a given value using logarithmic bucketing.
    ///
    /// Algorithm:
    /// 1. For value < GROUP_SIZE: bucket_index = value
    /// 2. For value >= GROUP_SIZE:
    ///    - Find the position of the most significant bit (MSB)
    ///    - Determine which group of GROUP_SIZE buckets (group 0 has buckets 0-3, group 1 has 4-7,
    ///      etc.)
    ///    - Extract offset within that group using the 2 bits after MSB
    ///    - Bucket index = base of this group + offset within group
    fn calculate_bucket(value: u64) -> usize {
        if value < saturating_u64_from_usize(Self::GROUP_SIZE) {
            return saturating_usize_from_u64(value);
        }

        let bits_upto_msb = saturating_usize_from_u32(u64::BITS.saturating_sub(value.leading_zeros()));
        let group_index = bits_upto_msb.saturating_sub(Self::WIDTH);
        let bucket_slot_index = saturating_usize_from_u64((value >> group_index) & Self::MASK);

        let buckets_before_this_group = Self::GROUP_SIZE.saturating_add(group_index.saturating_mul(Self::GROUP_SIZE));
        buckets_before_this_group.saturating_add(bucket_slot_index)
    }

    fn bucket_min_value_for_index(bucket_index: usize) -> u64 {
        if bucket_index < Self::GROUP_ZERO_BUCKET_COUNT {
            return saturating_u64_from_usize(bucket_index);
        }

        let relative_bucket_index = bucket_index.saturating_sub(Self::GROUP_ZERO_BUCKET_COUNT);
        let group_index = split_relative_bucket_group_index(relative_bucket_index);
        let consumed_bucket_count = group_index.saturating_mul(Self::GROUP_SIZE);
        let bucket_slot_index = relative_bucket_index.saturating_sub(consumed_bucket_count);
        let bucket_pattern = bucket_slot_index | Self::GROUP_MSB_BIT;
        let shift_bits = saturating_u32_from_usize(group_index);
        let shifted_bucket_pattern = match bucket_pattern.checked_shl(shift_bits) {
            Some(shifted_bucket_pattern) => shifted_bucket_pattern,
            None => usize::MAX,
        };
        saturating_u64_from_usize(shifted_bucket_pattern)
    }

    /// Returns the total number of values recorded.
    #[allow(dead_code)]
    pub(crate) fn total(&self) -> u64 {
        self.buckets.iter().sum()
    }

    /// Calculates the value at the given percentile.
    ///
    /// Returns the minimum value of the bucket containing the percentile.
    /// For example, `percentile(0.5)` returns P50 (median), `percentile(0.99)` returns P99.
    ///
    /// Returns `0` if the histogram is empty.
    #[allow(dead_code)]
    pub(crate) fn percentile(&self, p: f64) -> u64 {
        let total = self.total();
        self.percentile_with_total(p, total)
    }

    /// Calculates the percentile given a specific total count.
    ///
    /// This is used internally when calculating multiple percentiles to avoid
    /// recalculating the total multiple times.
    #[allow(dead_code)]
    fn percentile_with_total(&self, p: f64, total: u64) -> u64 {
        let target = (total as f64 * p).ceil().max(1.0) as u64;
        let mut cumulative = 0u64;

        for (bucket_index, &count) in self.buckets.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return self.bucket_min_values[bucket_index];
            }
        }

        0
    }

    /// Returns common percentile statistics: P50, P90, P99.
    #[allow(dead_code)]
    pub(crate) fn percentile_stats(&self) -> PercentileStats {
        let total = self.total();
        PercentileStats {
            p50: self.percentile_with_total(0.50, total),
            p90: self.percentile_with_total(0.90, total),
            p99: self.percentile_with_total(0.99, total),
        }
    }

    #[cfg(test)]
    pub(crate) fn get_bucket(&self, index: usize) -> u64 {
        self.buckets.get(index).copied().unwrap_or(0)
    }

    #[cfg(test)]
    pub(crate) fn num_buckets(&self) -> usize {
        self.buckets.len()
    }
}

fn split_relative_bucket_group_index(relative_bucket_index: usize) -> usize {
    let group_size = Histogram::GROUP_SIZE;
    if group_size == 0 {
        return 0;
    }

    relative_bucket_index / group_size
}

fn saturating_u32_from_usize(value: usize) -> u32 {
    match u32::try_from(value) {
        Ok(converted_value) => converted_value,
        Err(_) => u32::MAX,
    }
}

fn saturating_u64_from_usize(value: usize) -> u64 {
    match u64::try_from(value) {
        Ok(converted_value) => converted_value,
        Err(_) => u64::MAX,
    }
}

fn saturating_usize_from_u32(value: u32) -> usize {
    match usize::try_from(value) {
        Ok(converted_value) => converted_value,
        Err(_) => usize::MAX,
    }
}

fn saturating_usize_from_u64(value: u64) -> usize {
    match usize::try_from(value) {
        Ok(converted_value) => converted_value,
        Err(_) => usize::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bucket_group_0() {
        assert_eq!(Histogram::calculate_bucket(0), 0);
        assert_eq!(Histogram::calculate_bucket(1), 1);
        assert_eq!(Histogram::calculate_bucket(2), 2);
        assert_eq!(Histogram::calculate_bucket(3), 3);
    }

    #[test]
    fn test_calculate_bucket_group_1() {
        assert_eq!(Histogram::calculate_bucket(4), 4);
        assert_eq!(Histogram::calculate_bucket(5), 5);
        assert_eq!(Histogram::calculate_bucket(6), 6);
        assert_eq!(Histogram::calculate_bucket(7), 7);
    }

    #[test]
    fn test_calculate_bucket_group_2() {
        assert_eq!(Histogram::calculate_bucket(8), 8);
        assert_eq!(Histogram::calculate_bucket(10), 9);
        assert_eq!(Histogram::calculate_bucket(12), 10);
        assert_eq!(Histogram::calculate_bucket(14), 11);
    }

    #[test]
    fn test_calculate_bucket_group_3() {
        assert_eq!(Histogram::calculate_bucket(16), 12);
        assert_eq!(Histogram::calculate_bucket(20), 13);
        assert_eq!(Histogram::calculate_bucket(24), 14);
        assert_eq!(Histogram::calculate_bucket(28), 15);
    }

    #[test]
    fn test_calculate_bucket_group_4() {
        assert_eq!(Histogram::calculate_bucket(32), 16);
        assert_eq!(Histogram::calculate_bucket(40), 17);
        assert_eq!(Histogram::calculate_bucket(48), 18);
        assert_eq!(Histogram::calculate_bucket(56), 19);
    }

    #[test]
    fn test_record_and_total() {
        let mut hist = Histogram::new();

        hist.record(1);
        hist.record(5);
        hist.record(10);
        hist.record(100);

        assert_eq!(hist.total(), 4);
        assert_eq!(hist.get_bucket(1), 1);
        assert_eq!(hist.get_bucket(5), 1);
        assert_eq!(hist.get_bucket(Histogram::calculate_bucket(10)), 1);
        assert_eq!(hist.get_bucket(Histogram::calculate_bucket(100)), 1);
    }

    #[test]
    fn test_record_same_bucket() {
        let mut hist = Histogram::new();

        hist.record(8);
        hist.record(8);
        hist.record(8);

        assert_eq!(hist.total(), 3);
        assert_eq!(hist.get_bucket(8), 3);
    }

    #[test]
    fn test_u64_max_coverage() {
        let max_bucket = Histogram::calculate_bucket(u64::MAX);
        assert_eq!(max_bucket, 251, "u64::MAX should map to bucket 251");
        assert_eq!(Histogram::BUCKETS_FOR_U64, 252, "Should need exactly 252 buckets");

        // Verify new() creates enough buckets to record u64::MAX
        let mut hist = Histogram::new();
        assert_eq!(hist.num_buckets(), 252);
        hist.record(u64::MAX);
        assert_eq!(hist.get_bucket(251), 1);
        assert_eq!(hist.total(), 1);
    }

    #[test]
    fn test_reasonable_bucket_ranges() {
        assert_eq!(Histogram::calculate_bucket(1024), 36);

        let million = 1_048_576;
        let million_bucket = Histogram::calculate_bucket(million);
        assert!(million_bucket < 80);

        let billion = 1_073_741_824;
        let billion_bucket = Histogram::calculate_bucket(billion);
        assert!(billion_bucket < 120);
    }

    #[test]
    fn test_bucket_min_values_lookup_table() {
        let hist = Histogram::new();

        // Group 0: [0, 1, 2, 3]
        assert_eq!(hist.bucket_min_values[0], 0);
        assert_eq!(hist.bucket_min_values[1], 1);
        assert_eq!(hist.bucket_min_values[2], 2);
        assert_eq!(hist.bucket_min_values[3], 3);

        // Group 1: [4, 5, 6, 7]
        assert_eq!(hist.bucket_min_values[4], 4);
        assert_eq!(hist.bucket_min_values[5], 5);
        assert_eq!(hist.bucket_min_values[6], 6);
        assert_eq!(hist.bucket_min_values[7], 7);

        // Group 2: [8, 10, 12, 14]
        assert_eq!(hist.bucket_min_values[8], 8);
        assert_eq!(hist.bucket_min_values[9], 10);
        assert_eq!(hist.bucket_min_values[10], 12);
        assert_eq!(hist.bucket_min_values[11], 14);

        // Group 3: [16, 20, 24, 28]
        assert_eq!(hist.bucket_min_values[12], 16);
        assert_eq!(hist.bucket_min_values[13], 20);
        assert_eq!(hist.bucket_min_values[14], 24);
        assert_eq!(hist.bucket_min_values[15], 28);
    }

    #[test]
    fn test_percentile_empty() {
        let hist = Histogram::new();
        assert_eq!(hist.percentile(0.5), 0);
        assert_eq!(hist.percentile_stats(), PercentileStats { p50: 0, p90: 0, p99: 0 });
    }

    #[test]
    fn test_percentile_single_value() {
        let mut hist = Histogram::new();
        hist.record(10);

        assert_eq!(hist.percentile(0.0), 10);
        assert_eq!(hist.percentile(0.5), 10);
        assert_eq!(hist.percentile(0.99), 10);
        assert_eq!(hist.percentile(1.0), 10);
    }

    #[test]
    fn test_percentile_multiple_values() {
        let mut hist = Histogram::new();

        // Record 100 values: 1-10 each recorded 10 times
        for value in 1..=10 {
            for _ in 0..10 {
                hist.record(value);
            }
        }

        assert_eq!(hist.total(), 100);

        // P50 should be around value 5-6 (bucket returns min value)
        let p50 = hist.percentile(0.5);
        assert!((4..=6).contains(&p50), "P50 = {}", p50);

        // P90 should be around value 9 (bucket 8 contains [8,9])
        let p90 = hist.percentile(0.9);
        assert!((8..=10).contains(&p90), "P90 = {}", p90);

        // P99 should be around value 10
        let p99 = hist.percentile(0.99);
        assert!((8..=10).contains(&p99), "P99 = {}", p99);
    }

    #[test]
    fn test_percentile_stats() {
        let mut hist = Histogram::new();

        for i in 1..=100 {
            hist.record(i);
        }

        let stats = hist.percentile_stats();

        // Due to logarithmic bucketing, values are grouped
        // P50 around 50, bucket min value might be 48
        assert!(stats.p50 >= 48 && stats.p50 <= 52, "P50 = {}", stats.p50);
        // P90 around 90, bucket min value might be 80
        assert!(stats.p90 >= 80 && stats.p90 <= 92, "P90 = {}", stats.p90);
        // P99 around 99, bucket min value might be 96
        assert!(stats.p99 >= 96 && stats.p99 <= 100, "P99 = {}", stats.p99);
    }

    #[test]
    fn test_percentile_large_values() {
        let mut hist = Histogram::new();

        // Record exponentially distributed values
        hist.record(1);
        hist.record(10);
        hist.record(100);
        hist.record(1000);
        hist.record(10000);

        assert_eq!(hist.total(), 5);

        // P50 (median) should be the 3rd value (100), but bucket returns min value (96)
        let p50 = hist.percentile(0.5);
        assert!((96..=100).contains(&p50), "P50 = {}", p50);

        // P80 should be the 4th value (1000), but bucket returns min value
        let p80 = hist.percentile(0.8);
        assert!((896..=1000).contains(&p80), "P80 = {}", p80);
    }
}
