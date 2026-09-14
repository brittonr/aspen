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
/// One name that can hide an import, with the source range where it hides it.
struct Scope {
    name: String,
    /// The range in which the name resolves to this binding.
    range: Range<usize>,
    /// Where the binding starts to hide the import.
    offset: usize,
    /// The binding's own span, which is never rewritten.
    declaration: Range<usize>,
    /// True when a refutable pattern can also name a constant.
    ambiguous: bool,
}

/// A name that a second pass resolves to its own scope.
struct Candidate {
    name: String,
    offset: usize,
    declaration: Range<usize>,
    /// True for an item declaration, which is scoped to its own module.
    item: bool,
    /// True when the pattern can also name a constant, as a match arm can.
    ambiguous: bool,
}

#[derive(Default)]
struct Bindings {
    names: BTreeSet<String>,
    candidates: Vec<Candidate>,
    shorthands: BTreeSet<String>,
}

impl Bindings {
    fn pattern(&mut self, pat: &syn::Pat, lines: &LineIndex) {
        self.record(pat, lines, None, false);
    }
    /// A `let` binding becomes visible after its statement, not at its pattern.
    fn local(&mut self, pat: &syn::Pat, lines: &LineIndex, scope_start: usize) {
        self.record(pat, lines, Some(scope_start), false);
    }
    /// A bare pattern in a refutable position can name a constant instead of a
    /// fresh binding, so the file cannot be rewritten without resolving it.
    fn refutable(&mut self, pat: &syn::Pat, lines: &LineIndex) {
        self.record(pat, lines, None, matches!(pat, syn::Pat::Ident(_)));
    }
    fn record(
        &mut self,
        pat: &syn::Pat,
        lines: &LineIndex,
        scope_start: Option<usize>,
        ambiguous: bool,
    ) {
        let mut collector = PatternNames {
            lines,
            names: Vec::new(),
        };
        collector.visit_pat(pat);
        for (name, declaration) in collector.names {
            self.names.insert(name.clone());
            self.candidates.push(Candidate {
                offset: scope_start.unwrap_or(declaration.start),
                declaration,
                name,
                item: false,
                ambiguous,
            });
        }
    }
    fn item(&mut self, name: String, declaration: Range<usize>) {
        self.names.insert(name.clone());
        self.candidates.push(Candidate {
            offset: declaration.start,
            declaration,
            name,
            item: true,
            ambiguous: false,
        });
    }
}

/// Binding identifiers with their own source spans.
struct PatternNames<'a> {
    lines: &'a LineIndex,
    names: Vec<(String, Range<usize>)>,
}

impl<'ast> Visit<'ast> for PatternNames<'_> {
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.names
            .push((node.ident.to_string(), self.lines.range(node.ident.span())));
        syn::visit::visit_pat_ident(self, node);
    }
    fn visit_pat_struct(&mut self, node: &'ast syn::PatStruct) {
        for field in &node.fields {
            if let syn::Member::Named(name) = &field.member {
                if field.colon_token.is_none() {
                    self.names.push((name.to_string(), self.lines.range(name.span())));
                }
            }
        }
        syn::visit::visit_pat_struct(self, node);
    }
}

/// The innermost block and inline module around one byte offset.
struct Enclosing<'a> {
    lines: &'a LineIndex,
    offset: usize,
    block: Option<Range<usize>>,
    body: Option<Range<usize>>,
    module: Option<Range<usize>>,
}

impl<'ast> Visit<'ast> for Enclosing<'_> {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        tighten(&mut self.block, self.lines.range(node.span()), self.offset);
        syn::visit::visit_block(self, node);
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if node.content.is_some() {
            tighten(&mut self.module, self.lines.range(node.span()), self.offset);
        }
        syn::visit::visit_item_mod(self, node);
    }
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.record_body(node.span(), &node.block);
        syn::visit::visit_item_fn(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.record_body(node.span(), &node.block);
        syn::visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(block) = &node.default {
            self.record_body(node.span(), block);
        }
        syn::visit::visit_trait_item_fn(self, node);
    }
    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        if let syn::Expr::Block(block) = node.body.as_ref() {
            self.record_body(node.span(), &block.block);
        }
        syn::visit::visit_expr_closure(self, node);
    }
}

impl Enclosing<'_> {
    /// A parameter is declared in the signature, so its scope is the body of
    /// the function or closure that owns it.
    fn record_body(&mut self, span: proc_macro2::Span, block: &syn::Block) {
        let signature = self.lines.range(span);
        if signature.contains(&self.offset) {
            set_smallest(&mut self.body, self.lines.range(block.span()));
        }
    }
}

fn set_smallest(slot: &mut Option<Range<usize>>, range: Range<usize>) {
    let width = range.end.saturating_sub(range.start);
    if slot
        .as_ref()
        .is_none_or(|current| current.end.saturating_sub(current.start) > width)
    {
        *slot = Some(range);
    }
}

fn tighten(slot: &mut Option<Range<usize>>, range: Range<usize>, offset: usize) {
    if !range.contains(&offset) {
        return;
    }
    let width = range.end.saturating_sub(range.start);
    if slot
        .as_ref()
        .is_none_or(|current| current.end.saturating_sub(current.start) > width)
    {
        *slot = Some(range);
    }
}

/// Resolve every candidate name to the range where it hides an import.
fn build_scopes(
    syntax: &syn::File,
    lines: &LineIndex,
    candidates: &[Candidate],
    file_range: Range<usize>,
) -> Vec<Scope> {
    let mut scopes = Vec::new();
    for candidate in candidates {
        let mut enclosing = Enclosing {
            lines,
            offset: candidate.offset,
            block: None,
            body: None,
            module: None,
        };
        enclosing.visit_file(syntax);
        let range = if candidate.item {
            enclosing.module.unwrap_or_else(|| file_range.clone())
        } else {
            match enclosing.block.or(enclosing.body) {
                Some(range) => range,
                None => continue,
            }
        };
        scopes.push(Scope {
            name: candidate.name.clone(),
            range,
            offset: candidate.offset,
            declaration: candidate.declaration.clone(),
            ambiguous: candidate.ambiguous,
        });
    }
    scopes
}

/// True when a later binding or item hides `name` at this offset.
fn is_hidden(scopes: &[Scope], name: &str, offset: usize) -> bool {
    scopes.iter().any(|scope| {
        scope.name == name
            && (scope.declaration.contains(&offset)
                || (scope.range.contains(&offset) && offset >= scope.offset))
    })
}

/// True when a pattern that can name a constant must be qualified instead.
///
/// Rust requires a non-snake-case name for a constant, so an uppercase name in
/// a refutable pattern names the constant and a snake-case name binds a fresh
/// value.
fn pattern_names_constant(name: &str, scopes: &[Scope], offset: usize) -> bool {
    name.chars().any(char::is_uppercase)
        && scopes.iter().any(|scope| {
            scope.ambiguous && scope.name == name && scope.declaration.contains(&offset)
        })
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
                self.record_item(&item.sig.ident);
            }
            syn::Item::Struct(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Enum(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Union(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Trait(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Type(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Const(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Static(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Mod(item) => {
                self.record_item(&item.ident);
            }
            syn::Item::Macro(item) => {
                if let Some(ident) = &item.ident {
                    self.record_item(ident);
                }
            }
            syn::Item::ExternCrate(item) => {
                self.record_item(&item.ident);
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
        // A variant is reachable only through its enum, so it does not hide a
        // module-level name.
        syn::visit::visit_variant(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        // A method name is reachable only through its receiver, so it does not
        // hide a module-level name.
        syn::visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        syn::visit::visit_trait_item_fn(self, node);
    }
    fn visit_generic_param(&mut self, node: &'ast syn::GenericParam) {
        match node {
            syn::GenericParam::Type(param) => {
                self.record_item(&param.ident);
            }
            syn::GenericParam::Const(param) => {
                self.record_item(&param.ident);
            }
            syn::GenericParam::Lifetime(_) => {}
        }
        syn::visit::visit_generic_param(self, node);
    }
    fn visit_fn_arg(&mut self, node: &'ast syn::FnArg) {
        if let syn::FnArg::Typed(typed) = node {
            self.bindings.pattern(&typed.pat, self.lines);
        }
        syn::visit::visit_fn_arg(self, node);
    }
    fn visit_local(&mut self, node: &'ast syn::Local) {
        let scope_start = self.lines.range(node.span()).end;
        self.bindings.local(&node.pat, self.lines, scope_start);
        syn::visit::visit_local(self, node);
    }
    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        self.bindings.refutable(&node.pat, self.lines);
        syn::visit::visit_arm(self, node);
    }
    fn visit_expr_let(&mut self, node: &'ast syn::ExprLet) {
        self.bindings.refutable(&node.pat, self.lines);
        syn::visit::visit_expr_let(self, node);
    }
    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        for input in &node.inputs {
            self.bindings.pattern(input, self.lines);
        }
        syn::visit::visit_expr_closure(self, node);
    }
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.bindings.pattern(&node.pat, self.lines);
        syn::visit::visit_expr_for_loop(self, node);
    }
    fn visit_field_value(&mut self, node: &'ast syn::FieldValue) {
        if node.colon_token.is_none() {
            if let syn::Member::Named(name) = &node.member {
                self.bindings.shorthands.insert(name.to_string());
            }
        }
        syn::visit::visit_field_value(self, node);
    }
}

impl FileBindings<'_> {
    fn record_item(&mut self, ident: &proc_macro2::Ident) {
        let range = self.lines.range(ident.span());
        self.bindings.item(ident.to_string(), range);
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
        if bindings.bindings.shorthands.contains(name) {
            return Err(format!("descendant {path} reads `{name}` from a field shorthand"));
        }
        if bindings.extra_use_bindings.contains(name) {
            return Err(format!("descendant {path} imports `{name}` again"));
        }
    }
    let scopes = build_scopes(&syntax, &lines, &bindings.bindings.candidates, 0..source.len());
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
        &scopes,
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
    scopes: &[Scope],
    edits: &mut Vec<(Range<usize>, String)>,
) -> Result<(), String> {
    let mut previous = String::new();
    let mut colon_run = 0_usize;
    let mut segments: Vec<(String, Range<usize>)> = Vec::new();
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
                    scopes,
                    edits,
                )?;
                previous.clear();
                colon_run = 0;
                segments.clear();
            }
            proc_macro2::TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let range = lines.range(ident.span());
                // Two colons continue one path, and a single colon ends it.
                if colon_run < 2 {
                    segments.clear();
                }
                segments.push((name.clone(), range.clone()));
                if let Some(owner) = owners.get(&name) {
                    let hidden = previous == "."
                        || is_macro_name(source, range.end)
                        || followed_by_colon(source, range.end);
                    let shadowed = !pattern_names_constant(&name, scopes, range.start)
                        && is_hidden(scopes, &name, range.start);
                    if !hidden && !shadowed {
                        record_reference(owner, &segments, depth, module_ranges, edits)?;
                    }
                }
                previous = name;
                colon_run = 0;
            }
            proc_macro2::TokenTree::Punct(punct) => {
                let character = punct.as_char();
                previous = character.to_string();
                if character == ':' {
                    colon_run += 1;
                } else {
                    colon_run = 0;
                    segments.clear();
                }
            }
            proc_macro2::TokenTree::Literal(_) => {
                previous.clear();
                colon_run = 0;
                segments.clear();
            }
        }
    }
    Ok(())
}

/// Record one reference to a parent import from inside a descendant module.
///
/// A single segment is a bare reference through `use super::*`. A path that
/// starts at `super` and lands inside the parent tree also names the import,
/// so the whole path is replaced. A path that climbs above the parent, or one
/// that starts at `self`, can name something else, so it is reported.
fn record_reference(
    owner: &str,
    segments: &[(String, Range<usize>)],
    depth: usize,
    module_ranges: &[Range<usize>],
    edits: &mut Vec<(Range<usize>, String)>,
) -> Result<(), String> {
    let Some((_, last)) = segments.last() else {
        return Ok(());
    };
    if segments.len() == 1 {
        let adjusted = depth + module_depth(module_ranges, last.start);
        edits.push((last.clone(), qualify_owner(owner, adjusted)));
        return Ok(());
    }
    let lead = segments[0].0.as_str();
    if lead != "super" && lead != "self" {
        return Ok(());
    }
    let supers = segments.iter().take_while(|(name, _)| name == "super").count();
    if supers > depth {
        // The path lands above the parent, so it names something else.
        return Ok(());
    }
    if lead == "self" {
        return Err(format!(
            "qualified reference through `self` to `{}` needs a manual path",
            segments.last().map(|(name, _)| name.clone()).unwrap_or_default()
        ));
    }
    let start = segments[0].1.start;
    let adjusted = depth + module_depth(module_ranges, start);
    edits.push((start..last.end, qualify_owner(owner, adjusted)));
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
        if file_bindings.extra_use_bindings.contains(name) {
            skipped.push(format!("second `use` binds `{name}`"));
        }
        if file_bindings.bindings.shorthands.contains(name) {
            skipped.push(format!("field shorthand reads `{name}`"));
        }
    }
    if !skipped.is_empty() {
        return Err(format!("manual repair required: {}", skipped.join("; ")));
    }
    let scopes = build_scopes(
        &syntax,
        &lines,
        &file_bindings.bindings.candidates,
        0..source.len(),
    );

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
        &scopes,
        source,
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
    scopes: &[Scope],
    source: &str,
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
                    scopes,
                    source,
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
                // A field name is not a path reference.
                let is_field = followed_by_colon(source, range.end);
                if !in_import && !is_qualified && !is_field {
                    if let Some(owner) = owners.get(&name) {
                        let depth = module_depth(module_ranges, start);
                        if pattern_names_constant(&name, scopes, start) {
                            // A constant pattern over an uppercase name: qualify
                            // it so the pattern keeps naming the constant.
                            edits.push((range, qualify_owner(owner, depth)));
                        } else if is_hidden(scopes, &name, start) {
                            // A later binding or item owns this name here, so the
                            // reference resolves to it, not to the import.
                        } else if owner.starts_with("self::") && depth > 0 {
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
    let (output, edits) = rewrite_descendant("src/addr/route/child.rs", 1, qualified, &owners)?;
    if edits != 1 || !output.contains("value: super::super::Addr") {
        return Err(format!("self test did not replace a qualified descendant path in\n{output}"));
    }
    let outer = "fn f(value: super::super::Addr) -> usize {\n    let _ = value;\n    0\n}\n";
    let (output, edits) = rewrite_descendant("src/addr/route/child.rs", 1, outer, &owners)?;
    if edits != 0 || !output.contains("value: super::super::Addr") {
        return Err(format!("self test rewrote a path above the parent in\n{output}"));
    }
    let unrelated = "fn f(value: crate::addr::Addr) -> usize {\n    let _ = value;\n    0\n}\n";
    let (output, edits) = rewrite_descendant("src/addr/route/child.rs", 1, unrelated, &owners)?;
    if edits != 0 || !output.contains("value: crate::addr::Addr") {
        return Err(format!("self test rewrote an unrelated absolute path in\n{output}"));
    }
    let shadowed_child = "fn f() {\n    let Addr = 1;\n    let _ = Addr;\n}\n\nfn g(value: Addr) -> Addr {\n    value\n}\n";
    let (output, edits) = rewrite_descendant("src/addr/route/child.rs", 1, shadowed_child, &owners)?;
    if edits != 2 || !output.contains("fn g(value: super::super::Addr) -> super::super::Addr") {
        return Err(format!("self test mishandled a partial shadow in\n{output}"));
    }
    if !output.contains("let Addr = 1;") || !output.contains("let _ = Addr;") {
        return Err(format!("self test rewrote a shadowed binding in\n{output}"));
    }
    let shadowed = "use crate::addr::Addr;\n\nfn f() {\n    let Addr = 1;\n    let _ = Addr;\n}\n\nfn g(value: Addr) {\n    let _ = value;\n}\n";
    let output = qualify_source("src/addr/route.rs", shadowed, &[1])?.output;
    if !output.contains("fn g(value: crate::addr::Addr)") {
        return Err(format!("self test missed an unshadowed reference in\n{output}"));
    }
    if !output.contains("let Addr = 1;") || output.contains("use crate::addr::Addr;") {
        return Err(format!("self test rewrote a shadowed binding in\n{output}"));
    }
    let anonymous = "use std::hash::Hash as _;\nuse std::hash::Hasher;\n";
    if qualify_source("src/addr/route.rs", anonymous, &[1]).is_ok() {
        return Err(String::from("self test accepted an anonymous import"));
    }
    let formatted = "use crate::addr::Addr;\n\nfn f(value: Addr) -> String {\n    format!(\"{Addr}\")\n}\n";
    if qualify_source("src/addr/route.rs", formatted, &[1]).is_ok() {
        return Err(String::from("self test accepted an inline format argument"));
    }
    // An uppercase name in a match arm names the imported constant, so the
    // pattern keeps its meaning only when it is qualified.
    let constant = "use crate::addr::ADDR;\n\nfn f(value: u32) -> u32 {\n    match value {\n        ADDR => 1,\n        _ => 2,\n    }\n}\n";
    let output = qualify_source("src/addr/route.rs", constant, &[1])?.output;
    if !output.contains("crate::addr::ADDR => 1") {
        return Err(format!("self test left a constant pattern unqualified in\n{output}"));
    }
    // A snake-case name in the same position binds a fresh value instead, so
    // the pattern keeps the import's meaning only as a local binding.
    let bound = "use crate::addr::addr;\n\nfn f(value: u32) -> u32 {\n    match value {\n        addr => addr,\n        _ => 2,\n    }\n}\n";
    let output = qualify_source("src/addr/route.rs", bound, &[1])?.output;
    if !output.contains("addr => addr") || output.contains("crate::addr::addr =>") {
        return Err(format!("self test rewrote a snake-case binding pattern in\n{output}"));
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
