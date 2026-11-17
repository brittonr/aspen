pub(super) struct Schedule<'a> {
    session: Option<crate::source::declared::ModuleSession>,
    work: super::Work<'a>,
    names: alloc::vec::Vec<alloc::string::String>,
    pending: alloc::vec::Vec<Option<super::Definition>>,
    installed: alloc::vec::Vec<bool>,
    remaining: usize,
    limits: Option<crate::Limits>,
    retained_bytes: u32,
}

struct PassPosition {
    index: usize,
    completed: usize,
}

impl<'a> Schedule<'a> {
    pub(super) fn setup(
        session: crate::source::declared::ModuleSession,
        mut work: super::Work<'a>,
        limits: Option<crate::Limits>,
        retained_bytes: u32,
    ) -> Result<Self, crate::source::Error> {
        let session = attempt!(super::reserve_words(session, work.definitions.len()));
        let count = work.definitions.len();
        let mut names = alloc::vec::Vec::with_capacity(count);
        let mut at = 0;
        while at < count {
            names.push(work.definitions[at].0.clone());
            at += 1;
        }
        let mut pending = alloc::vec::Vec::with_capacity(count);
        pending.resize_with(count, || None);
        let mut definitions = core::mem::take(&mut work.definitions);
        let mut at = count;
        while at != 0 {
            at -= 1;
            pending[at] = definitions.pop();
        }
        let remaining = pending.len();
        Ok(Self {
            session: Some(session),
            work,
            names,
            pending,
            installed: alloc::vec![false;remaining],
            remaining,
            limits,
            retained_bytes,
        })
    }

    pub(super) fn run(
        mut self,
    ) -> Result<(crate::source::declared::ModuleSession, super::Work<'a>), crate::source::Error>
    {
        let mut failure = None;
        while self.remaining != 0 && failure.is_none() {
            let (next, progress) = self.run_pass();
            self = next;
            if let Err(problem) = progress {
                failure = Some(problem);
            }
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        let session = attempt!(self.session.ok_or_else(|| {
            crate::source::declared::error(crate::source::Stage::Check, "missing module session")
        }));
        Ok((session, self.work))
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0015 Schedule::pass and declared::error plus E0493 owned result prevent const on both pinned Rust compilers; reassess const graph APIs."
    )]
    fn run_pass(mut self) -> (Self, Result<(), crate::source::Error>) {
        let (next, progress) = self.pass();
        self = next;
        match progress {
            Ok(completed) if completed != 0 => {
                self.remaining -= completed;
                (self, Ok(()))
            }
            Ok(_) => (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Resolve,
                    "cyclic local module definitions",
                )),
            ),
            Err(problem) => (self, Err(problem)),
        }
    }

    fn pass(mut self) -> (Self, Result<usize, crate::source::Error>) {
        let mut completed = 0;
        let mut position = 0;
        let count = self.pending.len();
        let mut failure = None;
        while position < count && failure.is_none() {
            let (next, progress) = self.pass_at(PassPosition {
                index: position,
                completed,
            });
            self = next;
            match progress {
                Ok(advanced) => completed = advanced,
                Err(problem) => failure = Some(problem),
            }
            position += 1;
        }
        let result = match failure {
            Some(problem) => Err(problem),
            None => Ok(completed),
        };
        (self, result)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0277/E0658 Vec Index and IndexMut plus E0493 owned result prevent const on both pinned Rust compilers; reassess const indexing."
    )]
    fn pass_at(mut self, position: PassPosition) -> (Self, Result<usize, crate::source::Error>) {
        let PassPosition {
            index: position,
            completed,
        } = position;
        if self.installed[position] {
            return (self, Ok(completed));
        }
        let (next, ready) = self.ready(position);
        self = next;
        match ready {
            Ok(true) => {
                let (next, installed) = self.commit(position);
                self = next;
                match installed {
                    Ok(()) => {
                        let completed = match completed.checked_add(1) {
                            Some(completed) => completed,
                            None => {
                                return (
                                    self,
                                    Err(crate::source::declared::error(
                                        crate::source::Stage::Check,
                                        "definition count overflow",
                                    )),
                                )
                            }
                        };
                        self.installed[position] = true;
                        (self, Ok(completed))
                    }
                    Err(problem) => (self, Err(problem)),
                }
            }
            Ok(false) => (self, Ok(completed)),
            Err(problem) => (self, Err(problem)),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0277/E0658 Vec Index and Deref plus E0015 blocked and declared::error prevent const on both pinned Rust compilers; reassess const graph APIs."
    )]
    fn ready(mut self, position: usize) -> (Self, Result<bool, crate::source::Error>) {
        let Some((name, _, tree)) = self.pending[position].as_ref() else {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Check,
                    "missing definition graph node",
                )),
            );
        };
        let is_signed_owner = self
            .session
            .as_ref()
            .and_then(|session| session.source.declared.as_ref())
            .is_some_and(|context| context.signed_self)
            && self
                .work
                .signatures
                .iter()
                .any(|(declared, _)| declared == name);
        let (meter, blocked) = super::scheduling::blocked(
            tree,
            &self.names,
            &self.installed,
            is_signed_owner.then_some(name.as_str()),
            self.work.meter,
        );
        self.work.meter = meter;
        let result = match blocked {
            Ok(is_blocked) => Ok(!is_blocked),
            Err(problem) => Err(problem),
        };
        (self, result)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; E0277/E0658 Vec IndexMut, Option::ok_or_else and mem::take plus E0493 owned state prevent const on both pinned Rust compilers; reassess const ownership APIs."
    )]
    fn commit(mut self, position: usize) -> (Self, Result<(), crate::source::Error>) {
        let (name, definition_bytes, _) = match self.pending[position].take().ok_or_else(|| {
            crate::source::declared::error(
                crate::source::Stage::Check,
                "missing ready module definition",
            )
        }) {
            Ok(definition) => definition,
            Err(problem) => return (self, Err(problem)),
        };
        let session = match self.session.take().ok_or_else(|| {
            crate::source::declared::error(crate::source::Stage::Check, "missing module session")
        }) {
            Ok(session) => session,
            Err(problem) => return (self, Err(problem)),
        };
        let installed = super::install_one(
            session,
            super::DefinitionInput {
                signature: self
                    .work
                    .signatures
                    .iter()
                    .find(|(declared, _)| declared == &name)
                    .map(|(_, scheme)| scheme.clone()),
                name,
                source: definition_bytes,
                limits: self.limits,
                retained_bytes: self.retained_bytes,
                exports: self.work.exports,
            },
            core::mem::take(&mut self.work.local_exports),
            &mut self.work.meter,
        );
        let (session, local_exports) = match installed {
            Ok(installed) => installed,
            Err(problem) => return (self, Err(problem)),
        };
        self.session = Some(session);
        self.work.local_exports = local_exports;
        (self, Ok(()))
    }
}
