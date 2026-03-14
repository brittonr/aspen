//! Pure functions for PSI (Pressure Stall Information) parsing.
//!
//! Formally verified — see `verus/pressure_spec.rs` for proofs.

/// Parse the avg10 value from a /proc/pressure/* file's content.
///
/// Format: `some avg10=0.00 avg60=0.00 avg300=0.00 total=0`
/// or:     `full avg10=0.00 avg60=0.00 avg300=0.00 total=0`
///
/// Parses the first `avg10=` value found (from the `some` line).
/// Returns 0.0 on parse failure or empty input.
#[inline]
pub fn parse_psi_avg10(content: &str) -> f32 {
    // Find "avg10=" in the first line (the "some" line)
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("some ") {
            return extract_avg10_from_line(rest);
        }
    }
    // Fallback: try the "full" line
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("full ") {
            return extract_avg10_from_line(rest);
        }
    }
    0.0
}

/// Extract avg10 value from a PSI line after the prefix.
/// Input format: `avg10=X.XX avg60=Y.YY avg300=Z.ZZ total=N`
fn extract_avg10_from_line(line: &str) -> f32 {
    for part in line.split_whitespace() {
        if let Some(val) = part.strip_prefix("avg10=") {
            return val.parse::<f32>().unwrap_or(0.0);
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_psi_avg10_normal() {
        let content =
            "some avg10=1.50 avg60=2.10 avg300=0.85 total=123456\nfull avg10=0.50 avg60=1.00 avg300=0.30 total=654321";
        assert!((parse_psi_avg10(content) - 1.50).abs() < 0.001);
    }

    #[test]
    fn test_parse_psi_avg10_only_full() {
        let content = "full avg10=2.75 avg60=1.00 avg300=0.30 total=654321";
        assert!((parse_psi_avg10(content) - 2.75).abs() < 0.001);
    }

    #[test]
    fn test_parse_psi_avg10_empty() {
        assert_eq!(parse_psi_avg10(""), 0.0);
    }

    #[test]
    fn test_parse_psi_avg10_garbage() {
        let content = "garbage input with no avg10 value";
        assert_eq!(parse_psi_avg10(content), 0.0);
    }

    #[test]
    fn test_parse_psi_avg10_missing_avg10() {
        let content = "some avg60=2.10 avg300=0.85 total=123456";
        assert_eq!(parse_psi_avg10(content), 0.0);
    }

    #[test]
    fn test_parse_psi_avg10_zero_values() {
        let content = "some avg10=0.00 avg60=0.00 avg300=0.00 total=0";
        assert_eq!(parse_psi_avg10(content), 0.0);
    }

    #[test]
    fn test_extract_avg10_from_line() {
        assert!((extract_avg10_from_line("avg10=1.50 avg60=2.10") - 1.50).abs() < 0.001);
        assert_eq!(extract_avg10_from_line("avg60=2.10 avg300=0.85"), 0.0);
        assert_eq!(extract_avg10_from_line(""), 0.0);
        assert_eq!(extract_avg10_from_line("avg10=invalid"), 0.0);
    }
}
