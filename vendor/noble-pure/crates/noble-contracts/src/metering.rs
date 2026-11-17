#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; Meter contains only limits and request-local work/node counters; these helpers perform checked work subtraction and limit-guarded node updates without access to source or published compiler state."
)]

#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
impl crate::Meter {
    pub const fn new(limits: crate::Limits) -> Self {
        Self {
            limits,
            initial_work: limits.work,
            work: limits.work,
            nodes: 0,
            pool: None,
        }
    }

    /// A meter whose work also draws from `pool`, the remaining work of the
    /// request it belongs to. Without a pool it is an ordinary meter.
    pub fn pooled(
        limits: crate::Limits,
        pool: Option<&alloc::rc::Rc<core::cell::Cell<u32>>>,
    ) -> Self {
        let mut meter = Self::new(limits);
        if let Some(pool) = pool {
            let available = meter.work.min(pool.get());
            meter.initial_work = available;
            meter.work = available;
            meter.pool = Some(alloc::rc::Rc::clone(pool));
        }
        meter
    }

    /// The shared request pool this meter draws from, if any.
    pub fn pool(&self) -> Option<&alloc::rc::Rc<core::cell::Cell<u32>>> {
        self.pool.as_ref()
    }

    /// Work this meter can still charge: its own share, further capped by
    /// what remains of a shared request.
    pub fn available(&self) -> u32 {
        self.pool
            .as_ref()
            .map_or(self.work, |pool| self.work.min(pool.get()))
    }

    /// Work charged to this meter, excluding any other meters in its pool.
    pub const fn spent(&self) -> u32 {
        self.initial_work.saturating_sub(self.work)
    }

    pub fn charge(&mut self, amount: u32, span: crate::Span) -> Result<(), crate::Diagnostic> {
        let exhausted = || {
            crate::Diagnostic::new(
                crate::DiagnosticKind::Exhausted,
                span,
                "frontend work limit exceeded",
            )
        };
        let remaining = self.work.checked_sub(amount).ok_or_else(exhausted)?;
        if let Some(pool) = &self.pool {
            pool.set(pool.get().checked_sub(amount).ok_or_else(exhausted)?);
        }
        self.work = remaining;
        Ok(())
    }

    pub fn node(&mut self, span: crate::Span) -> Result<(), crate::Diagnostic> {
        attempt!(self.charge(1, span));
        if self.nodes >= self.limits.nodes {
            return Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Exhausted,
                span,
                "frontend node limit exceeded",
            ));
        }
        self.nodes += 1;
        Ok(())
    }

    pub fn depth(&mut self, depth: u32, span: crate::Span) -> Result<(), crate::Diagnostic> {
        attempt!(self.charge(1, span));
        if depth > self.limits.depth {
            return Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Exhausted,
                span,
                "frontend depth limit exceeded",
            ));
        }
        Ok(())
    }
}
