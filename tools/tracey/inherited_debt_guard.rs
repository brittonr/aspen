//! Keep inherited traceability debt exact without claiming that the debt is coverage.
//!
//! A requirement is covered only when compiled Rust sources under `src/`, `crates/`, or `tests/`
//! carry both an `impl` and a `verify` marker for it. Markers anywhere else (`tools/`, `docs/`,
//! `scripts/`, `pilots/`, `nix/`, `flake.nix`) and `depends`/`related` markers are
//! cross-references: they are checked for dangling identities but never discharge debt.
//! Coverage-bearing files must not cite repository files that do not exist. r[impl molten.project.
//! inherited_tracey_debt.baseline] r[impl molten.project.inherited_tracey_debt.growth_denial]
//! r[impl molten.project.inherited_tracey_debt.non_claims]
//! r[impl molten.project.inherited_tracey_debt.active_delta_scope]
//! r[impl molten.project.inherited_tracey_debt.active_delta_admission]
//! r[impl molten.project.inherited_tracey_debt.active_delta_non_claims]
//! r[impl molten.testing.traceability_code_backed_coverage]

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

const REQUIREMENT_ROOT: &str = ".cairn/specs";
const ACTIVE_REQUIREMENT_ROOT: &str = ".cairn/changes";
const ACTIVE_SPEC_FILE: &str = "spec.md";
const COVERAGE_ROOTS: &[&str] = &["src", "crates", "tests"];
const COVERAGE_EXTENSIONS: &[&str] = &["rs"];
const CROSS_REFERENCE_ROOTS: &[&str] = &["tools", "docs", "scripts", "pilots", "nix"];
const ROOT_CROSS_REFERENCE_FILE: &str = "flake.nix";
const CROSS_REFERENCE_EXTENSIONS: &[&str] = &["rs", "ncl", "md", "sh", "nix", "py"];
const CITATION_PREFIXES: &[&str] = &[
    ".cairn/",
    "cairn/",
    "checks/",
    "config/",
    "crates/",
    "docs/",
    "evidence/",
    "fixtures/",
    "nix/",
    "pilots/",
    "schemas/",
    "scripts/",
    "src/",
    "tests/",
    "tools/",
    "wit/",
];
const CITATION_TEMPLATE_CHARACTERS: &[char] = &['*', '?', '{', '}', '<', '>', '$'];
const REQUIREMENT_EXTENSION: &str = "md";
const OPTION_ROOT: &str = "--root";
const OPTION_BASELINE: &str = "--baseline";
const OPTION_WRITE_BASELINE: &str = "--write-baseline";
const ADJACENT_PAIR_WIDTH: usize = 2;
const CARGO_MANIFEST: &str = "Cargo.toml";
const TARGET_SECTIONS: &[&str] = &["[lib]", "[[bin]]", "[[test]]", "[[bench]]", "[[example]]"];
const CONVENTIONAL_TARGETS: &[&str] = &["src/lib.rs", "src/main.rs", "build.rs"];
const AUTO_TARGET_DIRECTORIES: &[(&str, &str)] = &[
    ("src/bin", "autobins"),
    ("tests", "autotests"),
    ("benches", "autobenches"),
    ("examples", "autoexamples"),
];
const TARGET_MAIN_FILE: &str = "main.rs";
const MOD_RS_STEMS: &[&str] = &["mod", "lib", "main"];
const CFG_DISABLED: &[&str] = &["#[cfg(any())]", "#[cfg(FALSE)]", "#[cfg(not(all()))]"];
const CRATE_CFG_DISABLED: &[&str] = &["#![cfg(any())]", "#![cfg(FALSE)]", "#![cfg(not(all()))]"];
const INCLUDE_MACRO: &str = "include!(";
const MACRO_RULES: &str = "macro_rules!";
const BUILTIN_INCLUDE_QUALIFIERS: &[&str] = &["", "std::", "core::", "::std::", "::core::"];
const MANIFEST_DIR_VARIABLE: &str = "CARGO_MANIFEST_DIR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkerVerb {
    Impl,
    Verify,
    Depends,
    Related,
}

impl MarkerVerb {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "impl" => Some(Self::Impl),
            "verify" => Some(Self::Verify),
            "depends" => Some(Self::Depends),
            "related" => Some(Self::Related),
            _ => None,
        }
    }
}

/// Whether a scanned file can discharge debt (compiled code/test sources) or only cross-reference
/// requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceClass {
    Coverage,
    CrossReference,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct References {
    /// Every referenced identity, from any verb and any scanned root; used for dangling detection
    /// only.
    all: BTreeSet<String>,
    implemented: BTreeSet<String>,
    verified: BTreeSet<String>,
    broken_citations: BTreeSet<String>,
    comment_only_stubs: BTreeSet<String>,
    /// Rust files under coverage roots that carry `impl`/`verify` markers but no compiled target
    /// reaches.
    uncompiled_marker_files: BTreeSet<String>,
    /// Rust files defining a macro named `include`, which would shadow the built-in the scanner
    /// follows.
    shadowed_include_files: BTreeSet<String>,
}

impl References {
    /// Records every marker for dangling detection. For compiled sources, only markers written in
    /// live comments (not string literals, and not inside items disabled by an always-false
    /// `cfg`) discharge coverage.
    fn record_text(&mut self, class: SourceClass, label: &str, text: &str, cited_path_exists: impl Fn(&str) -> bool) {
        for line in text.lines() {
            self.all.extend(reference_markers(line).into_iter().map(|(_, id)| id));
        }
        if class != SourceClass::Coverage {
            return;
        }
        let live_comments = live_comment_text(text);
        let mut is_coverage_bearing = false;
        for line in live_comments.lines() {
            for (verb, id) in reference_markers(line) {
                match verb {
                    MarkerVerb::Impl => {
                        is_coverage_bearing = true;
                        self.implemented.insert(id);
                    }
                    MarkerVerb::Verify => {
                        is_coverage_bearing = true;
                        self.verified.insert(id);
                    }
                    MarkerVerb::Depends | MarkerVerb::Related => {}
                }
            }
        }
        if !is_coverage_bearing {
            return;
        }
        for (index, line) in live_comments.lines().enumerate() {
            for citation in file_citations(line) {
                if !cited_path_exists(&citation) {
                    self.broken_citations.insert(format!("{label}:{}: cites missing {citation}", index + 1));
                }
            }
        }
    }

    /// Records an uncompiled Rust file that contains nothing but comments and traceability markers:
    /// such stubs exist only to look like coverage and are rejected outright.
    fn record_comment_only_stub(&mut self, label: &str, text: &str) {
        let has_marker = text.lines().any(|line| !reference_markers(line).is_empty());
        let is_comment_only = text.lines().map(str::trim).all(|line| line.is_empty() || line.starts_with("//"));
        if has_marker && is_comment_only {
            self.comment_only_stubs.insert(label.to_string());
        }
    }

    fn covered(&self) -> BTreeSet<String> {
        self.implemented.intersection(&self.verified).cloned().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Coverage {
    requirements: BTreeSet<String>,
    references: BTreeSet<String>,
    covered: BTreeSet<String>,
    missing: BTreeSet<String>,
    dangling: BTreeSet<String>,
    pending: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BaselineComparison {
    unexpected_missing: BTreeSet<String>,
    stale_baseline: BTreeSet<String>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn requirement_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let marker = trimmed.strip_prefix("r[")?;
    let end = marker.find(']')?;
    let raw = &marker[..end];
    if raw.split_whitespace().count() != 1 {
        return None;
    }
    let id = raw.split('+').next().unwrap_or(raw);
    valid_id(id).then(|| id.to_string())
}

fn reference_markers(line: &str) -> Vec<(MarkerVerb, String)> {
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = line[cursor..].find("r[") {
        let marker_start = cursor + offset + "r[".len();
        let Some(end_offset) = line[marker_start..].find(']') else {
            break;
        };
        let raw = &line[marker_start..marker_start + end_offset];
        let mut parts = raw.split_whitespace();
        let verb = MarkerVerb::parse(parts.next().unwrap_or_default());
        let raw_id = parts.next().unwrap_or_default();
        let id = raw_id.split('+').next().unwrap_or(raw_id);
        if let Some(verb) = verb.filter(|_| valid_id(id)) {
            found.push((verb, id.to_string()));
        }
        cursor = marker_start + end_offset + 1;
    }
    found
}

/// Backtick-quoted repository paths cited on one line. Globs and templates are not file citations;
/// a trailing `::item`, `#anchor`, `:line` suffix, or sentence punctuation is not part of the cited
/// path.
fn file_citations(line: &str) -> Vec<String> {
    line.split('`')
        .skip(1)
        .step_by(ADJACENT_PAIR_WIDTH)
        .filter(|token| CITATION_PREFIXES.iter().any(|prefix| token.starts_with(prefix)))
        .filter(|token| !token.contains(char::is_whitespace) && !token.contains(CITATION_TEMPLATE_CHARACTERS))
        .map(|token| {
            let path = token.split("::").next().unwrap_or(token);
            let path = path.split('#').next().unwrap_or(path);
            let path = path.split(':').next().unwrap_or(path);
            path.trim_end_matches(['.', ',', ';', ')']).to_string()
        })
        .collect()
}

fn classify(
    requirements: BTreeSet<String>,
    active_requirements: &BTreeSet<String>,
    references: &References,
) -> Coverage {
    let covered = references.covered();
    let missing = requirements.difference(&covered).cloned().collect();
    let (pending, dangling) = references
        .all
        .difference(&requirements)
        .cloned()
        .partition(|requirement| active_requirements.contains(requirement));
    Coverage {
        requirements,
        references: references.all.clone(),
        covered,
        missing,
        dangling,
        pending,
    }
}

fn compare_baseline(actual: &BTreeSet<String>, baseline: &BTreeSet<String>) -> BaselineComparison {
    BaselineComparison {
        unexpected_missing: actual.difference(baseline).cloned().collect(),
        stale_baseline: baseline.difference(actual).cloned().collect(),
    }
}

fn baseline_is_sorted_and_unique(lines: &[String]) -> bool {
    !lines.iter().any(String::is_empty) && lines.windows(ADJACENT_PAIR_WIDTH).all(|pair| pair[0] < pair[1])
}

fn walk_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    if !path.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;
        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|error| format!("{}: {error}", entry_path.display()))?;
        if file_type.is_dir() {
            walk_files(&entry_path, files)?;
        } else if file_type.is_file() {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.contains(&extension))
}

fn read_requirements(root: &Path) -> Result<BTreeSet<String>, String> {
    let requirement_root = root.join(REQUIREMENT_ROOT);
    if !requirement_root.is_dir() {
        return Err(format!("requirement root {} is missing", requirement_root.display()));
    }
    let mut files = Vec::new();
    walk_files(&requirement_root, &mut files)?;
    files.retain(|path| has_extension(path, &[REQUIREMENT_EXTENSION]));
    let mut requirements = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        for line in text.lines() {
            if let Some(requirement) = requirement_marker(line) {
                requirements.insert(requirement);
            }
        }
    }
    if requirements.is_empty() {
        return Err(format!("requirement root {} contains no requirements", requirement_root.display()));
    }
    Ok(requirements)
}

fn is_active_delta_spec(change_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(change_root) else {
        return false;
    };
    let mut components = relative.components();
    if !matches!(components.next(), Some(Component::Normal(_)))
        || components.next() != Some(Component::Normal(OsStr::new("specs")))
        || !matches!(components.next(), Some(Component::Normal(_)))
    {
        return false;
    }
    components.next().is_some() && relative.file_name() == Some(OsStr::new(ACTIVE_SPEC_FILE))
}

fn active_requirement_markers(text: &str) -> impl Iterator<Item = String> + '_ {
    let mut admits_requirements = false;
    text.lines().filter_map(move |line| {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            admits_requirements = matches!(trimmed, "## ADDED Requirements" | "## MODIFIED Requirements");
            return None;
        }
        admits_requirements.then(|| requirement_marker(trimmed)).flatten()
    })
}

fn read_active_requirements(root: &Path) -> Result<BTreeSet<String>, String> {
    let change_root = root.join(ACTIVE_REQUIREMENT_ROOT);
    let mut files = Vec::new();
    walk_files(&change_root, &mut files)?;
    let mut requirements = BTreeSet::new();
    for path in files.iter().filter(|path| is_active_delta_spec(&change_root, path)) {
        let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        requirements.extend(active_requirement_markers(&text));
    }
    Ok(requirements)
}

fn read_references(root: &Path) -> Result<References, String> {
    let compiled = compiled_sources(root)?;
    let mut sources = Vec::new();
    for source_root in COVERAGE_ROOTS {
        let mut files = Vec::new();
        walk_files(&root.join(source_root), &mut files)?;
        for path in files {
            if has_extension(&path, COVERAGE_EXTENSIONS) && compiled.contains(&normalize(&path)) {
                sources.push((SourceClass::Coverage, path));
            } else if has_extension(&path, CROSS_REFERENCE_EXTENSIONS) {
                sources.push((SourceClass::CrossReference, path));
            }
        }
    }
    let mut files = Vec::new();
    for source_root in CROSS_REFERENCE_ROOTS {
        walk_files(&root.join(source_root), &mut files)?;
    }
    walk_files(&root.join(ROOT_CROSS_REFERENCE_FILE), &mut files)?;
    files.retain(|path| has_extension(path, CROSS_REFERENCE_EXTENSIONS));
    sources.extend(files.into_iter().map(|path| (SourceClass::CrossReference, path)));

    let mut references = References::default();
    for (class, path) in sources {
        let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let label = path.strip_prefix(root).unwrap_or(&path).display().to_string();
        let is_uncompiled_rust = class == SourceClass::CrossReference && has_extension(&path, COVERAGE_EXTENSIONS);
        let has_coverage_marker = text
            .lines()
            .flat_map(reference_markers)
            .any(|(verb, _)| matches!(verb, MarkerVerb::Impl | MarkerVerb::Verify));
        if is_uncompiled_rust && has_coverage_marker && COVERAGE_ROOTS.iter().any(|root| label.starts_with(root)) {
            references.uncompiled_marker_files.insert(label.clone());
        }
        references.record_text(class, &label, &text, |citation| root.join(citation).exists());
        if has_extension(&path, COVERAGE_EXTENSIONS) && defines_include_macro(&rust_views(&text).code) {
            references.shadowed_include_files.insert(label.clone());
        }
        if is_uncompiled_rust {
            references.record_comment_only_stub(&label, &text);
        }
    }
    Ok(references)
}

/// Lexically resolves `.` and `..` components so module paths compare equal to walked paths.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn quoted_strings(text: &str) -> Vec<String> {
    text.split('"').skip(1).step_by(ADJACENT_PAIR_WIDTH).map(str::to_string).collect()
}

/// Workspace member package directories named by the root manifest (the root package alone without
/// one).
fn workspace_members(root: &Path) -> Result<Vec<PathBuf>, String> {
    let manifest_path = root.join(CARGO_MANIFEST);
    let manifest =
        fs::read_to_string(&manifest_path).map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let members = manifest
        .find("members")
        .and_then(|start| manifest[start..].find(']').map(|end| &manifest[start..start + end]))
        .map(quoted_strings)
        .unwrap_or_default();
    if members.is_empty() {
        return Ok(vec![normalize(root)]);
    }
    Ok(members.iter().map(|member| normalize(&root.join(member))).collect())
}

/// Every compiled target root of one package: explicit
/// `[lib]`/`[[bin]]`/`[[test]]`/`[[bench]]`/`[[example]]` paths plus Cargo's conventional and
/// auto-discovered targets.
fn target_roots(package: &Path) -> Result<Vec<PathBuf>, String> {
    let Ok(manifest) = fs::read_to_string(package.join(CARGO_MANIFEST)) else {
        return Ok(Vec::new());
    };
    let mut roots = Vec::new();
    let mut section = "";
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line;
        } else if TARGET_SECTIONS.contains(&section) && line.starts_with("path") {
            roots.extend(quoted_strings(line).first().map(|path| package.join(path)));
        }
    }
    let is_disabled = |key: &str| manifest.lines().any(|line| line.replace(' ', "") == format!("{key}=false"));
    roots.extend(CONVENTIONAL_TARGETS.iter().map(|path| package.join(path)).filter(|path| path.is_file()));
    for (directory, key) in AUTO_TARGET_DIRECTORIES {
        if is_disabled(key) {
            continue;
        }
        let Ok(entries) = fs::read_dir(package.join(directory)) else {
            continue;
        };
        for entry in entries {
            let path = entry.map_err(|error| format!("{}: {error}", package.display()))?.path();
            if path.is_file() && has_extension(&path, COVERAGE_EXTENSIONS) {
                roots.push(path);
            } else if path.join(TARGET_MAIN_FILE).is_file() {
                roots.push(path.join(TARGET_MAIN_FILE));
            }
        }
    }
    Ok(roots)
}

/// Rust sources reachable from a workspace target's module tree through `mod` declarations,
/// `#[path]` mounts, and `include!`, excluding items and files disabled by an always-false `cfg`.
fn compiled_sources(root: &Path) -> Result<BTreeSet<PathBuf>, String> {
    let mut reached = BTreeSet::new();
    let mut queue = Vec::new();
    for package in workspace_members(root)? {
        for target in target_roots(&package)? {
            let bases = target.parent().map(Path::to_path_buf).into_iter().collect::<Vec<_>>();
            queue.push((normalize(&target), bases, package.clone()));
        }
    }
    while let Some((file, bases, package)) = queue.pop() {
        if !file.is_file() || reached.contains(&file) {
            continue;
        }
        let text = fs::read_to_string(&file).map_err(|error| format!("{}: {error}", file.display()))?;
        if file_is_cfg_disabled(&text) {
            continue;
        }
        reached.insert(file.clone());
        let directory = file.parent().map(Path::to_path_buf).unwrap_or_default();
        for (name, path_attribute) in module_declarations(&text) {
            let mut candidates = Vec::new();
            for base in bases.iter().chain(std::iter::once(&directory)) {
                match &path_attribute {
                    Some(path) => candidates.push(base.join(path)),
                    None => candidates.extend([base.join(format!("{name}.rs")), base.join(&name).join("mod.rs")]),
                }
            }
            if let Some(child) = candidates.into_iter().map(|path| normalize(&path)).find(|path| path.is_file()) {
                let child_bases = module_bases(&child);
                queue.push((child, child_bases, package.clone()));
            }
        }
        for included in included_paths(&text, &directory, &package) {
            let mut included_bases = bases.clone();
            included_bases.extend(module_bases(&included));
            queue.push((included, included_bases, package.clone()));
        }
    }
    Ok(reached)
}

/// Directories in which `mod child;` declared by `file` may resolve: beside a `mod.rs`/root file,
/// or under the file stem for non-`mod.rs` modules.
fn module_bases(file: &Path) -> Vec<PathBuf> {
    let directory = file.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = file.file_stem().and_then(OsStr::to_str).unwrap_or_default();
    if MOD_RS_STEMS.contains(&stem) {
        vec![directory]
    } else {
        vec![directory.join(stem), directory]
    }
}

/// Item-level out-of-line `mod name;` declarations with their `#[path]` attribute. Only real tokens
/// count: comments and string literals are blanked first, and declarations inside always-false
/// `cfg` items, `macro_rules!` bodies, or any macro invocation's arguments are skipped.
fn module_declarations(text: &str) -> Vec<(String, Option<String>)> {
    let views = rust_views(text);
    let disabled = inert_lines(&views);
    let arguments = macro_argument_ranges(&views.code);
    let original_lines = text.split('\n').collect::<Vec<_>>();
    let mut declarations = Vec::new();
    let mut path_attribute = None;
    let mut offset = 0;
    for (index, raw_line) in views.code.split('\n').enumerate() {
        let line_start = offset + (raw_line.len() - raw_line.trim_start().len());
        offset += raw_line.len() + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#[") {
            if line.starts_with("#[path") {
                path_attribute =
                    original_lines.get(index).and_then(|original| quoted_strings(original).into_iter().next());
            }
            continue;
        }
        if let Some(name) = out_of_line_module(line)
            && !disabled.contains(&index)
            && !arguments.iter().any(|range| range.contains(&line_start))
        {
            declarations.push((name, path_attribute.clone()));
        }
        path_attribute = None;
    }
    declarations
}

fn out_of_line_module(line: &str) -> Option<String> {
    let line = line.strip_prefix("pub").map_or(line, |rest| {
        let rest = rest.trim_start();
        rest.strip_prefix('(')
            .and_then(|scoped| scoped.split_once(')'))
            .map_or(rest, |(_, after)| after.trim_start())
    });
    let name = line.strip_prefix("mod ")?.strip_suffix(';')?.trim();
    let name = name.strip_prefix("r#").unwrap_or(name);
    (!name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
        .then(|| name.to_string())
}

/// Files named by item-level `include!` tokens, relative to the including file or, with
/// `CARGO_MANIFEST_DIR`, to the package root. Invocations in comments, string literals,
/// always-false `cfg` items, `macro_rules!` bodies, or another macro invocation's arguments are
/// ignored, and macro-mediated includes are never resolved, so files reached only through a macro
/// are reported as debt rather than coverage.
fn included_paths(text: &str, directory: &Path, package: &Path) -> Vec<PathBuf> {
    let views = rust_views(text);
    let inert = inert_lines(&views);
    let arguments = macro_argument_ranges(&views.code);
    let code = views.code.as_bytes();
    let mut paths = Vec::new();
    for (start, _) in views.code.match_indices(INCLUDE_MACRO) {
        let is_token_start = start == 0 || !is_identifier_byte(code[start - 1]);
        if !is_token_start
            || !is_builtin_include_path(&macro_path_qualifier(code, start))
            || inert.contains(&line_of(code, start))
            || arguments.iter().any(|range| range.contains(&start))
        {
            continue;
        }
        let open = start + code[start..].iter().position(|byte| *byte == b'(').unwrap_or_default();
        let invocation = &text[start..matching_close(code, open).min(text.len())];
        let strings = quoted_strings(invocation);
        let path = if invocation.contains(MANIFEST_DIR_VARIABLE) {
            let relative = strings.iter().filter(|value| *value != MANIFEST_DIR_VARIABLE).cloned().collect::<String>();
            package.join(relative.trim_start_matches('/'))
        } else if let Some(relative) = strings.first() {
            directory.join(relative)
        } else {
            continue;
        };
        paths.push(normalize(&path));
    }
    paths
}

/// Byte ranges (open..=close delimiter) of the token trees passed to every macro invocation other
/// than `include!` itself. Tokens inside them are not item-level: the macro may discard or rewrite
/// them, so `mod`/`include!` there add no reachability. `include!`'s own argument stays live, so
/// `include!(concat!(env!(…), "…"))` still resolves.
fn macro_argument_ranges(code: &str) -> Vec<std::ops::RangeInclusive<usize>> {
    let bytes = code.as_bytes();
    let mut ranges = Vec::new();
    for (bang, _) in code.match_indices('!') {
        if bang == 0 || !is_identifier_byte(bytes[bang - 1]) {
            continue;
        }
        let name_start =
            bytes[..bang].iter().rposition(|byte| !is_identifier_byte(*byte)).map_or(0, |position| position + 1);
        let is_builtin_include = &code[name_start..bang] == INCLUDE_MACRO.trim_end_matches("!(")
            && is_builtin_include_path(&macro_path_qualifier(bytes, name_start));
        if is_builtin_include {
            continue;
        }
        let Some(relative) = bytes[bang + 1..].iter().position(|byte| !byte.is_ascii_whitespace()) else {
            continue;
        };
        let open = bang + 1 + relative;
        if matches!(bytes[open], b'(' | b'{' | b'[') {
            ranges.push(open..=matching_close(bytes, open).min(bytes.len().saturating_sub(1)));
        }
    }
    ranges
}

/// The `path::` prefix written before a macro name starting at `name_start` (empty when
/// unqualified).
fn macro_path_qualifier(code: &[u8], name_start: usize) -> String {
    let mut start = name_start;
    while start >= 2 && &code[start - 2..start] == b"::" {
        start -= 2;
        while start > 0 && (is_identifier_byte(code[start - 1]) || code[start - 1] == b'$') {
            start -= 1;
        }
    }
    String::from_utf8_lossy(&code[start..name_start]).into_owned()
}

/// The guard does not resolve macro paths: only the unqualified name or an explicit `std`/`core`
/// path is treated as the built-in `include!`.
fn is_builtin_include_path(qualifier: &str) -> bool {
    BUILTIN_INCLUDE_QUALIFIERS.contains(&qualifier)
}

/// Whether the source defines a macro named `include` (`macro_rules! include`, `r#include`, or
/// `macro include`), which would shadow the built-in the scanner follows.
fn defines_include_macro(code: &str) -> bool {
    let bytes = code.as_bytes();
    code.match_indices("macro").any(|(start, keyword)| {
        if start > 0 && is_identifier_byte(bytes[start - 1]) {
            return false;
        }
        let rest = &code[start + keyword.len()..];
        let rest = rest.strip_prefix("_rules!").unwrap_or(rest);
        if rest.len() == code[start + keyword.len()..].len() && !rest.starts_with(char::is_whitespace) {
            return false;
        }
        let name = rest.trim_start();
        let name = name.strip_prefix("r#").unwrap_or(name);
        name.strip_prefix("include").is_some_and(|after| {
            !after.starts_with(|character: char| character.is_ascii_alphanumeric() || character == '_')
        })
    })
}

/// Byte ranges (open..=close delimiter) of every `macro_rules!` definition body.
fn macro_bodies(code: &str) -> Vec<std::ops::RangeInclusive<usize>> {
    let bytes = code.as_bytes();
    let mut bodies = Vec::new();
    for (start, keyword) in code.match_indices(MACRO_RULES) {
        if start > 0 && is_identifier_byte(bytes[start - 1]) {
            continue;
        }
        let after = start + keyword.len();
        let Some(relative) = bytes[after..].iter().position(|byte| matches!(byte, b'{' | b'(' | b'[')) else {
            continue;
        };
        let open = after + relative;
        bodies.push(open..=matching_close(bytes, open).min(bytes.len().saturating_sub(1)));
    }
    bodies
}

/// Lines whose tokens and comments never take effect as written: always-false `cfg` spans and
/// `macro_rules!` definition bodies. They add no module reachability and discharge no coverage.
fn inert_lines(views: &RustViews) -> BTreeSet<usize> {
    let bytes = views.code.as_bytes();
    let mut inert = cfg_disabled_lines(views);
    for body in macro_bodies(&views.code) {
        inert.extend(line_of(bytes, *body.start())..=line_of(bytes, *body.end()));
    }
    inert
}

/// Byte-for-byte views of Rust source. `code` blanks comments and the contents of string,
/// raw-string, and char literals; `comments` keeps only comment text. Newlines survive in both, so
/// offsets and line numbers align with the original text.
struct RustViews {
    code: String,
    comments: String,
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn blank(buffer: &mut [u8], source: &[u8], range: std::ops::Range<usize>) {
    for index in range {
        if source[index] != b'\n' {
            buffer[index] = b' ';
        }
    }
}

fn rust_views(text: &str) -> RustViews {
    let bytes = text.as_bytes();
    let length = bytes.len();
    let mut code = bytes.to_vec();
    let mut comments = bytes.iter().map(|byte| if *byte == b'\n' { b'\n' } else { b' ' }).collect::<Vec<_>>();
    let at = |index: usize| bytes.get(index).copied().unwrap_or_default();
    let mut index = 0;
    while index < length {
        let byte = bytes[index];
        let previous_is_identifier = index > 0 && is_identifier_byte(bytes[index - 1]);
        // `r"…"`/`r#"…"#` or the byte form `br"…"`, but not an identifier that merely ends in `r`.
        let is_byte_raw_prefix =
            index > 0 && bytes[index - 1] == b'b' && (index < 2 || !is_identifier_byte(bytes[index - 2]));
        let starts_raw_string = byte == b'r' && (!previous_is_identifier || is_byte_raw_prefix);
        let end = if byte == b'/' && at(index + 1) == b'/' {
            let end = bytes[index..].iter().position(|byte| *byte == b'\n').map_or(length, |offset| index + offset);
            comments[index..end].copy_from_slice(&bytes[index..end]);
            blank(&mut code, bytes, index..end);
            end
        } else if byte == b'/' && at(index + 1) == b'*' {
            let mut depth = 1usize;
            let mut cursor = index + 2;
            while cursor < length && depth > 0 {
                if at(cursor) == b'/' && at(cursor + 1) == b'*' {
                    depth += 1;
                    cursor += 2;
                } else if at(cursor) == b'*' && at(cursor + 1) == b'/' {
                    depth -= 1;
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
            let end = cursor.min(length);
            comments[index..end].copy_from_slice(&bytes[index..end]);
            blank(&mut code, bytes, index..end);
            end
        } else if byte == b'"' {
            let mut cursor = index + 1;
            while cursor < length && bytes[cursor] != b'"' {
                cursor += if bytes[cursor] == b'\\' { 2 } else { 1 };
            }
            let end = cursor.min(length);
            blank(&mut code, bytes, index + 1..end);
            end + 1
        } else if starts_raw_string && matches!(at(index + 1), b'#' | b'"') {
            let hashes = bytes[index + 1..].iter().take_while(|byte| **byte == b'#').count();
            let open = index + 1 + hashes;
            if at(open) != b'"' {
                index += 1;
                continue;
            }
            let mut closing = vec![b'"'];
            closing.extend(std::iter::repeat_n(b'#', hashes));
            let end = bytes[open + 1..]
                .windows(closing.len())
                .position(|window| window == closing.as_slice())
                .map_or(length, |offset| open + 1 + offset);
            blank(&mut code, bytes, open + 1..end);
            end + closing.len()
        } else if byte == b'\'' {
            let literal_end = if at(index + 1) == b'\\' {
                bytes[index + 2..].iter().position(|byte| *byte == b'\'').map(|offset| index + 2 + offset)
            } else {
                let width = text[index + 1..].chars().next().map_or(1, char::len_utf8);
                (at(index + 1 + width) == b'\'').then_some(index + 1 + width)
            };
            match literal_end {
                Some(end) => {
                    blank(&mut code, bytes, index + 1..end);
                    end + 1
                }
                None => index + 1,
            }
        } else {
            index + 1
        };
        index = end.max(index + 1);
    }
    RustViews {
        code: String::from_utf8(code).unwrap_or_default(),
        comments: String::from_utf8(comments).unwrap_or_default(),
    }
}

fn line_of(bytes: &[u8], offset: usize) -> usize {
    bytes[..offset.min(bytes.len())].iter().filter(|byte| **byte == b'\n').count()
}

fn compact(line: &str) -> String {
    line.chars().filter(|character| !character.is_whitespace()).collect()
}

/// Whether an inner always-false `cfg` attribute at file scope disables the whole file.
fn file_is_cfg_disabled(text: &str) -> bool {
    let views = rust_views(text);
    let code = views.code.as_bytes();
    let mut offset = 0;
    for line in views.code.split('\n') {
        let compacted = compact(line);
        if CRATE_CFG_DISABLED.iter().any(|attribute| compacted.starts_with(attribute))
            && enclosing_open_brace(code, offset).is_none()
        {
            return true;
        }
        offset += line.len() + 1;
    }
    false
}

fn enclosing_open_brace(code: &[u8], offset: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..offset.min(code.len())).rev() {
        match code[index] {
            b'}' => depth += 1,
            b'{' if depth == 0 => return Some(index),
            b'{' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Offset of the delimiter closing the `(`, `[`, or `{` at `open` (end of input when unbalanced).
fn matching_close(code: &[u8], open: usize) -> usize {
    let (opener, closer) = match code.get(open) {
        Some(b'(') => (b'(', b')'),
        Some(b'[') => (b'[', b']'),
        _ => (b'{', b'}'),
    };
    let mut depth = 0usize;
    for (index, byte) in code.iter().enumerate().skip(open) {
        if *byte == opener {
            depth += 1;
        } else if *byte == closer {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return index;
            }
        }
    }
    code.len()
}

/// Zero-based line numbers covered by items disabled by an always-false `cfg`: an outer attribute
/// disables the following item through its closing `}` or `;`, together with the comment and
/// attribute lines directly attached above it; an inner attribute disables its enclosing block.
fn cfg_disabled_lines(views: &RustViews) -> BTreeSet<usize> {
    let bytes = views.code.as_bytes();
    let code_lines = views.code.split('\n').collect::<Vec<_>>();
    let comment_lines = views.comments.split('\n').collect::<Vec<_>>();
    let mut disabled = BTreeSet::new();
    let mut offset = 0;
    for (index, line) in code_lines.iter().enumerate() {
        let compacted = compact(line);
        let span = if CFG_DISABLED.iter().any(|attribute| compacted.starts_with(attribute)) {
            let start = offset + line.find(']').map_or(line.len(), |position| position + 1);
            let item_end = bytes[start..].iter().position(|byte| matches!(byte, b'{' | b';')).map(|relative| {
                let position = start + relative;
                if bytes[position] == b'{' {
                    matching_close(bytes, position)
                } else {
                    position
                }
            });
            let mut first = index;
            while first > 0 {
                let above = code_lines[first - 1].trim();
                let is_attached_comment =
                    above.is_empty() && comment_lines.get(first - 1).is_some_and(|comment| !comment.trim().is_empty());
                if !(is_attached_comment || above.starts_with("#[")) {
                    break;
                }
                first -= 1;
            }
            Some((first, line_of(bytes, item_end.unwrap_or(bytes.len()))))
        } else if CRATE_CFG_DISABLED.iter().any(|attribute| compacted.starts_with(attribute)) {
            enclosing_open_brace(bytes, offset)
                .map(|open| (line_of(bytes, open), line_of(bytes, matching_close(bytes, open))))
        } else {
            None
        };
        if let Some((first, last)) = span {
            disabled.extend(first..=last);
        }
        offset += line.len() + 1;
    }
    disabled
}

/// Comment text of compiled source with every comment inside an always-false `cfg` item or a
/// `macro_rules!` body blanked.
fn live_comment_text(text: &str) -> String {
    let views = rust_views(text);
    let disabled = inert_lines(&views);
    views
        .comments
        .split('\n')
        .enumerate()
        .map(|(index, line)| if disabled.contains(&index) { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_baseline(path: &Path) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    if !baseline_is_sorted_and_unique(&lines) {
        return Err("baseline must contain unique, sorted, non-empty requirement identifiers".to_string());
    }
    let values = lines.iter().cloned().collect();
    Ok((lines, values))
}

fn write_baseline(path: &Path, missing: &BTreeSet<String>) -> Result<(), String> {
    let mut text = missing.iter().cloned().collect::<Vec<_>>().join("\n");
    text.push('\n');
    fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))
}

fn option_value(args: &[String], option: &str) -> Option<PathBuf> {
    args.windows(ADJACENT_PAIR_WIDTH).find(|pair| pair[0] == option).map(|pair| PathBuf::from(&pair[1]))
}

fn render_values(label: &str, values: &BTreeSet<String>) {
    for value in values {
        eprintln!("{label}: {value}");
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let root = option_value(args, OPTION_ROOT).unwrap_or_else(|| PathBuf::from("."));
    let requirements = read_requirements(&root)?;
    let active_requirements = read_active_requirements(&root)?;
    let references = read_references(&root)?;
    if !references.shadowed_include_files.is_empty() {
        render_values("shadowed_include", &references.shadowed_include_files);
        return Err("a macro named `include` shadows the built-in include! the scanner follows".to_string());
    }
    if !references.broken_citations.is_empty() || !references.comment_only_stubs.is_empty() {
        render_values("broken_citation", &references.broken_citations);
        render_values("comment_only_stub", &references.comment_only_stubs);
        return Err("coverage-bearing files must cite existing files and Rust stubs must not hold markers".to_string());
    }
    let coverage = classify(requirements, &active_requirements, &references);
    println!("uncompiled_marker_files={}", references.uncompiled_marker_files.len());
    render_values("uncompiled_marker_file", &references.uncompiled_marker_files);
    println!("pending_references={}", coverage.pending.len());
    println!(concat!(
        "pending_non_claim=identity resolution only; not accepted coverage, marker truth, ",
        "package validity, runtime behavior, sync/archive authority, or release readiness"
    ));

    if let Some(output) = option_value(args, OPTION_WRITE_BASELINE) {
        if !coverage.dangling.is_empty() {
            render_values("dangling", &coverage.dangling);
            return Err("cannot write a baseline while dangling references exist".to_string());
        }
        write_baseline(&output, &coverage.missing)?;
        println!("baseline_written={}", output.display());
        println!("uncovered={}", coverage.missing.len());
        return Ok(());
    }

    let baseline_path =
        option_value(args, OPTION_BASELINE).ok_or_else(|| format!("missing required option {OPTION_BASELINE}"))?;
    let (baseline_lines, baseline) = read_baseline(&baseline_path)?;
    let comparison = compare_baseline(&coverage.missing, &baseline);

    println!("requirements={}", coverage.requirements.len());
    println!("referenced={}", coverage.requirements.intersection(&coverage.references).count());
    println!("covered={}", coverage.requirements.intersection(&coverage.covered).count());
    println!("uncovered={}", coverage.missing.len());
    println!("baseline_entries={}", baseline_lines.len());
    println!("dangling={}", coverage.dangling.len());
    println!(concat!(
        "non_claim=inherited requirements remain uncovered until compiled src/, crates/, or tests/ Rust sources ",
        "carry both impl and verify markers; prose, tool, flake, depends, and related markers never discharge debt"
    ));

    if !coverage.dangling.is_empty() {
        render_values("dangling", &coverage.dangling);
        return Err("dangling traceability references are not permitted".to_string());
    }
    if !comparison.unexpected_missing.is_empty() || !comparison.stale_baseline.is_empty() {
        render_values("unexpected_missing", &comparison.unexpected_missing);
        render_values("stale_baseline", &comparison.stale_baseline);
        return Err("traceability debt differs from the reviewed baseline".to_string());
    }
    println!("verdict=pass");
    Ok(())
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if let Err(error) = run(&args) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture texts spell marker openings as `@[` so that this tool's own source does not carry
    /// fixture markers into the live repository scan; fixture files are written with real `r[`
    /// markers.
    const FIXTURE_MARKER_OPEN: &str = "@[";

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    /// References whose identities all carry compiled `impl` and `verify` markers.
    fn code_refs(values: &[&str]) -> References {
        References {
            all: set(values),
            implemented: set(values),
            verified: set(values),
            ..References::default()
        }
    }

    /// A throwaway repository tree under the system temporary directory, removed on drop.
    struct FixtureRoot(PathBuf);

    impl FixtureRoot {
        fn new(name: &str, files: &[(&str, &str)]) -> Self {
            let root = env::temp_dir().join(format!("molten-tracey-guard-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("fixture root");
            if !files.iter().any(|(path, _)| *path == CARGO_MANIFEST) {
                fs::write(root.join(CARGO_MANIFEST), "[package]\nname = \"fixture\"\n").expect("fixture manifest");
            }
            for (path, text) in files {
                let path = root.join(path);
                fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
                fs::write(&path, text.replace(FIXTURE_MARKER_OPEN, "r[")).expect("fixture file");
            }
            Self(root)
        }
    }

    impl Drop for FixtureRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const FIXTURE_SPEC: &str = "# Spec
r[prose.only]
r[flake.only]
r[stub.only]
r[code.covered]
r[code.impl_only]
r[code.verify_only]
r[code.depends_related]
r[code.orphan]
r[code.disabled]
r[code.dead_include]
r[code.disabled_crate]
r[code.mounted]
r[code.included]
";

    #[test]
    fn prose_flake_stub_and_partial_markers_do_not_discharge_debt() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        // r[verify molten.project.inherited_tracey_debt.fixtures]
        let fixture = FixtureRoot::new("coverage", &[
            (".cairn/specs/fixture/spec.md", FIXTURE_SPEC),
            ("docs/guide.md", "@[impl prose.only] @[verify prose.only] @[related prose.only]\n"),
            ("flake.nix", "# @[impl flake.only]\n# @[verify flake.only]\n{ }\n"),
            (
                "tools/tracey/stub_proof.rs",
                "use std::fmt;\nfn main() {}\n// @[impl stub.only]\n// @[verify stub.only]\n",
            ),
            ("src/lib.rs", "// @[impl code.covered]\n// @[impl code.impl_only]\npub fn covered() {}\n"),
            ("tests/covered.rs", "// @[verify code.covered]\n// @[verify code.verify_only]\n#[test]\nfn t() {}\n"),
            (
                "crates/core/src/lib.rs",
                "// @[depends code.depends_related]\n// @[related code.depends_related]\npub fn core() {}\n",
            ),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        assert!(references.broken_citations.is_empty(), "{:?}", references.broken_citations);
        assert!(references.comment_only_stubs.is_empty(), "{:?}", references.comment_only_stubs);
        let requirements = read_requirements(&fixture.0).expect("fixture requirements");
        let coverage = classify(requirements, &set(&[]), &references);

        assert_eq!(coverage.covered, set(&["code.covered"]));
        assert_eq!(
            coverage.missing,
            set(&[
                "code.dead_include",
                "code.depends_related",
                "code.disabled",
                "code.disabled_crate",
                "code.impl_only",
                "code.included",
                "code.mounted",
                "code.orphan",
                "code.verify_only",
                "flake.only",
                "prose.only",
                "stub.only",
            ])
        );
        assert!(coverage.dangling.is_empty());
    }

    #[test]
    fn comment_only_rust_stubs_are_rejected() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        let fixture = FixtureRoot::new("stub", &[
            (".cairn/specs/fixture/spec.md", FIXTURE_SPEC),
            (
                "tools/tracey/stub.only_proof.rs",
                "//! Trace-only anchors.\n\n// @[impl stub.only]\n// @[verify stub.only]\n",
            ),
            ("tools/tracey/real_tool.rs", "// @[related stub.only]\nfn main() {}\n"),
            ("docs/notes.md", "// @[impl stub.only]\n"),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        assert_eq!(references.comment_only_stubs, set(&["tools/tracey/stub.only_proof.rs"]));
        let error = run(&["--root".to_string(), fixture.0.display().to_string()]).unwrap_err();
        assert!(error.contains("stubs must not hold markers"), "{error}");
    }

    #[test]
    fn only_module_tree_reachable_enabled_sources_supply_coverage() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        let pair = |id: &str| format!("// @[impl {id}]\n// @[verify {id}]\npub fn item() {{}}\n");
        let fixture = FixtureRoot::new("reachability", &[
            (".cairn/specs/fixture/spec.md", FIXTURE_SPEC),
            (
                "src/lib.rs",
                "#[cfg(any())]\nmod disabled;\nmod live;\nmod crate_off;\n#[path = \"mounted/real.rs\"]\npub mod \
                 mounted;\ninclude!(\"parts/body.rs\");\n#[cfg(any())]\ninclude!(\"parts/dead.rs\");\n",
            ),
            ("src/live.rs", &pair("code.covered")),
            ("src/disabled.rs", &pair("code.disabled")),
            ("src/orphan.rs", &pair("code.orphan")),
            ("src/mounted/real.rs", &pair("code.mounted")),
            ("src/parts/body.rs", &pair("code.included")),
            ("src/parts/dead.rs", &pair("code.dead_include")),
            ("src/crate_off.rs", &format!("#![cfg(any())]\n{}", pair("code.disabled_crate"))),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        let requirements = read_requirements(&fixture.0).expect("fixture requirements");
        let coverage = classify(requirements, &set(&[]), &references);

        assert_eq!(coverage.covered, set(&["code.covered", "code.included", "code.mounted"]));
        for rejected in [
            "code.orphan",
            "code.disabled",
            "code.dead_include",
            "code.disabled_crate",
        ] {
            assert!(coverage.missing.contains(rejected), "{rejected}");
        }
        assert_eq!(
            references.uncompiled_marker_files,
            set(&[
                "src/crate_off.rs",
                "src/disabled.rs",
                "src/orphan.rs",
                "src/parts/dead.rs"
            ])
        );
    }

    const SYNTAX_FIXTURE_SPEC: &str = "# Spec
r[code.live]
r[code.commented]
r[code.block_commented]
r[code.in_string]
r[code.raw_mod]
r[code.raw_include]
r[code.disabled_fn]
r[code.disabled_impl]
r[code.disabled_inner]
r[code.string_marker]
";

    const SYNTAX_FIXTURE_LIB: &str = r###"mod live;
// include!("commented.rs");
/* mod block_commented;
   include!("block_commented.rs"); */
const NOT_CODE: &str = "include!(\"in_string.rs\")";
const RAW: &str = r#"
mod raw_mod;
include!("raw.rs");
"#;
const MARKER_TEXT: &str = "// @[impl code.string_marker] @[verify code.string_marker]";
pub fn live_fn() {}
#[cfg(any())]
fn disabled_fn() {
    // @[impl code.disabled_fn]
    // @[verify code.disabled_fn]
}
#[cfg(FALSE)]
impl Disabled {
    // @[impl code.disabled_impl]
    // @[verify code.disabled_impl]
}
mod inner {
    #![cfg(not(all()))]
    // @[impl code.disabled_inner]
    // @[verify code.disabled_inner]
}
"###;

    #[test]
    fn discovery_and_markers_ignore_comments_literals_and_cfg_disabled_items() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        let pair = |id: &str| format!("// @[impl {id}]\n// @[verify {id}]\npub fn item() {{}}\n");
        let fixture = FixtureRoot::new("syntax", &[
            (".cairn/specs/fixture/spec.md", SYNTAX_FIXTURE_SPEC),
            ("src/lib.rs", SYNTAX_FIXTURE_LIB),
            ("src/live.rs", &pair("code.live")),
            ("src/commented.rs", &pair("code.commented")),
            ("src/block_commented.rs", &pair("code.block_commented")),
            ("src/in_string.rs", &pair("code.in_string")),
            ("src/raw_mod.rs", &pair("code.raw_mod")),
            ("src/raw.rs", &pair("code.raw_include")),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        let requirements = read_requirements(&fixture.0).expect("fixture requirements");
        let coverage = classify(requirements, &set(&[]), &references);

        assert_eq!(coverage.covered, set(&["code.live"]));
        assert_eq!(
            references.uncompiled_marker_files,
            set(&[
                "src/block_commented.rs",
                "src/commented.rs",
                "src/in_string.rs",
                "src/raw.rs",
                "src/raw_mod.rs",
            ])
        );
        for rejected in [
            "code.disabled_fn",
            "code.disabled_impl",
            "code.disabled_inner",
            "code.string_marker",
        ] {
            assert!(coverage.missing.contains(rejected), "{rejected}");
            assert!(references.all.contains(rejected), "{rejected} still resolves for dangling checks");
        }
    }

    const MACRO_FIXTURE_SPEC: &str = "# Spec
r[code.live]
r[code.leading]
r[code.unused_macro]
r[code.macro_part]
r[code.two_arm]
r[code.concat_macro]
r[code.macro_comment]
";

    const MACRO_FIXTURE_LIB: &str = r###"mod live;

// @[impl code.leading]
// @[verify code.leading]
#[cfg(any())]
fn leading() {}

macro_rules! unused_include {
    () => {
        include!("unused_macro.rs");
    };
}

macro_rules! unused_markers {
    () => {
        // @[impl code.macro_comment]
        // @[verify code.macro_comment]
        fn generated() {}
    };
}

macro_rules! include_part {
    ($path:literal) => {
        include!($path);
    };
}
include_part!("macro_part.rs");

macro_rules! two_arm {
    ($path:literal) => {
        include!($path);
    };
    (skip $path:literal) => {
        const _: &str = $path;
    };
}
two_arm!(skip "two_arm.rs");

macro_rules! include_concat {
    ($name:expr) => {
        include!(concat!("parts/", $name));
    };
}
include_concat!("concat.rs");
"###;

    #[test]
    fn leading_markers_macro_bodies_and_macro_includes_never_discharge_debt() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        let pair = |id: &str| format!("// @[impl {id}]\n// @[verify {id}]\npub fn item() {{}}\n");
        let fixture = FixtureRoot::new("macros", &[
            (".cairn/specs/fixture/spec.md", MACRO_FIXTURE_SPEC),
            ("src/lib.rs", MACRO_FIXTURE_LIB),
            ("src/live.rs", &pair("code.live")),
            ("src/unused_macro.rs", &pair("code.unused_macro")),
            ("src/macro_part.rs", &pair("code.macro_part")),
            ("src/two_arm.rs", &pair("code.two_arm")),
            ("src/parts/concat.rs", &pair("code.concat_macro")),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        let requirements = read_requirements(&fixture.0).expect("fixture requirements");
        let coverage = classify(requirements, &set(&[]), &references);

        assert_eq!(coverage.covered, set(&["code.live"]));
        assert_eq!(
            coverage.missing,
            set(&[
                "code.concat_macro",
                "code.leading",
                "code.macro_comment",
                "code.macro_part",
                "code.two_arm",
                "code.unused_macro",
            ])
        );
        assert_eq!(
            references.uncompiled_marker_files,
            set(&[
                "src/macro_part.rs",
                "src/parts/concat.rs",
                "src/two_arm.rs",
                "src/unused_macro.rs"
            ])
        );
    }

    const INVOCATION_FIXTURE_SPEC: &str = "# Spec
r[code.live]
r[code.manifest_part]
r[code.brace_include]
r[code.paren_include]
r[code.bracket_include]
r[code.brace_mod]
r[code.paren_mod]
r[code.nested_include]
";

    const INVOCATION_FIXTURE_LIB: &str = r###"mod live;
macro_rules! ignore {
    ($($tokens:tt)*) => {};
}
ignore! { include!("brace_include.rs"); }
ignore!(include!("paren_include.rs"));
ignore![include!("bracket_include.rs")];
ignore! {
    mod brace_mod;
}
ignore!(
    mod paren_mod;
);
ignore! {
    ignore! { include!("nested_include.rs"); }
}
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/manifest_part.rs"));
"###;

    #[test]
    fn mod_and_include_inside_macro_invocation_arguments_are_not_followed() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        let pair = |id: &str| format!("// @[impl {id}]\n// @[verify {id}]\npub fn item() {{}}\n");
        let fixture = FixtureRoot::new("invocations", &[
            (".cairn/specs/fixture/spec.md", INVOCATION_FIXTURE_SPEC),
            ("src/lib.rs", INVOCATION_FIXTURE_LIB),
            ("src/live.rs", &pair("code.live")),
            ("src/manifest_part.rs", &pair("code.manifest_part")),
            ("src/brace_include.rs", &pair("code.brace_include")),
            ("src/paren_include.rs", &pair("code.paren_include")),
            ("src/bracket_include.rs", &pair("code.bracket_include")),
            ("src/brace_mod.rs", &pair("code.brace_mod")),
            ("src/paren_mod.rs", &pair("code.paren_mod")),
            ("src/nested_include.rs", &pair("code.nested_include")),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        let requirements = read_requirements(&fixture.0).expect("fixture requirements");
        let coverage = classify(requirements, &set(&[]), &references);

        assert_eq!(coverage.covered, set(&["code.live", "code.manifest_part"]));
        assert_eq!(
            references.uncompiled_marker_files,
            set(&[
                "src/brace_include.rs",
                "src/brace_mod.rs",
                "src/bracket_include.rs",
                "src/nested_include.rs",
                "src/paren_include.rs",
                "src/paren_mod.rs",
            ])
        );
    }

    const QUALIFIED_FIXTURE_SPEC: &str = "# Spec
r[code.std_part]
r[code.core_part]
r[code.crate_orphan]
r[code.self_orphan]
r[code.other_orphan]
";

    #[test]
    fn only_std_or_core_qualified_include_is_followed_and_shadowing_fails_closed() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        let pair = |id: &str| format!("// @[impl {id}]\n// @[verify {id}]\npub fn item() {{}}\n");
        let fixture = FixtureRoot::new("qualified", &[
            (".cairn/specs/fixture/spec.md", QUALIFIED_FIXTURE_SPEC),
            (
                "src/lib.rs",
                "std::include!(\"std_part.rs\");\n::core::include!(\"core_part.rs\");\ncrate::include!(\"crate_orphan.rs\");\nself::include!(\"self_orphan.rs\");\nother::include!(\"other_orphan.rs\");\n",
            ),
            ("src/std_part.rs", &pair("code.std_part")),
            ("src/core_part.rs", &pair("code.core_part")),
            ("src/crate_orphan.rs", &pair("code.crate_orphan")),
            ("src/self_orphan.rs", &pair("code.self_orphan")),
            ("src/other_orphan.rs", &pair("code.other_orphan")),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        let requirements = read_requirements(&fixture.0).expect("fixture requirements");
        let coverage = classify(requirements, &set(&[]), &references);
        assert_eq!(coverage.covered, set(&["code.core_part", "code.std_part"]));
        assert_eq!(
            references.uncompiled_marker_files,
            set(&["src/crate_orphan.rs", "src/other_orphan.rs", "src/self_orphan.rs"])
        );
        assert!(references.shadowed_include_files.is_empty());

        let shadowed = FixtureRoot::new("shadowed", &[
            (".cairn/specs/fixture/spec.md", QUALIFIED_FIXTURE_SPEC),
            (
                "src/lib.rs",
                "#[macro_export]\nmacro_rules! include {\n    ($($tokens:tt)*) => {};\n}\ncrate::include!(\"crate_orphan.rs\");\n",
            ),
            ("src/crate_orphan.rs", &pair("code.crate_orphan")),
            ("tests/raw.rs", "macro_rules! r#include { () => {}; }\n"),
            ("tests/decl.rs", "pub macro include($path:expr) {}\n"),
        ]);
        let references = read_references(&shadowed.0).expect("shadowed references");
        assert_eq!(references.shadowed_include_files, set(&["src/lib.rs", "tests/decl.rs", "tests/raw.rs"]));
        let error = run(&["--root".to_string(), shadowed.0.display().to_string()]).unwrap_err();
        assert!(error.contains("shadows the built-in include!"), "{error}");
    }

    #[test]
    fn broken_file_citations_in_coverage_bearing_files_are_rejected() {
        // r[verify molten.testing.traceability_code_backed_coverage]
        let fixture = FixtureRoot::new("citation", &[
            (".cairn/specs/fixture/spec.md", FIXTURE_SPEC),
            ("src/lib.rs", "mod cites;\nmod present;\nmod uncited;\n"),
            ("src/present.rs", "pub fn present() {}\n"),
            (
                "src/cites.rs",
                "//! Implementation lives in `src/present.rs::present` and `src/testing/proof_trace.rs`.\n// @[impl code.covered]\n",
            ),
            ("tests/cites.rs", "// @[verify code.covered] see `src/present.rs:1`, `src/**/*.rs`, and `tests/`.\n"),
            ("src/uncited.rs", "//! Prose without markers may cite `src/never_written.rs`.\n"),
        ]);
        let references = read_references(&fixture.0).expect("fixture references");
        assert_eq!(references.broken_citations, set(&["src/cites.rs:1: cites missing src/testing/proof_trace.rs"]));
        let error = run(&["--root".to_string(), fixture.0.display().to_string()]).unwrap_err();
        assert!(error.contains("cite existing files"), "{error}");

        assert_eq!(file_citations("`src/a.rs::f`, `docs/b.md#x`. `src/*.rs` `plain` `tests/c.rs:12`"), vec![
            "src/a.rs".to_string(),
            "docs/b.md".to_string(),
            "tests/c.rs".to_string(),
        ]);
    }

    #[test]
    fn exact_baseline_passes() {
        // r[verify molten.project.inherited_tracey_debt.baseline]
        // r[verify molten.project.inherited_tracey_debt.validation]
        // r[verify molten.project.inherited_tracey_debt.fixtures]
        let coverage = classify(set(&["alpha", "beta"]), &set(&[]), &code_refs(&["alpha"]));
        let comparison = compare_baseline(&coverage.missing, &set(&["beta"]));
        assert!(comparison.unexpected_missing.is_empty());
        assert!(comparison.stale_baseline.is_empty());
        assert!(coverage.dangling.is_empty());
    }

    #[test]
    fn drift_and_dangling_fail() {
        // r[verify molten.project.inherited_tracey_debt.growth_denial]
        // r[verify molten.project.inherited_tracey_debt.fixtures]
        let coverage = classify(set(&["alpha", "beta"]), &set(&[]), &code_refs(&["alpha", "gamma"]));
        let comparison = compare_baseline(&coverage.missing, &set(&["alpha"]));
        assert_eq!(comparison.unexpected_missing, set(&["beta"]));
        assert_eq!(comparison.stale_baseline, set(&["alpha"]));
        assert_eq!(coverage.dangling, set(&["gamma"]));
    }

    #[test]
    fn malformed_markers_and_baselines_fail() {
        // r[verify molten.project.inherited_tracey_debt.fixtures]
        // r[verify molten.project.inherited_tracey_debt.marker_repair]
        assert_eq!(requirement_marker("r[valid.id] text"), Some("valid.id".to_string()));
        assert_eq!(requirement_marker("heading r[hidden.id]"), None);
        assert!(reference_markers("r[verify invalid/id]").is_empty());
        assert!(!baseline_is_sorted_and_unique(&["beta".to_string(), "alpha".to_string()]));
        assert!(!baseline_is_sorted_and_unique(&["alpha".to_string(), "alpha".to_string()]));
        assert!(!baseline_is_sorted_and_unique(&[String::new()]));
    }

    #[test]
    fn missing_requirement_root_fails_closed() {
        // r[verify molten.project.inherited_tracey_debt.growth_denial]
        let error = read_requirements(Path::new("/nonexistent/molten-tracey-root")).unwrap_err();
        assert!(error.contains("is missing"));
    }

    #[test]
    fn pending_resolution_preserves_accepted_debt_and_unknown_denial() {
        // r[verify molten.project.inherited_tracey_debt.active_delta_scope]
        // r[verify molten.project.inherited_tracey_debt.active_delta_validation]
        let coverage = classify(
            set(&["alpha", "beta"]),
            &set(&["gamma", "not_yet_implemented"]),
            &code_refs(&["alpha", "gamma", "unknown"]),
        );
        assert_eq!(coverage.missing, set(&["beta"]));
        assert_eq!(coverage.pending, set(&["gamma"]));
        assert_eq!(coverage.dangling, set(&["unknown"]));
        assert_eq!(coverage.requirements.intersection(&coverage.references).count(), 1);
        let comparison = compare_baseline(&coverage.missing, &set(&["beta"]));
        assert!(comparison.unexpected_missing.is_empty());
        assert!(comparison.stale_baseline.is_empty());
    }

    #[test]
    fn accepted_overlap_is_not_pending_and_does_not_mask_debt_growth() {
        // r[verify molten.project.inherited_tracey_debt.active_delta_scope]
        // r[verify molten.project.inherited_tracey_debt.growth_denial]
        let coverage = classify(set(&["alpha", "beta"]), &set(&["alpha", "beta"]), &code_refs(&["alpha"]));
        assert!(coverage.pending.is_empty());
        assert_eq!(coverage.missing, set(&["beta"]));
        assert_eq!(compare_baseline(&coverage.missing, &set(&[])).unexpected_missing, set(&["beta"]));
    }

    #[test]
    fn only_added_and_modified_requirement_sections_admit_pending_ids() {
        // r[verify molten.project.inherited_tracey_debt.active_delta_admission]
        let text = "r[outside]
## ADDED Requirements
r[added]
### Requirement: Added
r[added.scenario]
## REMOVED Requirements
r[removed]
## MODIFIED Requirements
r[modified]
## RENAMED Requirements
r[renamed]
# Appendix
r[appendix]
";
        assert_eq!(
            active_requirement_markers(text).collect::<BTreeSet<_>>(),
            set(&["added", "added.scenario", "modified"])
        );
    }

    #[test]
    fn pending_identity_becomes_dangling_when_its_delta_is_removed() {
        // r[verify molten.project.inherited_tracey_debt.active_delta_admission]
        let before = classify(set(&["accepted"]), &set(&["proposed"]), &code_refs(&["proposed"]));
        let after = classify(set(&["accepted"]), &set(&[]), &code_refs(&["proposed"]));
        assert_eq!(before.pending, set(&["proposed"]));
        assert!(before.dangling.is_empty());
        assert!(after.pending.is_empty());
        assert_eq!(after.dangling, set(&["proposed"]));
    }

    #[test]
    fn only_native_active_spec_paths_supply_pending_definitions() {
        // r[verify molten.project.inherited_tracey_debt.active_delta_admission]
        let root = Path::new(".cairn/changes");
        for path in [
            ".cairn/changes/change/specs/topic/spec.md",
            ".cairn/changes/change/specs/topic/nested/spec.md",
        ] {
            assert!(is_active_delta_spec(root, Path::new(path)), "{path}");
        }
        for path in [
            ".cairn/archive/change/specs/topic/spec.md",
            ".cairn/changes/change/proposal.md",
            ".cairn/changes/change/tasks.md",
            ".cairn/changes/change/evidence/topic/spec.md",
            ".cairn/changes/change/specs/topic/notes.md",
            ".cairn/changes/change/specs/spec.md",
        ] {
            assert!(!is_active_delta_spec(root, Path::new(path)), "{path}");
        }
    }
}
