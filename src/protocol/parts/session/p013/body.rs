
fn contains_dependency_marker(text: &str, marker: &str) -> bool {
    text.to_ascii_lowercase().contains(&marker.to_ascii_lowercase())
}
