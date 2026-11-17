// Materialize only checked, addressable outside spellings for a staged namespace.

type OutsideWord = (alloc::string::String, crate::source::Target);

struct QualifiedParts {
    prefix_bytes: usize,
    separator_bytes: usize,
    name_bytes: usize,
}

fn capacity_error() -> crate::source::Error {
    super::super::super::error(
        crate::source::Stage::Check,
        "qualified module words exceed the host address space",
    )
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; Option.and_then const FnOnce and capacity_error are unavailable on both pinned Rust compilers; checked string bytes reject at Stage::Check; reassess const APIs."
)]
fn qualified_capacity(parts: QualifiedParts) -> Result<usize, crate::source::Error> {
    let Some(bytes) = parts
        .prefix_bytes
        .checked_add(parts.separator_bytes)
        .and_then(|bytes| bytes.checked_add(parts.name_bytes))
    else {
        return Err(capacity_error());
    };
    if bytes > isize::MAX as usize {
        return Err(capacity_error());
    }
    Ok(bytes)
}

fn add_export_count(
    mut count: Option<usize>,
    module: &super::super::super::Module,
) -> Option<usize> {
    let mut entry_slot = 0usize;
    while entry_slot < module.exports.len() && count.is_some() {
        count = count.and_then(|total| total.checked_add(module.exports[entry_slot].words.len()));
        entry_slot += 1;
    }
    count
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; capacity_error is non-const on both pinned Rust compilers; checked word-element byte bounds reject at Stage::Check; reassess const errors."
)]
fn checked_word_count(count: Option<usize>) -> Result<usize, crate::source::Error> {
    let Some(count) = count else {
        return Err(capacity_error());
    };
    let Some(bytes) = count.checked_mul(const { core::mem::size_of::<OutsideWord>() }) else {
        return Err(capacity_error());
    };
    if bytes > isize::MAX as usize {
        return Err(capacity_error());
    }
    Ok(count)
}

fn word_capacity(
    session: &super::super::ModuleSession,
    permit_signed: bool,
) -> Result<usize, crate::source::Error> {
    let mut count = Some(session.source.definitions.len());
    let mut alias_slot = 0usize;
    while alias_slot < session.aliases.len() && count.is_some() {
        let module_slot = session.aliases[alias_slot].module;
        if !session.aliases[alias_slot].pending && module_slot < session.modules.len() {
            count = add_export_count(count, &session.modules[module_slot]);
        }
        alias_slot += 1;
    }
    let mut module_slot = 0usize;
    while module_slot < session.modules.len() && count.is_some() {
        if permit_signed || !session.module_has_unverified_signed_owner(module_slot) {
            count = add_export_count(count, &session.modules[module_slot]);
        }
        module_slot += 1;
    }
    checked_word_count(count)
}

fn alias_qualified(
    alias: &super::super::super::Alias,
    name: &str,
) -> Result<alloc::string::String, crate::source::Error> {
    let capacity_bytes = attempt!(qualified_capacity(QualifiedParts {
        prefix_bytes: alias.spelling.len(),
        separator_bytes: 1,
        name_bytes: name.len(),
    }));
    let mut qualified = alloc::string::String::with_capacity(capacity_bytes);
    qualified.push_str(&alias.spelling);
    qualified.push('.');
    qualified.push_str(name);
    Ok(qualified)
}

fn module_qualified(
    module: &super::super::super::Module,
    name: &str,
    version_width: usize,
) -> Result<alloc::string::String, crate::source::Error> {
    let Some(separator_bytes) = version_width.checked_add(2) else {
        return Err(capacity_error());
    };
    let capacity_bytes = attempt!(qualified_capacity(QualifiedParts {
        prefix_bytes: module.name.len(),
        separator_bytes,
        name_bytes: name.len(),
    }));
    let mut qualified = alloc::string::String::with_capacity(capacity_bytes);
    qualified.push_str(&module.name);
    qualified.push('@');
    qualified = super::super::super::append_decimal(qualified, module.version);
    qualified.push('.');
    qualified.push_str(name);
    Ok(qualified)
}

fn append_alias_exports(
    session: &super::super::ModuleSession,
    words: alloc::vec::Vec<OutsideWord>,
) -> Result<alloc::vec::Vec<OutsideWord>, crate::source::Error> {
    let mut alias_slot = 0usize;
    let mut outcome = Ok(words);
    while alias_slot < session.aliases.len() && outcome.is_ok() {
        let module_slot = session.aliases[alias_slot].module;
        if !session.aliases[alias_slot].pending && module_slot < session.modules.len() {
            outcome = match outcome {
                Ok(words) => project_alias_module(
                    &session.aliases[alias_slot],
                    &session.modules[module_slot],
                    words,
                ),
                Err(problem) => Err(problem),
            };
        }
        alias_slot += 1;
    }
    outcome
}

fn project_alias_module(
    alias: &super::super::super::Alias,
    module: &super::super::super::Module,
    words: alloc::vec::Vec<OutsideWord>,
) -> Result<alloc::vec::Vec<OutsideWord>, crate::source::Error> {
    let mut entry_slot = 0usize;
    let mut outcome = Ok(words);
    while entry_slot < module.exports.len() && outcome.is_ok() {
        outcome = match outcome {
            Ok(words) => project_alias_export(alias, &module.exports[entry_slot], words),
            Err(problem) => Err(problem),
        };
        entry_slot += 1;
    }
    outcome
}

fn project_alias_export(
    alias: &super::super::super::Alias,
    export: &super::super::super::Export,
    mut words: alloc::vec::Vec<OutsideWord>,
) -> Result<alloc::vec::Vec<OutsideWord>, crate::source::Error> {
    let mut word_slot = 0usize;
    let mut failure = None;
    while word_slot < export.words.len() && failure.is_none() {
        let (name, target) = &export.words[word_slot];
        match alias_qualified(alias, name) {
            Ok(qualified) => words.push((qualified, *target)),
            Err(problem) => failure = Some(problem),
        }
        word_slot += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(words),
    }
}

fn append_versioned_exports(
    session: &super::super::ModuleSession,
    words: alloc::vec::Vec<OutsideWord>,
    permit_signed: bool,
) -> Result<alloc::vec::Vec<OutsideWord>, crate::source::Error> {
    let mut module_slot = 0usize;
    let mut outcome = Ok(words);
    while module_slot < session.modules.len() && outcome.is_ok() {
        if permit_signed || !session.module_has_unverified_signed_owner(module_slot) {
            outcome = match outcome {
                Ok(words) => project_versioned_module(&session.modules[module_slot], words),
                Err(problem) => Err(problem),
            };
        }
        module_slot += 1;
    }
    outcome
}

const fn decimal_width(version: u32) -> usize {
    let mut width = 1usize;
    let mut remaining = version;
    while remaining >= 10 {
        width += 1;
        remaining /= 10;
    }
    width
}

fn project_versioned_module(
    module: &super::super::super::Module,
    words: alloc::vec::Vec<OutsideWord>,
) -> Result<alloc::vec::Vec<OutsideWord>, crate::source::Error> {
    let version_width = decimal_width(module.version);
    let mut entry_slot = 0usize;
    let mut outcome = Ok(words);
    while entry_slot < module.exports.len() && outcome.is_ok() {
        outcome = match outcome {
            Ok(words) => {
                project_versioned_export(module, &module.exports[entry_slot], version_width, words)
            }
            Err(problem) => Err(problem),
        };
        entry_slot += 1;
    }
    outcome
}

fn project_versioned_export(
    module: &super::super::super::Module,
    export: &super::super::super::Export,
    version_width: usize,
    mut words: alloc::vec::Vec<OutsideWord>,
) -> Result<alloc::vec::Vec<OutsideWord>, crate::source::Error> {
    let mut word_slot = 0usize;
    let mut failure = None;
    while word_slot < export.words.len() && failure.is_none() {
        let (name, target) = &export.words[word_slot];
        match module_qualified(module, name, version_width) {
            Ok(qualified) => words.push((qualified, *target)),
            Err(problem) => failure = Some(problem),
        }
        word_slot += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(words),
    }
}

impl super::super::ModuleSession {
    pub(in crate::source::declared::state) fn outside_words(
        self,
    ) -> Result<Self, crate::source::Error> {
        self.build_outside_words(false)
    }

    /// Pending signed exports are visible only inside an unpublished
    /// expression's derivation; its owner must pass original-source and
    /// concrete kernel checks before a prepared expression can escape.
    pub(in crate::source::declared::state) fn pending_signed_expression_words(
        self,
    ) -> Result<Self, crate::source::Error> {
        self.build_outside_words(true)
    }

    fn build_outside_words(mut self, permit_signed: bool) -> Result<Self, crate::source::Error> {
        attempt!(self.refresh_outside_words(permit_signed));
        Ok(self)
    }

    /// Refresh an unpublished successor after replaying a checked outside
    /// definition; no repeated whole-session clone is needed per dependent.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable."
    )]
    pub(in crate::source::declared::state) fn refresh_outside_words(
        &mut self,
        permit_signed: bool,
    ) -> Result<(), crate::source::Error> {
        let word_slots_count = attempt!(word_capacity(self, permit_signed));
        let mut words = alloc::vec::Vec::with_capacity(word_slots_count);
        let mut definition_slot = 0usize;
        while definition_slot < self.source.definitions.len() {
            if self.source.definitions[definition_slot].owner.is_none() {
                words.push((
                    self.source.definitions[definition_slot].name.clone(),
                    crate::source::Target::Named(definition_slot as u32),
                ));
            }
            definition_slot += 1;
        }
        words = attempt!(append_alias_exports(self, words));
        words = attempt!(append_versioned_exports(self, words, permit_signed));
        let mut aliases = alloc::vec::Vec::with_capacity(self.aliases.len());
        for alias in &self.aliases {
            if alias.pending {
                continue;
            }
            let module = self.modules.get(alias.module).ok_or_else(|| {
                super::super::super::error(
                    crate::source::Stage::Check,
                    "checked alias has no immutable module owner",
                )
            })?;
            aliases.push((alias.spelling.clone(), module.identity));
        }
        if let Some(context) = &mut self.source.declared {
            context.owner = None;
            context.environment.caller_module = None;
            context.words = words;
            context.aliases = aliases;
        }
        Ok(())
    }
}
