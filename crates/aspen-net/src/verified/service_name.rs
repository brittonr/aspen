//! Pure service name validation.
//!
//! Formally verified — see `verus/service_name_spec.rs` for proofs.

/// Validate that a service name conforms to mesh naming rules.
///
/// Valid names: `^[a-z0-9][a-z0-9.-]{0,252}$`
/// - Starts with lowercase letter or digit
/// - Contains only lowercase letters, digits, hyphens, dots
/// - 1 to 253 characters long
#[inline]
pub fn is_valid_service_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 253 {
        return false;
    }

    let bytes = name.as_bytes();

    // First character must be [a-z0-9]
    let first = bytes[0];
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return false;
    }

    // All characters must be [a-z0-9.-]
    for &b in bytes {
        if !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.') {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        assert!(is_valid_service_name("mydb"));
        assert!(is_valid_service_name("web-frontend"));
        assert!(is_valid_service_name("my.service.name"));
        assert!(is_valid_service_name("a"));
        assert!(is_valid_service_name("0start"));
        assert!(is_valid_service_name("prod.myapp-web"));
    }

    #[test]
    fn invalid_names() {
        assert!(!is_valid_service_name(""));
        assert!(!is_valid_service_name("-starts-with-dash"));
        assert!(!is_valid_service_name(".starts-with-dot"));
        assert!(!is_valid_service_name("UPPERCASE"));
        assert!(!is_valid_service_name("has space"));
        assert!(!is_valid_service_name("has_underscore"));
        assert!(!is_valid_service_name("special!char"));
        // Too long
        let long_name = "a".repeat(254);
        assert!(!is_valid_service_name(&long_name));
    }

    #[test]
    fn boundary_length() {
        // Exactly 253 chars is valid
        let max_name = "a".repeat(253);
        assert!(is_valid_service_name(&max_name));
        // 254 chars is invalid
        let over_name = "a".repeat(254);
        assert!(!is_valid_service_name(&over_name));
    }
}
