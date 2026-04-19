//! Unified diff text renderer.
//!
//! Converts `DiffEntry` records with loaded content into standard unified
//! diff format (`---`/`+++` headers, `@@` hunks, `+`/`-` lines). Uses
//! the `similar` crate for line-level diffing.

use similar::ChangeTag;
use similar::TextDiff;

use super::diff::DiffEntry;
use super::diff::DiffKind;

/// Maximum width for the diffstat bar chart.
const DIFFSTAT_BAR_WIDTH: u32 = 40;

fn checked_line_count_u32(input: &str) -> u32 {
    u32::try_from(input.lines().count()).unwrap_or(u32::MAX)
}

fn checked_u32_to_usize(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

struct DiffstatBarScale {
    total_changes: u32,
    max_changes: u32,
}

fn scaled_bar_width(scale: DiffstatBarScale) -> u32 {
    debug_assert!(scale.max_changes > 0);
    if scale.max_changes <= DIFFSTAT_BAR_WIDTH {
        return scale.total_changes;
    }
    let scaled = u64::from(scale.total_changes)
        .saturating_mul(u64::from(DIFFSTAT_BAR_WIDTH))
        .checked_div(u64::from(scale.max_changes))
        .unwrap_or(0);
    u32::try_from(scaled).unwrap_or(DIFFSTAT_BAR_WIDTH)
}

struct DiffstatBarComponent {
    component_changes: u32,
    bar_total: u32,
    total_changes: u32,
}

fn scaled_bar_component(component: DiffstatBarComponent) -> u32 {
    debug_assert!(component.total_changes > 0);
    if component.total_changes == 0 {
        return 0;
    }
    let scaled = u64::from(component.component_changes)
        .saturating_mul(u64::from(component.bar_total))
        .checked_div(u64::from(component.total_changes))
        .unwrap_or(0);
    u32::try_from(scaled).unwrap_or(component.bar_total)
}

fn push_line(out: &mut String, line: &str) {
    debug_assert!(line.len() < usize::MAX);
    out.push_str(line);
    out.push('\n');
}

fn push_fmt_line(out: &mut String, args: std::fmt::Arguments<'_>) {
    let line = args.to_string();
    push_line(out, &line);
}

/// Render unified diff text from diff entries.
///
/// Each entry with loaded content produces a file header and hunks.
/// Entries without content (large blobs or content not requested)
/// get a hash-only summary line.
///
/// # Arguments
///
/// * `entries` - Diff entries (content should be loaded via `include_content`)
/// * `context_lines` - Number of unchanged context lines around changes (default: 3)
pub fn render_unified_diff(entries: &[DiffEntry], context_lines: u32) -> String {
    let mut out = String::new();

    for entry in entries {
        render_entry(&mut out, entry, context_lines);
    }

    out
}

/// Render a diffstat summary (like `git diff --stat`).
///
/// Shows per-file insertion/deletion counts and a bar chart.
pub fn render_diffstat(entries: &[DiffEntry]) -> String {
    debug_assert!(DIFFSTAT_BAR_WIDTH > 0);
    if entries.is_empty() {
        return String::new();
    }

    let mut lines: Vec<(String, u32, u32)> = Vec::with_capacity(entries.len());
    let mut total_add: u32 = 0;
    let mut total_del: u32 = 0;
    let mut max_path_len: usize = 0;

    for entry in entries {
        let path = display_path(entry);
        max_path_len = max_path_len.max(path.len());

        let (add, del) = count_changes(entry);
        total_add = total_add.saturating_add(add);
        total_del = total_del.saturating_add(del);
        lines.push((path, add, del));
    }

    let mut out = String::new();
    let max_changes: u32 = lines
        .iter()
        .map(|(_, add_count, del_count)| add_count.saturating_add(*del_count))
        .max()
        .unwrap_or(1)
        .max(1);

    for (path, add, del) in &lines {
        let total = add.saturating_add(*del);
        let bar_total = scaled_bar_width(DiffstatBarScale {
            total_changes: total,
            max_changes,
        });
        let bar_add = if total > 0 {
            scaled_bar_component(DiffstatBarComponent {
                component_changes: *add,
                bar_total,
                total_changes: total,
            })
        } else {
            0
        };
        let bar_del = bar_total.saturating_sub(bar_add);

        push_fmt_line(
            &mut out,
            format_args!(
                " {:<width$} | {:>5} {}{}",
                path,
                total,
                "+".repeat(checked_u32_to_usize(bar_add)),
                "-".repeat(checked_u32_to_usize(bar_del)),
                width = max_path_len,
            ),
        );
    }

    push_fmt_line(
        &mut out,
        format_args!(" {} files changed, {} insertions(+), {} deletions(-)", lines.len(), total_add, total_del,),
    );

    out
}

/// Render a single diff entry.
fn render_entry(out: &mut String, entry: &DiffEntry, context_lines: u32) {
    debug_assert!(!entry.path.is_empty());
    match entry.kind {
        DiffKind::Renamed => {
            let old = entry.old_path.as_deref().unwrap_or("unknown");
            push_fmt_line(out, format_args!("rename from {old}"));
            push_fmt_line(out, format_args!("rename to {}", entry.path));
            if entry.old_mode != entry.new_mode
                && let (Some(old_mode), Some(new_mode)) = (entry.old_mode, entry.new_mode)
            {
                push_fmt_line(out, format_args!("old mode {old_mode:o}"));
                push_fmt_line(out, format_args!("new mode {new_mode:o}"));
            }
        }
        DiffKind::Added => {
            push_line(out, "--- /dev/null");
            push_fmt_line(out, format_args!("+++ b/{}", entry.path));
            render_content_diff(out, Some(b""), entry.new_content.as_deref(), context_lines);
        }
        DiffKind::Removed => {
            push_fmt_line(out, format_args!("--- a/{}", entry.path));
            push_line(out, "+++ /dev/null");
            render_content_diff(out, entry.old_content.as_deref(), Some(b""), context_lines);
        }
        DiffKind::Modified => {
            push_fmt_line(out, format_args!("--- a/{}", entry.path));
            push_fmt_line(out, format_args!("+++ b/{}", entry.path));
            render_content_diff(out, entry.old_content.as_deref(), entry.new_content.as_deref(), context_lines);
        }
    }
}

/// Render line-level diff between old and new content.
fn render_content_diff(out: &mut String, old: Option<&[u8]>, new: Option<&[u8]>, context_lines: u32) {
    debug_assert!(context_lines <= u32::try_from(usize::MAX).unwrap_or(u32::MAX));
    let (old_bytes, new_bytes) = match (old, new) {
        (Some(old_bytes), Some(new_bytes)) => (old_bytes, new_bytes),
        (Some(old_bytes), None) => (old_bytes, &b""[..]),
        (None, Some(new_bytes)) => (&b""[..], new_bytes),
        (None, None) => {
            push_line(out, "Binary files differ (content not loaded)");
            return;
        }
    };

    let old_str = match std::str::from_utf8(old_bytes) {
        Ok(value) => value,
        Err(_) => {
            push_line(out, "Binary files differ");
            return;
        }
    };
    let new_str = match std::str::from_utf8(new_bytes) {
        Ok(value) => value,
        Err(_) => {
            push_line(out, "Binary files differ");
            return;
        }
    };

    let diff = TextDiff::from_lines(old_str, new_str);
    let formatted = diff.unified_diff().context_radius(checked_u32_to_usize(context_lines)).to_string();

    for line in formatted.lines() {
        if line.starts_with("---") || line.starts_with("+++") {
            continue;
        }
        push_line(out, line);
    }
}

/// Count insertions and deletions for a diff entry.
fn count_changes(entry: &DiffEntry) -> (u32, u32) {
    match entry.kind {
        DiffKind::Added => {
            let lines = entry
                .new_content
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(checked_line_count_u32)
                .unwrap_or(1);
            (lines, 0)
        }
        DiffKind::Removed => {
            let lines = entry
                .old_content
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(checked_line_count_u32)
                .unwrap_or(1);
            (0, lines)
        }
        DiffKind::Modified | DiffKind::Renamed => {
            let old_str = entry.old_content.as_deref().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
            let new_str = entry.new_content.as_deref().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");

            if old_str.is_empty() && new_str.is_empty() {
                return (0, 0);
            }

            let diff = TextDiff::from_lines(old_str, new_str);
            let mut add: u32 = 0;
            let mut del: u32 = 0;
            for change in diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Insert => add = add.saturating_add(1),
                    ChangeTag::Delete => del = del.saturating_add(1),
                    ChangeTag::Equal => {}
                }
            }
            (add, del)
        }
    }
}

/// Display path for diffstat, including rename arrow.
fn display_path(entry: &DiffEntry) -> String {
    if entry.kind == DiffKind::Renamed
        && let Some(old) = &entry.old_path
    {
        return format!("{old} => {}", entry.path);
    }
    entry.path.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(kind: DiffKind, path: &str, old: Option<&[u8]>, new: Option<&[u8]>) -> DiffEntry {
        DiffEntry {
            path: path.to_string(),
            kind,
            old_mode: if kind == DiffKind::Added { None } else { Some(0o100644) },
            new_mode: if kind == DiffKind::Removed {
                None
            } else {
                Some(0o100644)
            },
            old_hash: if kind == DiffKind::Added { None } else { Some([0u8; 32]) },
            new_hash: if kind == DiffKind::Removed {
                None
            } else {
                Some([1u8; 32])
            },
            old_path: None,
            old_content: old.map(|b| b.to_vec()),
            new_content: new.map(|b| b.to_vec()),
        }
    }

    #[test]
    fn test_unified_modified_file() {
        let entry = make_entry(
            DiffKind::Modified,
            "src/main.rs",
            Some(b"line1\nline2\nline3\n"),
            Some(b"line1\nchanged\nline3\n"),
        );

        let output = render_unified_diff(&[entry], 3);
        assert!(output.contains("--- a/src/main.rs"));
        assert!(output.contains("+++ b/src/main.rs"));
        assert!(output.contains("@@"));
        assert!(output.contains("-line2"));
        assert!(output.contains("+changed"));
    }

    #[test]
    fn test_unified_added_file() {
        let entry = make_entry(DiffKind::Added, "new.txt", None, Some(b"hello\nworld\n"));

        let output = render_unified_diff(&[entry], 3);
        assert!(output.contains("--- /dev/null"));
        assert!(output.contains("+++ b/new.txt"));
        assert!(output.contains("+hello"));
        assert!(output.contains("+world"));
    }

    #[test]
    fn test_unified_removed_file() {
        let entry = make_entry(DiffKind::Removed, "old.txt", Some(b"goodbye\n"), None);

        let output = render_unified_diff(&[entry], 3);
        assert!(output.contains("--- a/old.txt"));
        assert!(output.contains("+++ /dev/null"));
        assert!(output.contains("-goodbye"));
    }

    #[test]
    fn test_unified_renamed_file() {
        let mut entry = make_entry(DiffKind::Renamed, "new_name.rs", None, None);
        entry.old_path = Some("old_name.rs".to_string());

        let output = render_unified_diff(&[entry], 3);
        assert!(output.contains("rename from old_name.rs"));
        assert!(output.contains("rename to new_name.rs"));
    }

    #[test]
    fn test_unified_binary_file() {
        let entry = make_entry(
            DiffKind::Modified,
            "image.png",
            Some(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(&[0xFF, 0xD8, 0xFF, 0xE1]),
        );

        let output = render_unified_diff(&[entry], 3);
        assert!(output.contains("Binary files differ"));
    }

    #[test]
    fn test_unified_empty_diff() {
        let output = render_unified_diff(&[], 3);
        assert!(output.is_empty());
    }

    #[test]
    fn test_diffstat_format() {
        let entries = vec![
            make_entry(DiffKind::Modified, "src/main.rs", Some(b"old\n"), Some(b"new\nline\n")),
            make_entry(DiffKind::Added, "README.md", None, Some(b"# Title\n")),
        ];

        let output = render_diffstat(&entries);
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("README.md"));
        assert!(output.contains("files changed"));
        assert!(output.contains("insertions(+)"));
        assert!(output.contains("deletions(-)"));
    }
}
