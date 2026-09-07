/// Recognize the existing remote/content locator syntax; this grants no authority.
pub(crate) fn is_remote(value: &str) -> bool {
    value.contains("://") || ["iroh:", "http:", "https:", "blake3:"].iter().any(|prefix| value.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    #[test]
    fn recognition_matches_the_previous_guards() {
        for lead in ["", "relative/", " "] {
            for scheme in ["iroh", "http", "https", "blake3", "ssh", "custom", "HTTP", "", "a"] {
                for suffix in ["", ":", ":/", "://host", ":value", "::value"] {
                    let value = format!("{lead}{scheme}{suffix}");
                    // Independent explicit reference to the previous two call-site guards.
                    let previous = value.contains("://")
                        || value.starts_with("iroh:")
                        || value.starts_with("http:")
                        || value.starts_with("https:")
                        || value.starts_with("blake3:");
                    assert_eq!(super::is_remote(&value), previous, "{value:?}");
                }
            }
        }
    }
}
