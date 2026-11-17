impl super::Scanner<'_> {
    pub(super) fn text(
        &mut self,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<u8>, crate::Diagnostic> {
        let start = self.at;
        self.at += 1;
        let mut bytes = alloc::vec::Vec::new();
        let mut failure = None;
        let mut is_closed = false;
        while self.at <= self.source.len() {
            match self.text_step(&mut bytes, start, meter) {
                Ok(true) => {
                    is_closed = true;
                    break;
                }
                Ok(false) => {}
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None if is_closed => Ok(bytes),
            None => Err(crate::internal(self.full)),
        }
    }

    #[expect(
        tigerstyle::borrowed_argument_types,
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; each metered byte grows the owned literal buffer after reservation or returns a text/escape diagnostic; a fixed slice cannot append bytes and these operations are not const."
    )]
    fn text_step(
        &mut self,
        bytes: &mut alloc::vec::Vec<u8>,
        start: usize,
        meter: &mut crate::Meter,
    ) -> Result<bool, crate::Diagnostic> {
        let byte = attempt!(self.byte(meter));
        if byte == b'"' {
            return Ok(true);
        }
        bytes.reserve(1);
        match byte {
            b'\r' | b'\n' => Err(crate::invalid(
                attempt!(self.span(start)),
                "raw line terminator in text literal",
            )),
            b'\\' => {
                attempt!(self.escape(bytes, start, meter));
                Ok(false)
            }
            _ => {
                let previous = self.at.saturating_sub(1);
                if self.source.get(previous..previous.saturating_add(2)) == Some(&[0xc2, 0x85])
                    || self.source.get(previous..previous.saturating_add(3))
                        == Some(&[0xe2, 0x80, 0xa8])
                    || self.source.get(previous..previous.saturating_add(3))
                        == Some(&[0xe2, 0x80, 0xa9])
                {
                    return Err(crate::invalid(
                        attempt!(self.span(start)),
                        "raw Unicode line terminator in text literal",
                    ));
                }
                bytes.push(byte);
                Ok(false)
            }
        }
    }

    #[expect(
        tigerstyle::borrowed_argument_types,
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; escape decoding appends owned UTF-8 bytes after bounded reservation and metered input reads, requiring a growable Vec and allocating diagnostics on invalid escapes."
    )]
    fn escape(
        &mut self,
        bytes: &mut alloc::vec::Vec<u8>,
        start: usize,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        let escaped = attempt!(self.byte(meter));
        match escaped {
            b'"' | b'\\' => bytes.push(escaped),
            b'n' => bytes.push(b'\n'),
            b'r' => bytes.push(b'\r'),
            b't' => bytes.push(b'\t'),
            b'u' => {
                let scalar = attempt!(self.unicode(start, meter));
                let mut encoded = [0u8; 4];
                bytes.reserve(4);
                bytes.extend_from_slice(scalar.encode_utf8(&mut encoded).as_bytes());
            }
            _ => {
                return Err(crate::invalid(
                    attempt!(self.span(start)),
                    "unsupported text escape",
                ))
            }
        }
        Ok(())
    }

    fn unicode(
        &mut self,
        start: usize,
        meter: &mut crate::Meter,
    ) -> Result<char, crate::Diagnostic> {
        if attempt!(self.byte(meter)) != b'{' {
            return Err(crate::invalid(
                attempt!(self.span(start)),
                "Unicode escape requires braces",
            ));
        }
        let mut scalar = 0u32;
        let mut digits = 0u32;
        let mut failure = None;
        while digits <= 6 {
            match self.digit(start, digits, meter) {
                Ok(Some(digit)) => {
                    scalar = scalar.saturating_mul(16).saturating_add(u32::from(digit));
                    digits += 1;
                }
                Ok(None) => break,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        if digits == 0 {
            return Err(crate::invalid(
                attempt!(self.span(start)),
                "empty Unicode escape",
            ));
        }
        match char::from_u32(scalar) {
            Some(scalar) => Ok(scalar),
            None => Err(crate::invalid(
                attempt!(self.span(start)),
                "Unicode escape is not a scalar value",
            )),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; one Unicode digit is read under the work meter before enforcing the six-digit and hexadecimal bounds, returning owned diagnostics on rejection."
    )]
    fn digit(
        &mut self,
        start: usize,
        digits: u32,
        meter: &mut crate::Meter,
    ) -> Result<Option<u8>, crate::Diagnostic> {
        let byte = attempt!(self.byte(meter));
        if byte == b'}' {
            return Ok(None);
        }
        if digits >= 6 {
            return Err(crate::invalid(
                attempt!(self.span(start)),
                "Unicode escape has more than six digits",
            ));
        }
        match hexadecimal(byte) {
            Some(digit) => Ok(Some(digit)),
            None => Err(crate::invalid(
                attempt!(self.span(start)),
                "nonhexadecimal Unicode escape",
            )),
        }
    }
}

const fn hexadecimal(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte.saturating_sub(b'0')),
        b'a'..=b'f' => Some(byte.saturating_sub(b'a').saturating_add(10)),
        b'A'..=b'F' => Some(byte.saturating_sub(b'A').saturating_add(10)),
        _ => None,
    }
}
