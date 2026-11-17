pub(super) struct Accumulator {
    pub(super) value: i64,
    is_negative: bool,
}

impl Accumulator {
    pub(super) fn new(is_negative: bool) -> Self {
        Self {
            value: 0,
            is_negative,
        }
    }

    pub(super) fn digit(
        &mut self,
        bytes: &[u8],
        at: usize,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        let byte = match bytes.get(at) {
            Some(byte) => *byte,
            None => return Err(crate::internal(span)),
        };
        if !byte.is_ascii_digit() {
            return Err(crate::invalid(span, "malformed integer literal"));
        }
        let value = match self.value.checked_mul(10) {
            Some(value) => value,
            None => return Err(crate::invalid(span, "integer literal is outside I64")),
        };
        #[expect(
            tigerstyle::raw_arithmetic_overflow,
            reason = "Owner: noble-maintainers; is_ascii_digit immediately above proves byte is in b'0'..=b'9', so subtracting b'0' cannot underflow."
        )]
        let digit = i64::from(byte - b'0');
        // Accumulating negative digits directly keeps i64::MIN representable.
        self.value = match if self.is_negative {
            value.checked_sub(digit)
        } else {
            value.checked_add(digit)
        } {
            Some(value) => value,
            None => return Err(crate::invalid(span, "integer literal is outside I64")),
        };
        Ok(())
    }
}
