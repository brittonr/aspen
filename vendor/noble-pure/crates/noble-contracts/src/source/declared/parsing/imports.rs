//! Exact import declaration and alias classification.

pub(super) fn parse(
    cursor: super::Cursor<'_>,
    name: alloc::string::String,
    version: u32,
) -> Result<super::ParsedUnit, crate::Diagnostic> {
    let cursor = attempt!(cursor.expected("as"));
    let (cursor, alias) = attempt!(cursor.ident());
    if !cursor.at.eq(&cursor.tokens.len()) {
        return Err(super::parse_diagnostic(
            cursor.span,
            "import declaration contains trailing expressions",
        ));
    }
    Ok(super::ParsedUnit::Import {
        name,
        version,
        alias,
    })
}
