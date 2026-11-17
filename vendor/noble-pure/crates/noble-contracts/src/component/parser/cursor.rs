impl<'a> super::Cursor<'a> {
    pub(super) fn next(&mut self) -> Result<&'a str, crate::component::Error> {
        self.remaining = match self.remaining.checked_sub(1) {
            Some(value) => value,
            None => return Err(crate::component::exhausted()),
        };
        let token = match self.tokens.get(self.at) {
            Some(token) => token.clone(),
            None => return Err(crate::component::invalid("unexpected end of WIT")),
        };
        self.at = self.at.saturating_add(1);
        match self.source.get(token) {
            Some(token) => Ok(token),
            None => Err(crate::component::invalid("invalid WIT token boundary")),
        }
    }

    pub(super) fn take(&mut self, value: &str) -> Result<(), crate::component::Error> {
        if attempt!(self.next()) == value {
            Ok(())
        } else {
            Err(crate::component::invalid("unexpected WIT token"))
        }
    }

    pub(super) fn peek(&self, value: &str) -> bool {
        let span = match self.tokens.get(self.at) {
            Some(span) => span.clone(),
            None => return false,
        };
        match self.source.get(span) {
            Some(token) => token == value,
            None => false,
        }
    }

    pub(super) fn name(&mut self) -> Result<alloc::string::String, crate::component::Error> {
        identifier(attempt!(self.next()))
    }

    pub(super) fn version(&mut self) -> Result<alloc::string::String, crate::component::Error> {
        let mut version = alloc::string::String::with_capacity(super::MAX_VERSION_BYTES);
        let mut at = 0u32;
        let mut failure = None;
        while at < super::VERSION_PARTS {
            if let Err(problem) = self.version_part(at, &mut version) {
                failure = Some(problem);
                break;
            }
            at = at.saturating_add(1);
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(version),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-maintainers; this token-consuming helper appends each validated numeric part to the caller-owned String, requiring growable storage, and uses non-const integer parsing and owned diagnostics."
    )]
    fn version_part(
        &mut self,
        at: u32,
        version: &mut alloc::string::String,
    ) -> Result<(), crate::component::Error> {
        if at != 0 {
            attempt!(self.take("."));
        }
        let token = attempt!(self.next());
        if token.is_empty()
            || token.len() > super::MAX_VERSION_DIGITS
            || !token.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(crate::component::unsupported(
                "WIT requires an exact major.minor.patch version",
            ));
        }
        if token.len() > 1 && token.as_bytes().first() == Some(&b'0') {
            return Err(crate::component::unsupported(
                "WIT requires an exact major.minor.patch version",
            ));
        }
        if token.parse::<u32>().is_err() {
            return Err(crate::component::invalid("WIT version component overflows"));
        }
        append_version_part(version, at, token);
        Ok(())
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    tigerstyle::mutating_input_in_pure,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; this helper uses non-const String growth to append a validated numeric token and optional separator to the caller-owned version buffer; it does not mutate the borrowed source token."
)]
fn append_version_part(version: &mut alloc::string::String, at: u32, token: &str) {
    if at != 0 {
        version.push('.');
    }
    version.push_str(token);
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; escaping, reserved names, byte length, character classes and kebab boundaries are explicit fallible checks on WIT input, not assertion preconditions."
)]
fn identifier(token: &str) -> Result<alloc::string::String, crate::component::Error> {
    let is_escaped = token.as_bytes().first() == Some(&b'%');
    let start = if is_escaped { 1 } else { 0 };
    let name = match token.get(start..) {
        Some(name) => name,
        None => return Err(crate::component::invalid("invalid WIT identifier")),
    };
    if !is_escaped && reserved(name) {
        return Err(crate::component::invalid(
            "reserved WIT identifier requires percent escaping",
        ));
    }
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > super::MAX_NAME_BYTES {
        return Err(crate::component::invalid("invalid WIT identifier"));
    }
    if !bytes[0].is_ascii_lowercase() || bytes.last() == Some(&b'-') {
        return Err(crate::component::invalid("invalid WIT identifier"));
    }
    let mut at = 0usize;
    let mut is_previous_dash = false;
    let mut failure = None;
    while let Some(byte) = bytes.get(at).copied() {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && byte != b'-' {
            failure = Some(crate::component::invalid("invalid WIT identifier"));
            break;
        }
        if is_previous_dash && !byte.is_ascii_lowercase() {
            failure = Some(crate::component::invalid(
                "invalid kebab-case WIT identifier",
            ));
            break;
        }
        is_previous_dash = byte == b'-';
        at = at.saturating_add(1);
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(alloc::string::String::from(name)),
    }
}

fn reserved(name: &str) -> bool {
    declaration_keyword(name) || compound_keyword(name) || scalar_keyword(name)
}

fn declaration_keyword(name: &str) -> bool {
    matches!(
        name,
        "package"
            | "interface"
            | "world"
            | "import"
            | "export"
            | "include"
            | "use"
            | "with"
            | "as"
            | "type"
            | "resource"
            | "constructor"
            | "static"
            | "func"
            | "async"
    )
}

fn compound_keyword(name: &str) -> bool {
    matches!(
        name,
        "record"
            | "flags"
            | "variant"
            | "enum"
            | "tuple"
            | "list"
            | "option"
            | "result"
            | "own"
            | "borrow"
            | "future"
            | "stream"
    )
}

fn scalar_keyword(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "string"
            | "char"
            | "u8"
            | "s8"
            | "u16"
            | "s16"
            | "u32"
            | "s32"
            | "u64"
            | "s64"
            | "f32"
            | "f64"
    )
}
