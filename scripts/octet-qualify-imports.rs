#!/usr/bin/env -S nix shell nixpkgs#cargo nixpkgs#rustc nixpkgs#gcc --command cargo -q -Zscript
---
[dependencies]
proc-macro2 = { version = "=1.0.107", features = ["span-locations"] }
syn = { version = "=3.0.5", features = ["full", "visit"] }
---
// Octet `non_trait_imports` source repair.
//
// The Octet `non_trait_imports` lint rejects a private `use` of a concrete
// owner item because the import hides the owner path. The supported repair is
// the qualified owner path at every use site. This tool applies exactly that
// repair to the imports named by an Octet summary index:
//
//   1. Read the flagged `non_trait_imports` rows from an Octet summary.
//   2. For each flagged file, resolve the flagged lines back to their `use`
//      leaf and compute the owner path plus the locally bound name.
//   3. Delete the flagged leaf and rewrite every identifier reference to the
//      bound name with the qualified owner path, adjusting `super::` owners for
//      the inline-module depth of each use site.
//   4. Re-parse the result so a syntactically broken file is never written.
//
// The tool is deliberately conservative. A file is reported and skipped when
// the imports cannot be repaired by a textual, scope-local rewrite:
//
//   * a module parent that declares `mod child;` shares private imports with
//     children that write `use super::*;`;
//   * a `parts/**/body.rs` tree is spliced into one module with `include!`;
//   * the bound name is also bound by a local, parameter, item, field, generic
//     parameter, or a second `use` in the same file;
//   * the bound name is the anonymous `_` import;
//   * the leaf is an alias-free group member that shares a `use` group with an
//     unflagged leaf, or the leaf carries attributes.
//
// Compilation and tests stay the acceptance oracle for the files it rewrites.
//
// Usage:
//   scripts/octet-qualify-imports.rs --findings target/octet/summary.txt [--dry-run] [FILE...]
//   scripts/octet-qualify-imports.rs --self-test

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::Path as FsPath;

use syn::spanned::Spanned;
use syn::visit::Visit;

const LINT_NAME: &str = "non_trait_imports";
const SUMMARY_INDEX_PREFIX: &str = "  F";
const SELF_TEST_EXPECTED_EDITS: usize = 6;

/// Source geometry for one file: byte offset of every line start.
struct LineIndex {
    offsets: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut offsets = vec![0_usize];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                offsets.push(index + 1);
            }
        }
        Self { offsets }
    }

    fn offset(&self, position: proc_macro2::LineColumn) -> usize {
        self.offsets[position.line - 1] + position.column
    }

    fn range(&self, span: proc_macro2::Span) -> Range<usize> {
        self.offset(span.start())..self.offset(span.end())
    }
}

fn flatten_tree(prefix: &str, tree: &syn::UseTree, names: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            flatten_tree(&format!("{prefix}{}::", path.ident), &path.tree, names);
        }
        syn::UseTree::Name(name) => names.push(name.ident.to_string()),
        syn::UseTree::Rename(rename) => names.push(rename.rename.to_string()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                flatten_tree(prefix, item, names);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

/// Collect every bound name of a `use` item, in source order.
fn bound_names(tree: &syn::UseTree) -> Vec<String> {
    let mut names = Vec::new();
    flatten_tree("", tree, &mut names);
    names
}

/// Resolve the owner path and the locally bound name of the leaf named `target`.
fn resolve_leaf(
    prefix: &str,
    tree: &syn::UseTree,
    target: &str,
) -> Option<(String, String)> {
    match tree {
        syn::UseTree::Path(path) => {
            let next = format!("{prefix}{}::", path.ident);
            resolve_leaf(&next, &path.tree, target)
        }
        syn::UseTree::Name(name) => {
            let leaf = name.ident.to_string();
            if leaf == target {
                Some((format!("{prefix}{leaf}"), leaf))
            } else {
                None
            }
        }
        syn::UseTree::Rename(rename) => {
            let leaf = rename.ident.to_string();
            if rename.rename == target {
                Some((format!("{prefix}{leaf}"), rename.rename.to_string()))
            } else {
                None
            }
        }
        syn::UseTree::Glob(_) => None,
        syn::UseTree::Group(group) => {
            for item in &group.items {
                if let Some(found) = resolve_leaf(prefix, item, target) {
                    return Some(found);
                }
            }
            None
        }
    }
}

/// The flagged leaf's own span, used to delete one leaf from a `use` group.
fn leaf_span<'ast>(tree: &'ast syn::UseTree, target: &str) -> Option<syn::UseTree> {
    match tree {
        syn::UseTree::Path(path) => leaf_span(&path.tree, target),
        syn::UseTree::Name(name) if name.ident == target => Some(tree.clone()),
        syn::UseTree::Rename(rename) if rename.rename == target => Some(tree.clone()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                if let Some(found) = leaf_span(item, target) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn is_group_tree(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Group(_) => true,
        syn::UseTree::Path(path) => is_group_tree(&path.tree),
        _ => false,
    }
}

/// Collect every identifier the file binds outside its own `use` items.
///
/// A binding that matches a bound import name would make the token rewrite
/// change an unrelated name, so any collision skips the file.
#[derive(Default)]
struct Bindings {
    names: BTreeSet<String>,
}

impl Bindings {
    fn pattern(&mut self, pat: &syn::Pat) {
        let mut collector = PatternNames::default();
        collector.visit_pat(pat);
        self.names.extend(collector.names);
    }
}

#[derive(Default)]
struct PatternNames {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PatternNames {
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.names.insert(node.ident.to_string());
        syn::visit::visit_pat_ident(self, node);
    }
    fn visit_pat_struct(&mut self, node: &'ast syn::PatStruct) {
        for field in &node.fields {
            if let syn::Member::Named(name) = &field.member {
                if field.colon_token.is_none() {
                    self.names.insert(name.to_string());
                }
            }
        }
        syn::visit::visit_pat_struct(self, node);
    }
}

struct FileBindings<'a> {
    bindings: Bindings,
    extra_use_bindings: BTreeSet<String>,
    lines: &'a LineIndex,
    flagged_ranges: &'a [Range<usize>],
}

impl<'ast> Visit<'ast> for FileBindings<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        match node {
            syn::Item::Fn(item) => {
                self.bindings.names.insert(item.sig.ident.to_string());
            }
            syn::Item::Struct(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Enum(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Union(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Trait(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Type(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Const(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Static(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Mod(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Macro(item) => {
                if let Some(ident) = &item.ident {
                    self.bindings.names.insert(ident.to_string());
                }
            }
            syn::Item::ExternCrate(item) => {
                self.bindings.names.insert(item.ident.to_string());
            }
            syn::Item::Use(item) => {
                let range = self.lines.range(item.span());
                if !self
                    .flagged_ranges
                    .iter()
                    .any(|flagged| flagged.contains(&range.start))
                {
                    for name in bound_names(&item.tree) {
                        self.extra_use_bindings.insert(name);
                    }
                }
            }
            _ => {}
        }
        syn::visit::visit_item(self, node);
    }
    fn visit_variant(&mut self, node: &'ast syn::Variant) {
        self.bindings.names.insert(node.ident.to_string());
        syn::visit::visit_variant(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.bindings.names.insert(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.bindings.names.insert(node.sig.ident.to_string());
        syn::visit::visit_trait_item_fn(self, node);
    }
    fn visit_field(&mut self, node: &'ast syn::Field) {
        if let Some(ident) = &node.ident {
            self.bindings.names.insert(ident.to_string());
        }
        syn::visit::visit_field(self, node);
    }
    fn visit_generic_param(&mut self, node: &'ast syn::GenericParam) {
        match node {
            syn::GenericParam::Type(param) => {
                self.bindings.names.insert(param.ident.to_string());
            }
            syn::GenericParam::Const(param) => {
                self.bindings.names.insert(param.ident.to_string());
            }
            syn::GenericParam::Lifetime(_) => {}
        }
        syn::visit::visit_generic_param(self, node);
    }
    fn visit_fn_arg(&mut self, node: &'ast syn::FnArg) {
        if let syn::FnArg::Typed(typed) = node {
            self.bindings.pattern(&typed.pat);
        }
        syn::visit::visit_fn_arg(self, node);
    }
    fn visit_local(&mut self, node: &'ast syn::Local) {
        self.bindings.pattern(&node.pat);
        syn::visit::visit_local(self, node);
    }
    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        self.bindings.pattern(&node.pat);
        syn::visit::visit_arm(self, node);
    }
    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        for input in &node.inputs {
            self.bindings.pattern(input);
        }
        syn::visit::visit_expr_closure(self, node);
    }
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.bindings.pattern(&node.pat);
        syn::visit::visit_expr_for_loop(self, node);
    }
    fn visit_field_value(&mut self, node: &'ast syn::FieldValue) {
        if node.colon_token.is_none() {
            if let syn::Member::Named(name) = &node.member {
                self.bindings.names.insert(name.to_string());
            }
        }
        syn::visit::visit_field_value(self, node);
    }
}

/// Byte ranges of every inline `mod name { ... }` body in the file.
struct InlineModules<'a> {
    ranges: Vec<Range<usize>>,
    lines: &'a LineIndex,
}

impl<'ast> Visit<'ast> for InlineModules<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if node.content.is_some() {
            self.ranges.push(self.lines.range(node.span()));
        }
        syn::visit::visit_item_mod(self, node);
    }
}

/// True when `text` mentions `name` as a standalone identifier.
fn mentions_identifier(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = name.as_bytes();
    if needle.is_empty() || needle.len() > bytes.len() {
        return false;
    }
    let mut index = 0_usize;
    while let Some(found) = text[index..].find(name) {
        let start = index + found;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_identifier_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_identifier_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        index = end;
    }
    false
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Collect every `.rs` file under `directory`, excluding `exclude`.
fn rust_files(directory: &FsPath, exclude: &FsPath) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    let mut children: Vec<std::path::PathBuf> =
        entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    children.sort();
    for child in children {
        if child.is_dir() {
            found.extend(rust_files(&child, exclude));
        } else if child.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && child != exclude
        {
            found.push(child);
        }
    }
    found
}

/// The directory that holds a module's child modules.
fn module_directory(path: &str) -> Option<std::path::PathBuf> {
    let file = FsPath::new(path);
    let stem = file.file_stem()?.to_str()?;
    let parent = file.parent()?;
    if stem == "mod" {
        Some(parent.to_path_buf())
    } else {
        Some(parent.join(stem))
    }
}

/// Detect scopes that the token rewrite cannot repair safely.
///
/// A module parent shares its private imports with any descendant file that
/// writes `use super::*;` or that reaches the name through `super::`. A
/// `parts/**/body.rs` tree is spliced into one module with `include!`, so each
/// part sees the imports of its siblings. Removing an import from such a file
/// can break a file the summary never flagged, so the module tree and the part
/// tree are scanned for the bound names first.
/// One descendant module file and its distance below the module parent.
struct Descendant {
    path: String,
    depth: usize,
}

/// Collect the module files below a module parent.
///
/// `None` means the file is not a module parent, so no child shares its scope.
/// `Some(Err(..))` means a child shares the scope in a way this tool does not
/// rewrite.
fn module_descendants(path: &str, syntax: &syn::File) -> Option<Result<Vec<Descendant>, String>> {
    let mut declares_mod = false;
    for item in &syntax.items {
        match item {
            syn::Item::Mod(item_mod) if item_mod.content.is_none() => {
                declares_mod = true;
                if item_mod
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("path"))
                {
                    return Some(Err(format!("`#[path]` module `{}`", item_mod.ident)));
                }
            }
            syn::Item::Macro(item_macro)
                if item_macro
                    .mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "include") =>
            {
                return Some(Err(String::from("include! splice shares one module scope")));
            }
            _ => {}
        }
    }
    if !declares_mod {
        return None;
    }
    let directory = module_directory(path)?;
    if !directory.is_dir() {
        return None;
    }
    let parent = FsPath::new(path);
    let mut found = Vec::new();
    collect_descendants(&directory, &directory, parent, &mut found);
    found.sort_by(|left, right| left.path.cmp(&right.path));
    Some(Ok(found))
}

fn collect_descendants(
    root: &FsPath,
    directory: &FsPath,
    exclude: &FsPath,
    found: &mut Vec<Descendant>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut children: Vec<std::path::PathBuf> =
        entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    children.sort();
    for child in children {
        if child.is_dir() {
            collect_descendants(root, &child, exclude, found);
            continue;
        }
        if child.extension().and_then(|extension| extension.to_str()) != Some("rs") || child == exclude
        {
            continue;
        }
        let Ok(relative) = child.strip_prefix(root) else {
            continue;
        };
        let components = relative.components().count();
        let is_mod = child.file_name().is_some_and(|name| name == "mod.rs");
        let depth = if is_mod {
            components.saturating_sub(1)
        } else {
            components
        };
        if depth == 0 {
            continue;
        }
        found.push(Descendant {
            path: child.to_string_lossy().into_owned(),
            depth,
        });
    }
}

/// Detect the include! splice, which shares one scope across sibling parts.
fn parts_hazard(path: &str, syntax: &syn::File, owners: &BTreeMap<String, String>) -> Option<String> {
    if !path.contains("/parts/") {
        return None;
    }
    let file = FsPath::new(path);
    let mut current = file.parent();
    while let Some(directory) = current {
        if directory.file_name().is_some_and(|name| name == "parts") {
            return scan_for_names(directory, file, owners, "part body shares one module scope");
        }
        current = directory.parent();
    }
    let _ = syntax;
    Some(String::from("include! part body shares one module scope"))
}

/// Report the first sibling file that states any bound name, if any.
fn scan_for_names(
    directory: &FsPath,
    exclude: &FsPath,
    owners: &BTreeMap<String, String>,
    reason: &str,
) -> Option<String> {
    for candidate in rust_files(directory, exclude) {
        let Ok(text) = std::fs::read_to_string(&candidate) else {
            return Some(format!("{reason}: unreadable {}", candidate.display()));
        };
        for name in owners.keys() {
            if mentions_identifier(&text, name) {
                return Some(format!("{reason}: `{name}` in {}", candidate.display()));
            }
        }
    }
    None
}

/// True when the identifier at `end` is followed by a field colon.
fn followed_by_colon(source: &str, end: usize) -> bool {
    let rest = source.get(end..).unwrap_or_default().trim_start();
    rest.starts_with(':') && !rest.starts_with("::")
}

/// Rewrite the bare references to a bound name inside one descendant module.
///
/// A qualified reference such as `super::Name` reaches the parent through a
/// path this tool does not rewrite, and a local binding of the same name hides
/// the parent import, so both cases are reported instead of rewritten.
fn rewrite_descendant(
    path: &str,
    depth: usize,
    source: &str,
    owners: &BTreeMap<String, String>,
) -> Result<(String, usize), String> {
    let syntax = syn::parse_file(source).map_err(|error| format!("parse failed: {error}"))?;
    let lines = LineIndex::new(source);
    let mut bindings = FileBindings {
        bindings: Bindings::default(),
        extra_use_bindings: BTreeSet::new(),
        lines: &lines,
        flagged_ranges: &[],
    };
    bindings.visit_file(&syntax);
    for name in owners.keys() {
        if bindings.bindings.names.contains(name) {
            return Err(format!("descendant {path} binds `{name}` locally"));
        }
        if bindings.extra_use_bindings.contains(name) {
            return Err(format!("descendant {path} imports `{name}` again"));
        }
    }
    let mut modules = InlineModules {
        ranges: Vec::new(),
        lines: &lines,
    };
    modules.visit_file(&syntax);

    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let tokens: proc_macro2::TokenStream = source
        .parse()
        .map_err(|error| format!("tokenize failed: {error}"))?;
    collect_descendant_edits(
        tokens,
        owners,
        &modules.ranges,
        &lines,
        source,
        depth,
        &mut edits,
    )?;
    if edits.is_empty() {
        return Ok((source.to_string(), 0));
    }
    edits.sort_by_key(|(range, _)| range.start);
    let edit_count = edits.len();
    let mut output = source.to_string();
    for (range, replacement) in edits.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    syn::parse_file(&output).map_err(|error| format!("rewrite produced invalid syntax: {error}"))?;
    Ok((output, edit_count))
}

#[allow(clippy::too_many_arguments)]
fn collect_descendant_edits(
    stream: proc_macro2::TokenStream,
    owners: &BTreeMap<String, String>,
    module_ranges: &[Range<usize>],
    lines: &LineIndex,
    source: &str,
    depth: usize,
    edits: &mut Vec<(Range<usize>, String)>,
) -> Result<(), String> {
    let mut previous = String::new();
    for token in stream {
        match token {
            proc_macro2::TokenTree::Group(group) => {
                collect_descendant_edits(
                    group.stream(),
                    owners,
                    module_ranges,
                    lines,
                    source,
                    depth,
                    edits,
                )?;
                previous.clear();
            }
            proc_macro2::TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let range = lines.range(ident.span());
                if let Some(owner) = owners.get(&name) {
                    if previous == "::" {
                        return Err(format!("qualified reference to `{name}` needs a manual path"));
                    }
                    let hidden = previous == "::"
                        || previous == "."
                        || is_macro_name(source, range.end)
                        || followed_by_colon(source, range.end);
                    if !hidden {
                        let depth = depth + module_depth(module_ranges, range.start);
                        edits.push((range, qualify_owner(owner, depth)));
                    }
                }
                previous = name;
            }
            proc_macro2::TokenTree::Punct(punct) => {
                previous = if punct.as_char() == ':' && previous == ":" {
                    String::from("::")
                } else {
                    punct.as_char().to_string()
                };
            }
            proc_macro2::TokenTree::Literal(_) => previous.clear(),
        }
    }
    Ok(())
}

/// True when the identifier at `end` is a macro invocation name.
fn is_macro_name(source: &str, end: usize) -> bool {
    let rest = source.get(end..).unwrap_or_default();
    let mut characters = rest.chars();
    if characters.next() != Some('!') {
        return false;
    }
    characters.next() != Some('=')
}

/// Parse `  F123   non_trait_imports   crate   path:line` index rows.
fn read_flagged_lines(summary: &str) -> BTreeMap<String, Vec<usize>> {
    let mut flagged: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for line in summary.lines() {
        if !line.starts_with(SUMMARY_INDEX_PREFIX) {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 || fields[1] != LINT_NAME {
            continue;
        }
        let location = fields[3];
        let Some((path, number)) = location.rsplit_once(':') else {
            continue;
        };
        let Ok(line_number) = number.parse::<usize>() else {
            continue;
        };
        let entry = flagged.entry(path.to_owned()).or_default();
        if !entry.contains(&line_number) {
            entry.push(line_number);
        }
    }
    flagged
}

/// Extend a whole-item removal across its whole source lines.
fn line_deletion_range(source: &str, item: Range<usize>) -> Range<usize> {
    let bytes = source.as_bytes();
    let mut start = item.start;
    while start > 0 && bytes[start - 1] != b'\n' {
        start -= 1;
    }
    if source[start..item.start]
        .bytes()
        .any(|byte| byte != b' ' && byte != b'\t')
    {
        start = item.start;
    }
    let mut end = item.end;
    while end < bytes.len() && (bytes[end] == b' ' || bytes[end] == b'\t' || bytes[end] == b'\r') {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    }
    start..end
}

/// Extend a leaf range across a following comma so the group stays valid.
fn leaf_deletion_range(source: &str, leaf: Range<usize>) -> Range<usize> {
    let tail = source[leaf.end..].trim_start();
    match tail.strip_prefix(',') {
        Some(_) => leaf.start..leaf.end + source[leaf.end..].len() - tail.len() + 1,
        None => leaf,
    }
}

/// One repaired file plus the descendant modules that moved with it.
struct Repair {
    output: String,
    edits: usize,
    descendants: Vec<(String, String, usize)>,
}

/// Rewrite flagged imports in `source`, returning the new text and edit count.
fn qualify_source(
    path: &str,
    source: &str,
    flagged_lines: &[usize],
) -> Result<Repair, String> {
    let syntax = syn::parse_file(source).map_err(|error| format!("parse failed: {error}"))?;
    let lines = LineIndex::new(source);
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    let mut removals: Vec<Range<usize>> = Vec::new();
    let mut flagged_ranges: Vec<Range<usize>> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for item in &syntax.items {
        let syn::Item::Use(item_use) = item else {
            continue;
        };
        let item_range = lines.range(item_use.span());
        let start_line = item_use.span().start().line;
        if !flagged_lines.contains(&start_line) {
            continue;
        }
        if !item_use.attrs.is_empty() {
            skipped.push(format!("attributes on line {start_line}"));
            continue;
        }
        if !matches!(item_use.vis, syn::Visibility::Inherited) {
            skipped.push(format!("public import on line {start_line}"));
            continue;
        }
        if matches!(item_use.tree, syn::UseTree::Glob(_)) {
            skipped.push(format!("glob import on line {start_line}"));
            continue;
        }
        let names = bound_names(&item_use.tree);
        let is_group = is_group_tree(&item_use.tree);
        if is_group && names.len() != 1 {
            skipped.push(format!("shared use group on line {start_line}"));
            continue;
        }
        let colon = if item_use.leading_colon.is_some() {
            "::"
        } else {
            ""
        };
        let mut planned: Vec<String> = Vec::new();
        for target in &names {
            if target == "_" {
                skipped.push(format!("anonymous import on line {start_line}"));
                continue;
            }
            let Some((owner, bound)) = resolve_leaf(colon, &item_use.tree, target) else {
                skipped.push(format!("unresolved leaf `{target}` on line {start_line}"));
                continue;
            };
            if let Some(existing) = owners.get(&bound) {
                if existing != &owner {
                    skipped.push(format!("ambiguous owner for `{bound}` on line {start_line}"));
                    continue;
                }
            }
            owners.insert(bound.clone(), owner);
            planned.push(bound);
        }
        if planned.is_empty() {
            continue;
        }
        if is_group {
            let Some(leaf) = leaf_span(&item_use.tree, &planned[0]) else {
                skipped.push(format!("unresolved group leaf on line {start_line}"));
                continue;
            };
            removals.push(leaf_deletion_range(source, lines.range(leaf.span())));
        } else {
            removals.push(line_deletion_range(source, item_range.clone()));
        }
        flagged_ranges.push(item_range);
    }

    if !skipped.is_empty() {
        return Err(format!("manual repair required: {}", skipped.join("; ")));
    }
    if owners.is_empty() {
        return Ok(Repair {
            output: source.to_string(),
            edits: 0,
            descendants: Vec::new(),
        });
    }

    let mut file_bindings = FileBindings {
        bindings: Bindings::default(),
        extra_use_bindings: BTreeSet::new(),
        lines: &lines,
        flagged_ranges: &flagged_ranges,
    };
    file_bindings.visit_file(&syntax);
    for name in owners.keys() {
        if file_bindings.bindings.names.contains(name) {
            skipped.push(format!("local binding shadows `{name}`"));
        }
        if file_bindings.extra_use_bindings.contains(name) {
            skipped.push(format!("second `use` binds `{name}`"));
        }
    }
    if !skipped.is_empty() {
        return Err(format!("manual repair required: {}", skipped.join("; ")));
    }

    let mut modules = InlineModules {
        ranges: Vec::new(),
        lines: &lines,
    };
    modules.visit_file(&syntax);
    if let Some(hazard) = parts_hazard(path, &syntax, &owners) {
        return Err(format!("manual repair required: {hazard}"));
    }
    let mut descendants: Vec<(String, String, usize)> = Vec::new();
    if let Some(children) = module_descendants(path, &syntax) {
        for child in children? {
            let text = std::fs::read_to_string(&child.path)
                .map_err(|error| format!("cannot read {}: {error}", child.path))?;
            match rewrite_descendant(&child.path, child.depth, &text, &owners) {
                Ok((output, edits)) if edits > 0 => {
                    descendants.push((child.path.clone(), output, edits));
                }
                Ok(_) => {}
                Err(message) => return Err(format!("manual repair required: {message}")),
            }
        }
    }
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut manual: Vec<String> = Vec::new();
    let tokens: proc_macro2::TokenStream = source
        .parse()
        .map_err(|error| format!("tokenize failed: {error}"))?;
    collect_reference_edits(
        tokens.clone(),
        &owners,
        &flagged_ranges,
        &modules.ranges,
        &lines,
        &mut edits,
        &mut manual,
    );
    inline_format_references(tokens, &owners, &mut manual);
    if !manual.is_empty() {
        manual.sort();
        manual.dedup();
        return Err(format!("manual repair required: {}", manual.join("; ")));
    }
    for range in removals {
        edits.push((range, String::new()));
    }
    edits.sort_by_key(|(range, _)| range.start);
    for pair in edits.windows(2) {
        if pair[0].0.end > pair[1].0.start {
            return Err(format!(
                "overlapping edits at bytes {} and {}",
                pair[0].0.start, pair[1].0.start
            ));
        }
    }

    let edit_count = edits.len();
    let mut output = source.to_string();
    for (range, replacement) in edits.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    syn::parse_file(&output).map_err(|error| format!("rewrite produced invalid syntax: {error}"))?;
    Ok(Repair {
        output,
        edits: edit_count,
        descendants,
    })
}

/// Adjust an owner path for the inline-module depth of one use site.
///
/// A `super::` owner was written relative to the file module, so a use site
/// inside `depth` inline modules needs one more `super::` step per level.
/// `crate::` and `::` owners are already absolute.
fn qualify_owner(owner: &str, depth: usize) -> String {
    match owner.strip_prefix("super::") {
        Some(rest) if depth > 0 => {
            let mut prefix = String::new();
            for _ in 0..=depth {
                prefix.push_str("super::");
            }
            format!("{prefix}{rest}")
        }
        _ => owner.to_owned(),
    }
}

fn module_depth(ranges: &[Range<usize>], offset: usize) -> usize {
    ranges.iter().filter(|range| range.contains(&offset)).count()
}

/// Record one edit per reference identifier, skipping import text and paths.
#[allow(clippy::too_many_arguments)]
fn collect_reference_edits(
    stream: proc_macro2::TokenStream,
    owners: &BTreeMap<String, String>,
    flagged_ranges: &[Range<usize>],
    module_ranges: &[Range<usize>],
    lines: &LineIndex,
    edits: &mut Vec<(Range<usize>, String)>,
    manual: &mut Vec<String>,
) {
    let mut previous = String::new();
    for token in stream {
        match token {
            proc_macro2::TokenTree::Group(group) => {
                collect_reference_edits(
                    group.stream(),
                    owners,
                    flagged_ranges,
                    module_ranges,
                    lines,
                    edits,
                    manual,
                );
                previous.clear();
            }
            proc_macro2::TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let start = lines.offset(ident.span().start());
                let range = lines.range(ident.span());
                let in_import = flagged_ranges.iter().any(|item| item.contains(&start));
                let is_qualified = previous == "::" || previous == ".";
                if !in_import && !is_qualified {
                    if let Some(owner) = owners.get(&name) {
                        let depth = module_depth(module_ranges, start);
                        if owner.starts_with("self::") && depth > 0 {
                            manual.push(format!("`self::` owner for `{name}` inside a nested module"));
                        } else {
                            edits.push((range, qualify_owner(owner, depth)));
                        }
                    }
                }
                previous = name;
            }
            proc_macro2::TokenTree::Punct(punct) => {
                previous = if punct.as_char() == ':' && previous == ":" {
                    String::from("::")
                } else {
                    punct.as_char().to_string()
                };
            }
            proc_macro2::TokenTree::Literal(_) => previous.clear(),
        }
    }
}

/// Detect inline format arguments such as `format!("{NAME}")`.
///
/// The identifier lives inside a string literal, so the token rewrite cannot
/// qualify it. Removing the import would break the call, so the file is
/// reported for manual repair instead of being rewritten silently.
fn inline_format_references(
    stream: proc_macro2::TokenStream,
    owners: &BTreeMap<String, String>,
    manual: &mut Vec<String>,
) {
    for token in stream {
        match token {
            proc_macro2::TokenTree::Group(group) => {
                inline_format_references(group.stream(), owners, manual);
            }
            proc_macro2::TokenTree::Literal(literal) => {
                let text = literal.to_string();
                if !text.contains('{') {
                    continue;
                }
                for name in owners.keys() {
                    let open = format!("{{{name}}}");
                    let spec = format!("{{{name}:");
                    if text.contains(&open) || text.contains(&spec) {
                        manual.push(format!("inline format argument `{name}`"));
                    }
                }
            }
            _ => {}
        }
    }
}

fn run_summary(summary_path: &str, dry_run: bool, only: &[String]) -> Result<usize, String> {
    let summary = std::fs::read_to_string(summary_path)
        .map_err(|error| format!("cannot read {summary_path}: {error}"))?;
    let flagged = read_flagged_lines(&summary);
    let mut total_edits = 0_usize;
    let mut repaired = 0_usize;
    let mut failed = 0_usize;
    for (path, lines) in &flagged {
        if !only.is_empty() && !only.iter().any(|wanted| wanted == path) {
            continue;
        }
        if !path.ends_with(".rs") || !FsPath::new(path).exists() {
            continue;
        }
        let source =
            std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
        match qualify_source(path, &source, lines) {
            Ok(repair) if repair.edits == 0 && repair.descendants.is_empty() => {}
            Ok(repair) if dry_run => {
                let descendant_edits: usize = repair.descendants.iter().map(|(_, _, edits)| edits).sum();
                println!(
                    "{path}: {} edits, {} descendant files (dry run)",
                    repair.edits,
                    repair.descendants.len()
                );
                total_edits += repair.edits + descendant_edits;
                repaired += 1;
            }
            Ok(repair) => {
                std::fs::write(path, repair.output)
                    .map_err(|error| format!("cannot write {path}: {error}"))?;
                let mut descendant_edits = 0_usize;
                for (child, output, edits) in &repair.descendants {
                    std::fs::write(child, output)
                        .map_err(|error| format!("cannot write {child}: {error}"))?;
                    descendant_edits += edits;
                }
                println!(
                    "{path}: {} edits, {} descendant files",
                    repair.edits,
                    repair.descendants.len()
                );
                total_edits += repair.edits + descendant_edits;
                repaired += 1;
            }
            Err(message) => {
                eprintln!("{path}: SKIP ({message})");
                failed += 1;
            }
        }
    }
    println!("files repaired: {repaired}, edits: {total_edits}, skipped: {failed}");
    Ok(total_edits)
}

fn self_test() -> Result<(), String> {
    let source = r#"use std::collections::BTreeMap;
use crate::addr::Addr;
use crate::addr::Route as Path;
use crate::other::Unflagged;

pub struct Holder {
    table: BTreeMap<String, Addr>,
}

impl Holder {
    fn route(&self, addr: &Addr) -> Path {
        let qualified = crate::addr::Addr;
        let _ = qualified;
        Path::new()
    }

    fn untouched(&self) -> Unflagged {
        Unflagged
    }
}
"#;
    let repair = qualify_source("src/addr/route.rs", source, &[2, 3])?;
    if repair.edits != SELF_TEST_EXPECTED_EDITS {
        return Err(format!(
            "self test expected {SELF_TEST_EXPECTED_EDITS} edits, got {}",
            repair.edits
        ));
    }
    let output = repair.output;
    if output.contains("use crate::addr::Addr;") || output.contains("use crate::addr::Route as Path;")
    {
        return Err(String::from("self test kept a flagged import"));
    }
    if !output.contains("use std::collections::BTreeMap;") {
        return Err(String::from("self test removed an unflagged import"));
    }
    if !output.contains("use crate::other::Unflagged;") {
        return Err(String::from("self test removed an unflagged import"));
    }
    let expected = [
        "table: BTreeMap<String, crate::addr::Addr>",
        "addr: &crate::addr::Addr",
        "-> crate::addr::Route",
        "crate::addr::Route::new()",
    ];
    for fragment in expected {
        if !output.contains(fragment) {
            return Err(format!("self test missing `{fragment}` in\n{output}"));
        }
    }
    if !output.contains("let qualified = crate::addr::Addr;") {
        return Err(String::from("self test rewrote an already qualified path"));
    }

    let nested = "use super::Addr;\n\nmod inner {\n    fn f(value: Addr) -> Addr {\n        value\n    }\n}\n";
    let output = qualify_source("src/addr/route.rs", nested, &[1])?.output;
    if !output.contains("fn f(value: super::super::Addr) -> super::super::Addr") {
        return Err(format!("self test did not adjust inline-module depth in\n{output}"));
    }

    let absolute = "use ::syndicate::bag::BTreeBag;\n\nfn f() -> BTreeBag {\n    BTreeBag::new()\n}\n";
    let output = qualify_source("src/addr/route.rs", absolute, &[1])?.output;
    if !output.contains("-> ::syndicate::bag::BTreeBag") {
        return Err(format!("self test lost the absolute path in\n{output}"));
    }

    let negative = "use crate::addr::Addr;\nuse crate::other::Addr;\n";
    if qualify_source("src/addr/route.rs", negative, &[1, 2]).is_ok() {
        return Err(String::from("self test accepted two owners for one name"));
    }
    let group = "use crate::addr::{Addr, Route};\n";
    if qualify_source("src/addr/route.rs", group, &[1]).is_ok() {
        return Err(String::from("self test accepted a partially flagged group"));
    }
    let parent = "use crate::addr::Addr;\nmod child;\n";
    let output = qualify_source("src/addr/route.rs", parent, &[1])?.output;
    if output.contains("use crate::addr::Addr;") {
        return Err(String::from("self test kept a parent import without a sharing child"));
    }
    // The effects part tree is spliced into one module, so a sibling that
    // mentions the bound name must block the rewrite.
    let part = "use crate::codec::canonical_hash;\n";
    if qualify_source("src/effects/parts/mod/p999/body.rs", part, &[1]).is_ok() {
        return Err(String::from("self test accepted an include! part body"));
    }
    let isolated_part = "use crate::codec::unused_owner_item;\n";
    let output = qualify_source("src/effects/parts/mod/p999/body.rs", isolated_part, &[1])?.output;
    if output.contains("use crate::codec::unused_owner_item;") {
        return Err(String::from("self test skipped an isolated part body"));
    }

    // A descendant module sees the parent import through `use super::*`, so
    // the repair has to qualify the descendant reference with one more step.
    let mut owners = BTreeMap::new();
    owners.insert(String::from("Addr"), String::from("super::Addr"));
    let child = "use super::*;\n\nfn f(value: Addr) -> Addr {\n    value\n}\n";
    let (output, edits) = rewrite_descendant("src/addr/route/child.rs", 1, child, &owners)?;
    if edits != 2 || !output.contains("fn f(value: super::super::Addr) -> super::super::Addr") {
        return Err(format!("self test did not qualify a descendant in\n{output}"));
    }
    let grandchild = "use super::*;\n\nfn f(value: Addr) -> Addr {\n    value\n}\n";
    let (output, _edits) = rewrite_descendant("src/addr/route/child/grand.rs", 2, grandchild, &owners)?;
    if !output.contains("-> super::super::super::Addr") {
        return Err(format!("self test used the wrong descendant depth in\n{output}"));
    }
    let qualified = "fn f(value: super::Addr) -> usize {\n    let _ = value;\n    0\n}\n";
    if rewrite_descendant("src/addr/route/child.rs", 1, qualified, &owners).is_ok() {
        return Err(String::from("self test accepted a qualified descendant reference"));
    }
    let shadowed_child = "fn f() {\n    let Addr = 1;\n    let _ = Addr;\n}\n";
    if rewrite_descendant("src/addr/route/child.rs", 1, shadowed_child, &owners).is_ok() {
        return Err(String::from("self test accepted a shadowing descendant binding"));
    }
    let shadowed = "use crate::addr::Addr;\n\nfn f() {\n    let Addr = 1;\n    let _ = Addr;\n}\n";
    if qualify_source("src/addr/route.rs", shadowed, &[1]).is_ok() {
        return Err(String::from("self test accepted a shadowing binding"));
    }
    let anonymous = "use std::hash::Hash as _;\nuse std::hash::Hasher;\n";
    if qualify_source("src/addr/route.rs", anonymous, &[1]).is_ok() {
        return Err(String::from("self test accepted an anonymous import"));
    }
    let formatted = "use crate::addr::Addr;\n\nfn f(value: Addr) -> String {\n    format!(\"{Addr}\")\n}\n";
    if qualify_source("src/addr/route.rs", formatted, &[1]).is_ok() {
        return Err(String::from("self test accepted an inline format argument"));
    }
    println!("self test passed");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let mut summary: Option<String> = None;
    let mut dry_run = false;
    let mut self_check = false;
    let mut only: Vec<String> = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--findings" => {
                summary = Some(arguments.next().ok_or("missing value after --findings")?);
            }
            "--dry-run" => dry_run = true,
            "--self-test" => self_check = true,
            other if other.starts_with("--") => return Err(format!("unknown flag {other}").into()),
            other => only.push(other.to_owned()),
        }
    }
    if self_check {
        return self_test().map_err(Into::into);
    }
    let summary = summary.ok_or("missing --findings <octet summary path>")?;
    run_summary(&summary, dry_run, &only)?;
    Ok(())
}
