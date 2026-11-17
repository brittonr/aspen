//! Construct and validate words published by nominal schemas.

enum ExportRejection {
    RequiresExportedSchema,
    MissingPublicConstructor,
    EmptyConstructor,
}

pub(super) struct WordBuilder {
    words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
    exported_words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
    added_words: alloc::vec::Vec<alloc::string::String>,
}

impl WordBuilder {
    pub(super) fn validate(
        self,
        schema: &crate::source::declared::Schema,
        exports: &[alloc::string::String],
        is_visible: bool,
    ) -> Result<Self, crate::source::Error> {
        let mut remaining = exports.len();
        let mut index = 0;
        let mut outcome = Ok(self);
        while remaining != 0 && outcome.is_ok() {
            outcome = match outcome {
                Ok(builder) => validate_step(builder, schema, exports, index, is_visible),
                Err(rejection) => Err(rejection),
            };
            index += 1;
            remaining -= 1;
        }
        match outcome {
            Ok(builder) => Ok(builder),
            Err(ExportRejection::RequiresExportedSchema) => Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "constructor export requires exported schema",
            )),
            Err(ExportRejection::MissingPublicConstructor) => Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "export references private or missing constructor",
            )),
            Err(ExportRejection::EmptyConstructor) => Err(crate::source::declared::error(
                crate::source::Stage::Resolve,
                "empty exported constructor",
            )),
        }
    }
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0015 schema_operation and has_exported_word are non-const on both pinned Rust compilers; reassess const exported-name lookup."
    )]
    fn export_rejection(
        &self,
        schema: &crate::source::declared::Schema,
        exported: &str,
        is_visible: bool,
    ) -> Option<ExportRejection> {
        match schema_operation(exported, schema) {
            None => None,
            Some(empty_operation) => {
                if !is_visible {
                    Some(ExportRejection::RequiresExportedSchema)
                } else if !self.has_exported_word(exported) {
                    Some(ExportRejection::MissingPublicConstructor)
                } else if empty_operation {
                    Some(ExportRejection::EmptyConstructor)
                } else {
                    None
                }
            }
        }
    }
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0277/E0658 Vec deref and E0015 Iterator::any prevent const on both pinned Rust compilers; reassess const exported-name scanning."
    )]
    fn has_exported_word(&self, exported: &str) -> bool {
        self.exported_words
            .iter()
            .any(|(name, _)| name.as_str() == exported)
    }

    #[expect(
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
    )]
    pub(super) fn record_export(
        self,
        schema: &str,
        session: crate::source::declared::ModuleSession,
        mut local_exports: alloc::vec::Vec<crate::source::declared::Export>,
        ty: Option<&noble_kernel::types::Ty>,
        generic: Option<noble_kernel::types::NominalTypeId>,
        is_visible: bool,
    ) -> super::exports::Installed {
        if is_visible {
            local_exports.push(crate::source::declared::Export {
                name: alloc::string::String::from(schema),
                ty: ty.cloned(),
                generic,
                words: self.exported_words,
            });
        }
        super::exports::Installed {
            session,
            local_exports,
            words: self.words,
        }
    }

    fn add(
        mut self,
        schema: &crate::source::declared::Schema,
        suffix: &str,
        definition: Option<noble_kernel::contracts::Definition>,
        is_external: bool,
    ) -> Result<Self, crate::source::Error> {
        if let Some(definition) = definition {
            let spelling = attempt!(constructor_spelling(schema, suffix));
            // Two operations of one schema that publish the same spelling make
            // lookup order decide the meaning; refuse instead of publishing an
            // ambiguous word.
            if self.added_words.contains(&spelling) {
                return Err(crate::source::declared::error(
                    crate::source::Stage::Resolve,
                    "duplicate constructor word",
                ));
            }
            self.added_words.push(spelling.clone());
            let target = crate::source::Target::Builtin(definition.0);
            if is_external {
                self.exported_words.push((spelling.clone(), target));
            }
            self.words.push((spelling, target));
        }
        Ok(self)
    }
}

fn constructor_spelling(
    schema: &crate::source::declared::Schema,
    suffix: &str,
) -> Result<alloc::string::String, crate::source::Error> {
    let capacity_bytes = attempt!(constructor_capacity(schema, suffix));
    let mut spelling = alloc::string::String::with_capacity(capacity_bytes);
    spelling.push_str(&schema.name);
    spelling.push('.');
    spelling.push_str(suffix);
    Ok(spelling)
}
fn constructor_capacity(
    schema: &crate::source::declared::Schema,
    suffix: &str,
) -> Result<usize, crate::source::Error> {
    let capacity_bytes = schema
        .name
        .len()
        .checked_add(1)
        .and_then(|length| length.checked_add(suffix.len()));
    if let Some(capacity_bytes) = capacity_bytes {
        if capacity_bytes <= isize::MAX as usize {
            return Ok(capacity_bytes);
        }
    }
    Err(crate::source::declared::error(
        crate::source::Stage::Check,
        "constructor word length overflow",
    ))
}

fn validate_step(
    builder: WordBuilder,
    schema: &crate::source::declared::Schema,
    exports: &[alloc::string::String],
    index: usize,
    is_visible: bool,
) -> Result<WordBuilder, ExportRejection> {
    if let Some(rejection) = builder.export_rejection(schema, &exports[index], is_visible) {
        return Err(rejection);
    }
    Ok(builder)
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 split_once_ascii and String PartialEq prevent const on both pinned Rust compilers; reassess const qualified-name parsing."
)]
fn schema_operation(exported: &str, schema: &crate::source::declared::Schema) -> Option<bool> {
    match crate::source::declared::split_once_ascii(exported, b'.') {
        None => None,
        Some((base, operation)) => {
            if base == schema.name {
                Some(operation.is_empty())
            } else {
                None
            }
        }
    }
}

pub(super) fn operations(
    schema: &crate::source::declared::Schema,
    ops: &noble_kernel::contracts::NominalOps,
    public: [bool; 2],
    is_visible: bool,
    words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
) -> Result<WordBuilder, crate::source::Error> {
    debug_assert_eq!(
        ops.new.is_some(),
        matches!(
            &schema.kind,
            crate::source::declared::SchemaKind::Opaque { .. }
        )
    );
    debug_assert_eq!(
        ops.matcher.is_some(),
        matches!(
            &schema.kind,
            crate::source::declared::SchemaKind::Variant { .. }
        )
    );
    let mut builder = WordBuilder {
        words,
        exported_words: alloc::vec::Vec::with_capacity(3),
        added_words: alloc::vec::Vec::with_capacity(3),
    };
    let (first, second) = match &schema.kind {
        crate::source::declared::SchemaKind::Opaque { .. } => {
            (("new", ops.new), ("into", ops.into))
        }
        crate::source::declared::SchemaKind::Variant { left, right, .. } => {
            ((left.as_str(), ops.left), (right.as_str(), ops.right))
        }
    };
    builder = attempt!(builder.add(schema, first.0, first.1, public[0]));
    builder = attempt!(builder.add(schema, second.0, second.1, public[1]));
    if matches!(
        &schema.kind,
        crate::source::declared::SchemaKind::Variant { .. }
    ) {
        return builder.add(
            schema,
            "match",
            ops.matcher,
            is_visible && public[0] && public[1],
        );
    }
    Ok(builder)
}

#[cfg(test)]
mod tests {
    fn fixture() -> super::WordBuilder {
        super::WordBuilder {
            words: alloc::vec::Vec::new(),
            exported_words: alloc::vec::Vec::new(),
            added_words: alloc::vec::Vec::new(),
        }
    }

    fn schema() -> crate::source::declared::Schema {
        crate::source::declared::Schema {
            name: alloc::string::String::from("V"),
            kind: crate::source::declared::SchemaKind::Opaque {
                base: alloc::string::String::from("I64"),
                public: true,
            },
        }
    }

    #[test]
    fn adding_the_same_constructor_word_twice_is_refused() {
        let builder = fixture()
            .add(
                &schema(),
                "new",
                Some(noble_kernel::contracts::Definition(0)),
                true,
            )
            .expect("first word");
        assert_eq!(builder.words.len(), 1);
        let duplicate = builder.add(
            &schema(),
            "new",
            Some(noble_kernel::contracts::Definition(1)),
            true,
        );
        assert!(duplicate.is_err(), "a repeated spelling would be ambiguous");
        let Err(failure) = duplicate else { return };
        assert_eq!(failure.diagnostic().message, "duplicate constructor word");
        assert_eq!(failure.stage(), crate::source::Stage::Resolve);
    }

    #[test]
    fn distinct_constructor_words_still_publish() {
        let builder = fixture()
            .add(
                &schema(),
                "new",
                Some(noble_kernel::contracts::Definition(0)),
                true,
            )
            .expect("first word")
            .add(
                &schema(),
                "into",
                Some(noble_kernel::contracts::Definition(1)),
                true,
            )
            .expect("second word");
        assert_eq!(builder.words.len(), 2);
        assert_eq!(builder.exported_words.len(), 2);
    }
}
