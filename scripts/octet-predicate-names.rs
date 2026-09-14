#!/usr/bin/env -S nix shell nixpkgs#cargo nixpkgs#rustc nixpkgs#gcc --command cargo -q -Zscript
---
[dependencies]
proc-macro2 = { version = "=1.0.107", features = ["span-locations"] }
syn = { version = "=3.0.5", features = ["full", "visit"] }
---
// Octet `bool_naming` source repair.
//
// Octet requires a predicate prefix on a boolean `let` binding. This tool
// applies the prefix that Octet itself suggests:
//
//   1. Read the `bool_naming` rows from an Octet JSON result stream.
//   2. For each flagged binding, find the innermost block that contains it.
//   3. Rename every reference to that name inside the block, and skip tokens
//      after `.` or `::`, struct-literal keys, and nested `use` items.
//   4. Re-parse the result so a syntactically broken file is never written.
//
// The rewrite stays inside one block because the rename only needs to cover
// the scope of the binding. A file is reported and skipped when the suggested
// name already appears in the file, because the rename could then capture an
// unrelated binding instead of the flagged one.
//
// Compilation and tests stay the acceptance oracle for the files it rewrites.
//
// Usage:
//   scripts/octet-predicate-names.rs --results target/octet/results.jsonl [--dry-run] [FILE...]
//   scripts/octet-predicate-names.rs --self-test

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path as FsPath;

use syn::spanned::Spanned;
use syn::visit::Visit;

const LINT_NAME: &str = "bool_naming";
const MESSAGE_PREFIX: &str = "boolean binding `";
const MESSAGE_SUFFIX: &str = "` should have a predicate prefix";

/// Predicate prefixes that satisfy the lint, in preference order.
///
/// The first choice is Octet's own suggestion. A later choice is used when an
/// earlier one already names something else in the same file.
const PREDICATE_PREFIXES: &[&str] = &["is_", "should_", "has_", "can_", "was_", "will_", "needs_"];

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

/// One flagged boolean binding.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Binding {
    line: usize,
    column: usize,
    name: String,
}

impl Binding {
    fn preferred_name(&self) -> String {
        format!("{}{}", PREDICATE_PREFIXES[0], self.name)
    }
}

/// Minimal JSON object reader for one `results.jsonl` row.
///
/// The stream carries string and number values only, so a full JSON parser is
/// not needed. Escapes inside strings are decoded for the fields we read.
fn json_field(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let start = line.find(&needle)? + needle.len();
    let rest = line[start..].trim_start();
    if let Some(quoted) = rest.strip_prefix('"') {
        let mut value = String::new();
        let mut chars = quoted.chars();
        while let Some(character) = chars.next() {
            match character {
                '"' => return Some(value),
                '\\' => {
                    let escaped = chars.next()?;
                    value.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                }
                other => value.push(other),
            }
        }
        return None;
    }
    let end = rest.find([',', '}'])?;
    Some(rest[..end].trim().to_owned())
}

/// Parse the flagged bindings out of an Octet JSON result stream.
fn read_bindings(results: &str) -> BTreeMap<String, Vec<Binding>> {
    let mut flagged: BTreeMap<String, Vec<Binding>> = BTreeMap::new();
    for line in results.lines() {
        if !line.contains(LINT_NAME) {
            continue;
        }
        if json_field(line, "lint").as_deref() != Some(LINT_NAME) {
            continue;
        }
        let Some(message) = json_field(line, "message") else {
            continue;
        };
        let Some((_, tail)) = message.split_once(MESSAGE_PREFIX) else {
            continue;
        };
        let Some((name, _)) = tail.split_once(MESSAGE_SUFFIX) else {
            continue;
        };
        let Some(file) = json_field(line, "file") else {
            continue;
        };
        let line_number = json_field(line, "line").and_then(|value| value.parse::<usize>().ok());
        let column = json_field(line, "column").and_then(|value| value.parse::<usize>().ok());
        let (Some(line_number), Some(column)) = (line_number, column) else {
            continue;
        };
        let binding = Binding {
            line: line_number,
            column,
            name: name.to_owned(),
        };
        let entry = flagged.entry(file).or_default();
        if !entry.contains(&binding) {
            entry.push(binding);
        }
    }
    flagged
}

/// The tightest `let` statement that contains one byte offset.
struct EnclosingLocal<'a> {
    lines: &'a LineIndex,
    offset: usize,
    end: Option<usize>,
}

impl<'ast> Visit<'ast> for EnclosingLocal<'_> {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        let range = self.lines.range(node.span());
        if range.contains(&self.offset) {
            let end = range.end;
            if self.end.is_none_or(|current| end < current) {
                self.end = Some(end);
            }
        }
        syn::visit::visit_local(self, node);
    }
}

/// The tightest function body that contains one byte offset.
///
/// Rust lowers a function parameter to a synthetic `let` in the body, so a
/// flagged parameter has no enclosing block of its own. Its scope is the body
/// of the function whose signature contains the binding.
struct EnclosingFunction<'a> {
    lines: &'a LineIndex,
    offset: usize,
    best: Option<Range<usize>>,
}

impl<'ast> Visit<'ast> for EnclosingFunction<'_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.record(&node.sig, &node.block);
        syn::visit::visit_item_fn(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.record(&node.sig, &node.block);
        syn::visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(block) = &node.default {
            self.record(&node.sig, block);
        }
        syn::visit::visit_trait_item_fn(self, node);
    }
}

impl EnclosingFunction<'_> {
    fn record(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let signature = self.lines.range(sig.span());
        if !signature.contains(&self.offset) {
            return;
        }
        let body = self.lines.range(block.span());
        let width = body.end.saturating_sub(body.start);
        if self.best.as_ref().is_none_or(|current| current.end.saturating_sub(current.start) > width) {
            self.best = Some(body);
        }
    }
}

/// The tightest block that contains one byte offset.
struct EnclosingBlock<'a> {
    lines: &'a LineIndex,
    offset: usize,
    best: Option<Range<usize>>,
}

impl<'ast> Visit<'ast> for EnclosingBlock<'_> {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        let range = self.lines.range(node.span());
        if range.contains(&self.offset) {
            let width = range.end.saturating_sub(range.start);
            let replace = self
                .best
                .as_ref()
                .is_none_or(|current| current.end.saturating_sub(current.start) > width);
            if replace {
                self.best = Some(range);
            }
        }
        syn::visit::visit_block(self, node);
    }
}

/// Byte ranges of every `use` item in the file, at any depth.
struct UseRanges<'a> {
    lines: &'a LineIndex,
    ranges: Vec<Range<usize>>,
}

impl<'ast> Visit<'ast> for UseRanges<'_> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        self.ranges.push(self.lines.range(node.span()));
        syn::visit::visit_item_use(self, node);
    }
}

/// Field and member name ranges.
///
/// A struct or pattern field name is not the same identifier as the binding,
/// so a field name never gets renamed. A field shorthand such as
/// `Other { ready }` reads the binding and writes the field at the same time,
/// so that form expands to `Other { ready: is_ready }` instead.
struct FieldNames<'a> {
    lines: &'a LineIndex,
    protected: Vec<Range<usize>>,
    shorthands: BTreeMap<usize, String>,
}

impl<'ast> Visit<'ast> for FieldNames<'_> {
    fn visit_field(&mut self, node: &'ast syn::Field) {
        if let Some(ident) = &node.ident {
            self.protected.push(self.lines.range(ident.span()));
        }
        syn::visit::visit_field(self, node);
    }
    fn visit_variant(&mut self, node: &'ast syn::Variant) {
        self.protected.push(self.lines.range(node.ident.span()));
        syn::visit::visit_variant(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.protected.push(self.lines.range(node.sig.ident.span()));
        syn::visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.protected.push(self.lines.range(node.sig.ident.span()));
        syn::visit::visit_trait_item_fn(self, node);
    }
    fn visit_field_value(&mut self, node: &'ast syn::FieldValue) {
        if let syn::Member::Named(name) = &node.member {
            if node.colon_token.is_none() {
                self.shorthands
                    .insert(self.lines.offset(name.span().start()), name.to_string());
            } else {
                self.protected.push(self.lines.range(name.span()));
            }
        }
        syn::visit::visit_field_value(self, node);
    }
    fn visit_field_pat(&mut self, node: &'ast syn::FieldPat) {
        if let syn::Member::Named(name) = &node.member {
            if node.colon_token.is_none() {
                self.shorthands
                    .insert(self.lines.offset(name.span().start()), name.to_string());
            } else {
                self.protected.push(self.lines.range(name.span()));
            }
        }
        syn::visit::visit_field_pat(self, node);
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

/// True when the identifier at `end` is a macro invocation name.
///
/// `matches!(value)` and `x != value` both follow an identifier with `!`. Only
/// the invocation form is a name, so only that form stays untouched.
fn is_macro_name(source: &str, end: usize) -> bool {
    let Some(rest) = source.get(end..) else {
        return false;
    };
    let mut characters = rest.chars();
    if characters.next() != Some('!') {
        return false;
    }
    characters.next() != Some('=')
}

/// Rename the flagged bindings of one source file.
fn rewrite_source(
    path: &str,
    source: &str,
    bindings: &[Binding],
) -> Result<(String, usize), String> {
    let syntax = syn::parse_file(source).map_err(|error| format!("parse failed: {error}"))?;
    let lines = LineIndex::new(source);
    let mut renames: BTreeMap<String, String> = BTreeMap::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut scoped: Vec<(String, Range<usize>, usize, usize)> = Vec::new();

    for binding in bindings {
        // Octet reports one-based columns; the line index uses zero-based ones.
        let offset = lines.offsets[binding.line - 1] + binding.column.saturating_sub(1);
        let mut enclosing = EnclosingBlock {
            lines: &lines,
            offset,
            best: None,
        };
        enclosing.visit_file(&syntax);
        let block = match enclosing.best {
            Some(block) => block,
            None => {
                let mut function = EnclosingFunction {
                    lines: &lines,
                    offset,
                    best: None,
                };
                function.visit_file(&syntax);
                let Some(body) = function.best else {
                    skipped.push(format!("no enclosing block for `{}`", binding.name));
                    continue;
                };
                body
            }
        };
        let mut statement = EnclosingLocal {
            lines: &lines,
            offset,
            end: None,
        };
        statement.visit_file(&syntax);
        // A function parameter has no source `let`, so its own binding offset
        // starts the scope that the body uses share.
        let scope_start = statement.end.unwrap_or(offset);
        let renamed = PREDICATE_PREFIXES
            .iter()
            .map(|prefix| format!("{prefix}{}", binding.name))
            .find(|candidate| !mentions_identifier(source, candidate));
        let Some(suggested) = renamed else {
            skipped.push(format!(
                "every predicate name for `{}` already appears in {path}",
                binding.name
            ));
            continue;
        };
        renames.insert(binding.name.clone(), suggested.clone());
        scoped.push((binding.name.clone(), block, scope_start, offset));
    }

    if !skipped.is_empty() {
        return Err(format!("manual repair required: {}", skipped.join("; ")));
    }
    if renames.is_empty() {
        return Ok((source.to_string(), 0));
    }

    let mut uses = UseRanges {
        lines: &lines,
        ranges: Vec::new(),
    };
    uses.visit_file(&syntax);
    let mut fields = FieldNames {
        lines: &lines,
        protected: Vec::new(),
        shorthands: BTreeMap::new(),
    };
    fields.visit_file(&syntax);
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let tokens: proc_macro2::TokenStream = source
        .parse()
        .map_err(|error| format!("tokenize failed: {error}"))?;
    collect_renames(
        tokens,
        &renames,
        &scoped,
        &uses.ranges,
        &fields,
        &lines,
        source,
        &mut edits,
    );

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
    Ok((output, edit_count))
}

/// Record one rename per reference inside the binding's own block.
#[allow(clippy::too_many_arguments)]
fn collect_renames(
    stream: proc_macro2::TokenStream,
    renames: &BTreeMap<String, String>,
    scoped: &[(String, Range<usize>, usize, usize)],
    use_ranges: &[Range<usize>],
    fields: &FieldNames<'_>,
    lines: &LineIndex,
    source: &str,
    edits: &mut Vec<(Range<usize>, String)>,
) {
    let mut previous = String::new();
    for token in stream {
        match token {
            proc_macro2::TokenTree::Group(group) => {
                collect_renames(
                    group.stream(),
                    renames,
                    scoped,
                    use_ranges,
                    fields,
                    lines,
                    source,
                    edits,
                );
                previous.clear();
            }
            proc_macro2::TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let range = lines.range(ident.span());
                let start = range.start;
                let in_scope = scoped.iter().any(|(scope_name, block, scope_start, binding)| {
                    scope_name == &name
                        && (start == *binding || (block.contains(&start) && start >= *scope_start))
                });
                let in_use = use_ranges.iter().any(|item| item.contains(&start));
                let is_qualified = previous == "::" || previous == ".";
                let Some(replacement) = renames.get(&name) else {
                    previous = name;
                    continue;
                };
                if in_scope && !in_use && !is_qualified && !is_macro_name(source, range.end) {
                    if let Some(field) = fields.shorthands.get(&start) {
                        edits.push((range, format!("{field}: {replacement}")));
                    } else if !fields.protected.iter().any(|item| item.contains(&start)) {
                        edits.push((range, replacement.clone()));
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

fn run_results(results_path: &str, dry_run: bool, only: &[String]) -> Result<usize, String> {
    let results = std::fs::read_to_string(results_path)
        .map_err(|error| format!("cannot read {results_path}: {error}"))?;
    let flagged = read_bindings(&results);
    let mut total_edits = 0_usize;
    let mut repaired = 0_usize;
    let mut failed = 0_usize;
    for (path, bindings) in &flagged {
        if !only.is_empty() && !only.iter().any(|wanted| wanted == path) {
            continue;
        }
        if !path.ends_with(".rs") || !FsPath::new(path).exists() {
            continue;
        }
        let source =
            std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
        match rewrite_source(path, &source, bindings) {
            Ok((_output, edits)) if edits == 0 => {}
            Ok((_output, edits)) if dry_run => {
                println!("{path}: {edits} renames (dry run)");
                total_edits += edits;
                repaired += 1;
            }
            Ok((output, edits)) => {
                std::fs::write(path, output)
                    .map_err(|error| format!("cannot write {path}: {error}"))?;
                println!("{path}: {edits} renames");
                total_edits += edits;
                repaired += 1;
            }
            Err(message) => {
                eprintln!("{path}: SKIP ({message})");
                failed += 1;
            }
        }
    }
    println!("files repaired: {repaired}, renames: {total_edits}, skipped: {failed}");
    Ok(total_edits)
}

fn self_test() -> Result<(), String> {
    let source = r#"pub fn run() {
    let ready = true;
    if ready {
        report(ready);
    }
    let other = Other { ready };
    let _ = other;
}

fn report(value: bool) {
    let _ = value;
}
"#;
    let bindings = vec![Binding {
        line: 2,
        column: 9,
        name: String::from("ready"),
    }];
    let (output, edits) = rewrite_source("src/run.rs", source, &bindings)?;
    if edits != 4 {
        return Err(format!("self test expected 4 renames, got {edits}"));
    }
    if !output.contains("let is_ready = true;") {
        return Err(format!("self test did not rename the binding in\n{output}"));
    }
    if !output.contains("if is_ready {") || !output.contains("report(is_ready);") {
        return Err(format!("self test did not rename the references in\n{output}"));
    }
    if !output.contains("let other = Other { ready: is_ready };") {
        return Err(format!("self test did not expand a field shorthand in\n{output}"));
    }
    if !output.contains("fn report(value: bool)") {
        return Err(format!("self test renamed outside the binding block in\n{output}"));
    }

    let captured = "pub fn run() {\n    let ready = true;\n    let is_ready = false;\n    let _ = (ready, is_ready);\n}\n";
    let (output, _edits) = rewrite_source("src/run.rs", captured, &bindings)?;
    if !output.contains("let should_ready = true;") || !output.contains("(should_ready, is_ready)") {
        return Err(format!("self test did not fall back to a free predicate prefix in\n{output}"));
    }
    if bindings[0].preferred_name() != "is_ready" {
        return Err(String::from("self test changed the preferred predicate name"));
    }

    let qualified = "pub fn run() {\n    let ready = true;\n    let _ = ready;\n    let _ = Guard::ready();\n}\n";
    let (output, edits) = rewrite_source("src/run.rs", qualified, &bindings)?;
    if edits != 2 || !output.contains("Guard::ready()") {
        return Err(format!("self test rewrote a qualified path in\n{output}"));
    }
    let json = r#"{"lint":"bool_naming","file":"src/run.rs","line":2,"column":8,"message":"boolean binding `ready` should have a predicate prefix"}
{"lint":"no_unwrap","file":"src/other.rs","line":4,"column":3,"message":"call to `unwrap`"}
"#;
    let parsed = read_bindings(json);
    if parsed.len() != 1 || parsed["src/run.rs"].len() != 1 {
        return Err(String::from("self test did not filter the result stream"));
    }
    println!("self test passed");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let mut results: Option<String> = None;
    let mut dry_run = false;
    let mut self_check = false;
    let mut only: Vec<String> = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--results" => {
                results = Some(arguments.next().ok_or("missing value after --results")?);
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
    let results = results.ok_or("missing --results <octet results.jsonl path>")?;
    run_results(&results, dry_run, &only)?;
    Ok(())
}
