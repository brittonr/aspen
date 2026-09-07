pub fn entry(input: &str) -> bool { has_platform_prefix(input) }
const fn has_platform_prefix(input: &str) -> bool {
    let bytes = input.as_bytes();
    let has_drive_prefix = bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':');
    has_drive_prefix || input.starts_with("\\\\") || input.contains('\\')
}
