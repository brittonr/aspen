/// Hard ceilings bound retained storage, preflight scans and bit-mask work.
pub const MAX_SLOTS: usize = 256;
pub const MAX_OBLIGATIONS: u8 = 64;
pub const MAX_BYTES: usize = 67_108_864;
pub const MAX_WAKEUPS: u64 = 65_536;

/// Reservation includes a terminal result even when no result bytes are needed.
/// Input/result positions and native pins are numbered from zero, densely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub context: super::Context,
    pub native: super::NativeId,
    pub inputs: u8,
    pub results: u8,
    pub input_bytes: usize,
    pub result_bytes: usize,
    pub parked_bytes: usize,
    pub pins: u8,
    pub wakeups: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub tasks: usize,
    pub terminal_results: usize,
    pub bytes: usize,
    pub parked_payloads: usize,
    pub pins: usize,
    pub wakeups: u64,
    pub retirement_work: usize,
    pub generations: u64,
}

/// Retained admission reservations, held until explicit finalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    pub tasks: usize,
    pub terminal_results: usize,
    pub bytes: usize,
    pub parked_payloads: usize,
    pub pins: usize,
    pub wakeups: u64,
    pub retirement_work: usize,
}

impl Footprint {
    pub const fn empty() -> Self {
        Self {
            tasks: 0,
            terminal_results: 0,
            bytes: 0,
            parked_payloads: 0,
            pins: 0,
            wakeups: 0,
            retirement_work: 0,
        }
    }

    #[expect(
        tigerstyle::raw_arithmetic_overflow,
        reason = "Owner: noble-maintainers; only observation and prepare add privately admitted reservations: retained totals fit validated limits, and preflight adds at most one bounded request. Bytes are at most 2 * MAX_BYTES, wakeups at most 2 * MAX_WAKEUPS, pins at most 257 * 64 and retirement work at most 257 * 132; all fit their integer types."
    )]
    pub(super) const fn add(self, next: Self) -> Self {
        Self {
            tasks: self.tasks + next.tasks,
            terminal_results: self.terminal_results + next.terminal_results,
            bytes: self.bytes + next.bytes,
            parked_payloads: self.parked_payloads + next.parked_payloads,
            pins: self.pins + next.pins,
            wakeups: self.wakeups + next.wakeups,
            retirement_work: self.retirement_work + next.retirement_work,
        }
    }

    pub(super) const fn fits(self, limits: Limits) -> Result<(), super::Error> {
        if self.tasks > limits.tasks {
            return Err(super::Error::TaskCapacity);
        }
        if self.terminal_results > limits.terminal_results {
            return Err(super::Error::TerminalCapacity);
        }
        if self.bytes > limits.bytes {
            return Err(super::Error::ByteCapacity);
        }
        if self.parked_payloads > limits.parked_payloads {
            return Err(super::Error::ParkedCapacity);
        }
        if self.pins > limits.pins {
            return Err(super::Error::PinCapacity);
        }
        if self.wakeups > limits.wakeups {
            return Err(super::Error::WakeCapacity);
        }
        if self.retirement_work > limits.retirement_work {
            return Err(super::Error::RetirementCapacity);
        }
        Ok(())
    }
}

impl Request {
    pub const fn footprint(self) -> Result<Footprint, super::Error> {
        if self.inputs > MAX_OBLIGATIONS || self.results > MAX_OBLIGATIONS {
            return Err(super::Error::InvalidRequest);
        }
        if self.pins > MAX_OBLIGATIONS || self.native.0 == 0 {
            return Err(super::Error::InvalidRequest);
        }
        if self.input_bytes > MAX_BYTES
            || self.result_bytes > MAX_BYTES
            || self.parked_bytes > MAX_BYTES
        {
            return Err(super::Error::InvalidRequest);
        }
        if self.wakeups as u64 > MAX_WAKEUPS {
            return Err(super::Error::InvalidRequest);
        }
        let footprint = self.validated_footprint();
        if footprint.bytes > MAX_BYTES {
            return Err(super::Error::ByteCapacity);
        }
        Ok(footprint)
    }

    /// Used only after per-field bounds validation or for privately retained
    /// requests that passed admission. No caller can import a table snapshot.
    #[expect(
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-maintainers; each const usize conversion widens a u8 obligation count, which is representable on every Rust target; From<u8> is not const on the pinned compiler."
    )]
    #[expect(
        tigerstyle::raw_arithmetic_overflow,
        reason = "Owner: noble-maintainers; footprint validates each byte field against MAX_BYTES before calling this helper, and observation only reads admitted reservations. The three byte fields sum to at most 3 * MAX_BYTES; two obligation counts of at most 64, three buffer flags and one terminal result require at most 132 retirement steps."
    )]
    pub(super) const fn validated_footprint(self) -> Footprint {
        let bytes = self.input_bytes + self.result_bytes + self.parked_bytes;
        let input_buffer = if self.input_bytes == 0 { 0 } else { 1 };
        let result_buffer = if self.result_bytes == 0 { 0 } else { 1 };
        let parked = if self.parked_bytes == 0 { 0 } else { 1 };
        Footprint {
            tasks: 1,
            terminal_results: 1,
            bytes,
            parked_payloads: parked,
            pins: self.pins as usize,
            wakeups: self.wakeups as u64,
            retirement_work: self.inputs as usize
                + self.results as usize
                + input_buffer
                + result_buffer
                + parked
                + 1,
        }
    }
}

#[expect(
    tigerstyle::platform_dependent_cast,
    reason = "Owner: noble-maintainers; MAX_OBLIGATIONS is the u8 constant 64, representable as usize on every Rust target; the const limit checks preserve the public const footprint contract."
)]
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; only compile-time ceilings are multiplied: MAX_SLOTS * MAX_OBLIGATIONS is 16_384 and MAX_SLOTS * (2 * MAX_OBLIGATIONS + 4) is 33_792, both representable as usize."
)]
pub(super) const fn validate_limits(limits: Limits) -> Result<(), super::Error> {
    if limits.tasks == 0 || limits.tasks > MAX_SLOTS {
        return Err(super::Error::InvalidLimits);
    }
    if limits.terminal_results > MAX_SLOTS
        || limits.bytes > MAX_BYTES
        || limits.parked_payloads > MAX_SLOTS
    {
        return Err(super::Error::InvalidLimits);
    }
    if limits.pins > MAX_SLOTS * MAX_OBLIGATIONS as usize
        || limits.wakeups > MAX_WAKEUPS
        || limits.retirement_work > MAX_SLOTS * (2 * MAX_OBLIGATIONS as usize + 4)
    {
        return Err(super::Error::InvalidLimits);
    }
    Ok(())
}

#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; the subtraction is reached only for counts 1 through 63, so the shifted u64 is nonzero and subtracting one cannot underflow."
)]
pub(super) const fn mask(count: u8) -> u64 {
    if count == 0 {
        0
    } else if count >= MAX_OBLIGATIONS {
        u64::MAX
    } else {
        (1_u64 << count) - 1
    }
}
