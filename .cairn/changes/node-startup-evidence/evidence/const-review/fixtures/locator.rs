pub fn entry(value: &str) -> bool { has_platform_prefix(value) }
fn has_platform_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    let has_drive_prefix = bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':');
    has_drive_prefix || value.starts_with("\\\\") || value.contains('\\')
}
