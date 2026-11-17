mod graph;
mod scheduling;

pub(super) type Definition = (
    alloc::string::String,
    alloc::vec::Vec<u8>,
    crate::source::Tree,
);

pub(super) struct Work<'a> {
    pub definitions: alloc::vec::Vec<Definition>,
    pub signatures: alloc::vec::Vec<(alloc::string::String, noble_kernel::words::Scheme)>,
    pub exports: &'a [alloc::string::String],
    pub local_exports: alloc::vec::Vec<crate::source::declared::Export>,
    pub name: &'a str,
    pub version: u32,
    pub limits: crate::Limits,
    pub meter: crate::Meter,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
pub(super) fn install(
    session: crate::source::declared::ModuleSession,
    mut work: Work<'_>,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        alloc::vec::Vec<crate::source::declared::Export>,
    ),
    crate::source::Error,
> {
    let count = attempt!(u32::try_from(work.definitions.len()).map_err(|_| {
        crate::source::declared::error(crate::source::Stage::Check, "definition count overflow")
    }));
    let definition_limits = if session.source.calculator {
        // Temporary length prefixes are replaced with the original source unit.
        // Calculator modules share the already-metered registration budget
        // instead of dividing it equally: a large but valid definition can use
        // the slack of smaller ones.
        work.meter.limits.bytes = work.limits.bytes.saturating_add(count.saturating_mul(4));
        None
    } else {
        Some(attempt!(definition_limits(count, work.limits).ok_or_else(
            || {
                crate::source::declared::error(
                    crate::source::Stage::Check,
                    "invalid definition resource budget",
                )
            }
        )))
    };
    // Per-definition length headers are transient: after graph installation
    // the whole original module replaces these temporary history entries.
    let headers = attempt!(count.checked_mul(4).ok_or_else(|| {
        crate::source::declared::error(
            crate::source::Stage::Check,
            "temporary definition header count overflow",
        )
    }));
    let temporary_retained_bytes = attempt!(session
        .retained_source_limit
        .checked_add(headers)
        .ok_or_else(|| {
            crate::source::declared::error(
                crate::source::Stage::Check,
                "temporary definition retention bound overflow",
            )
        }));
    let history_start = session.source.history.len();
    let graph = attempt!(graph::Schedule::setup(
        session,
        work,
        definition_limits,
        temporary_retained_bytes,
    ));
    let (session, work) = attempt!(graph.run());
    finish(session, work, history_start)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 Vec::reserve and declared::error plus E0493 owned ModuleSession prevent const on both pinned Rust compilers; reassess const collection and error APIs."
)]
fn reserve_words(
    mut session: crate::source::declared::ModuleSession,
    definition_count: usize,
) -> Result<crate::source::declared::ModuleSession, crate::source::Error> {
    let context = match session.source.declared.as_mut() {
        Some(context) => context,
        None => {
            return Err(crate::source::declared::error(
                crate::source::Stage::Check,
                "missing declared context",
            ));
        }
    };
    context.words.reserve(definition_count);
    Ok(session)
}

fn finish(
    mut session: crate::source::declared::ModuleSession,
    work: Work<'_>,
    history_start: usize,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        alloc::vec::Vec<crate::source::declared::Export>,
    ),
    crate::source::Error,
> {
    if work.conflicts(&session) {
        return Err(crate::source::declared::error(
            crate::source::Stage::Resolve,
            "module export conflicts with a qualified definition",
        ));
    }
    // The transaction's temporary definition histories are replaced by the full source unit.
    session.source.history.truncate(history_start);
    Ok((session, work.local_exports))
}

impl Work<'_> {
    fn conflicts(&self, session: &crate::source::declared::ModuleSession) -> bool {
        session.source.definitions.iter().any(|definition| {
            conflicts_with_definition(definition, self.name, self.version, &self.local_exports)
        })
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 strip_qualified, split_once_ascii and str::parse plus E0658 Result::ok prevent const on both pinned Rust compilers; reassess const parsing APIs."
)]
fn conflicts_with_definition(
    definition: &crate::source::Named,
    name: &str,
    version: u32,
    local_exports: &[crate::source::declared::Export],
) -> bool {
    if definition.owner.is_some() {
        return false;
    }
    let Some(qualified) =
        crate::source::declared::strip_qualified(definition.name.as_str(), name.as_bytes(), b'@')
    else {
        return false;
    };
    let Some((declared_version, word)) = crate::source::declared::split_once_ascii(qualified, b'.')
    else {
        return false;
    };
    crate::source::declared::canonical_version(declared_version) == Some(version)
        && contains_exported_word(local_exports, word)
}

fn contains_exported_word(exports: &[crate::source::declared::Export], word: &str) -> bool {
    let mut entry_index = 0;
    let mut has_word = false;
    while entry_index < exports.len() && !has_word {
        has_word = contains_word(&exports[entry_index].words, word);
        entry_index += 1;
    }
    has_word
}

fn contains_word(words: &[(alloc::string::String, crate::source::Target)], word: &str) -> bool {
    let mut entry_index = 0;
    let mut has_word = false;
    while entry_index < words.len() && !has_word {
        has_word = words[entry_index].0.as_str() == word;
        entry_index += 1;
    }
    has_word
}

const fn definition_limits(count: u32, limits: crate::Limits) -> Option<crate::Limits> {
    let budget = count.saturating_add(1);
    let Some(nodes) = limits.nodes.checked_div(budget) else {
        return None;
    };
    let Some(work) = limits.work.checked_div(budget) else {
        return None;
    };
    Some(crate::Limits {
        // Temporary per-definition length prefixes do not count twice toward retained source bytes.
        bytes: limits.bytes.saturating_add(count.saturating_mul(4)),
        nodes,
        depth: limits.depth,
        work,
    })
}

struct DefinitionInput<'a> {
    name: alloc::string::String,
    signature: Option<noble_kernel::words::Scheme>,
    source: alloc::vec::Vec<u8>,
    /// Ordinary declared modules split the registration budget equally per
    /// definition; `None` selects the calculator's shared registration meter.
    limits: Option<crate::Limits>,
    retained_bytes: u32,
    exports: &'a [alloc::string::String],
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; the parameter is the owned builder state this stage accumulates into, not a borrowed input"
)]
fn install_one(
    session: crate::source::declared::ModuleSession,
    input: DefinitionInput<'_>,
    mut local_exports: alloc::vec::Vec<crate::source::declared::Export>,
    meter: &mut crate::Meter,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        alloc::vec::Vec<crate::source::declared::Export>,
    ),
    crate::source::Error,
> {
    let (mut session, target) = attempt!(commit_definition(
        session,
        &input.source,
        input.signature.as_ref(),
        input.limits,
        input.retained_bytes,
        meter,
    ));
    let Some(context) = session.source.declared.as_mut() else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "missing declared context",
        ));
    };
    if input.exports.contains(&input.name) {
        local_exports.push(exported_definition(&input.name, target));
    }
    context.words.push((input.name, target));
    Ok((session, local_exports))
}

fn exported_definition(
    name: &str,
    target: crate::source::Target,
) -> crate::source::declared::Export {
    crate::source::declared::Export {
        name: alloc::string::String::from(name),
        ty: None,
        generic: None,
        words: alloc::vec![(alloc::string::String::from(name), target)],
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; the parameter is the owned builder state this stage accumulates into, not a borrowed input"
)]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; committing mutates the owned session and the work meter through non-const calls."
)]
fn commit_definition(
    mut session: crate::source::declared::ModuleSession,
    definition_bytes: &[u8],
    signature: Option<&noble_kernel::words::Scheme>,
    limits: Option<crate::Limits>,
    retained_bytes: u32,
    meter: &mut crate::Meter,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        crate::source::Target,
    ),
    crate::source::Error,
> {
    let prepared = if let Some(limits) = limits {
        session
            .source
            .prepare_signed(definition_bytes, &[], signature, limits, retained_bytes)
    } else {
        // Each calculator definition is a separate source-checking body. Its
        // nodes are bounded independently; rechecking a retained dependency
        // must not consume every later body's node allowance. Validation
        // work, unlike nodes, remains shared across the whole registration.
        let mut body_meter = crate::Meter::pooled(meter.limits, meter.pool());
        body_meter.work = meter.work;
        let prepared = session.source.prepare_signed_metered(
            definition_bytes,
            &[],
            signature,
            retained_bytes,
            &mut body_meter,
        );
        meter.work = body_meter.work;
        prepared
    };
    let prepared = attempt!(prepared);
    if !prepared.is_definition() || prepared.submission().is_some() {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "module definition has executable initializer",
        ));
    }
    if prepared
        .definition
        .as_ref()
        .and_then(|definition| definition.signed.as_ref())
        .is_some_and(|relation| {
            usize::try_from(relation.source_slot).ok() != Some(session.source.definitions.len())
        })
    {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "prospective signed owner slot changed before module installation",
        ));
    }
    attempt!(session.source.commit(prepared));
    let Some(index) = session.source.definitions.len().checked_sub(1) else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "missing declared definition",
        ));
    };
    let target = crate::source::Target::Named(index as u32);
    Ok((session, target))
}
