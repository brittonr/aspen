//! Shared real owner/work preparation for the baseline and boundary-failure cases.

mod admission;
mod completion;
mod replacement;
mod uncommitted;
mod unconfigured;

#[derive(Clone, Copy)]
pub(super) enum Case {
    Unconfigured,
    MissingCredential,
    StaleRoots,
    Denied,
    Queued,
    CompletionFailure,
}

impl Case {
    pub(super) const fn all() -> [Self; 6] {
        [
            Self::Unconfigured,
            Self::MissingCredential,
            Self::StaleRoots,
            Self::Denied,
            Self::Queued,
            Self::CompletionFailure,
        ]
    }
    pub(super) const fn label(&self) -> &'static str {
        match self {
            Self::Unconfigured => "collaboration-native-unconfigured",
            Self::MissingCredential => "collaboration-native-missing-credential",
            Self::StaleRoots => "collaboration-native-stale-roots",
            Self::Denied => "collaboration-native-denied",
            Self::Queued => "collaboration-native-uncommitted",
            Self::CompletionFailure => "collaboration-native-completion-failure",
        }
    }

    pub(super) fn frame_budget(&self) -> crate::producer::Result<Option<u64>> {
        match self {
            Self::Queued => Ok(Some(crate::producer::uncommitted_frame_budget()?)),
            Self::Unconfigured
            | Self::MissingCredential
            | Self::StaleRoots
            | Self::Denied
            | Self::CompletionFailure => Ok(None),
        }
    }

    pub(super) fn run(self, cohort: &crate::producer::Cohort) -> crate::producer::Result<preserves::IOValue> {
        let observed = match self {
            Self::Unconfigured => return unconfigured::run(cohort),
            Self::MissingCredential => admission::run(cohort, admission::inputs::Mode::Missing),
            Self::StaleRoots => admission::run(cohort, admission::inputs::Mode::Stale),
            Self::Denied => admission::run(cohort, admission::inputs::Mode::Malformed),
            Self::Queued => uncommitted::queued(cohort),
            Self::CompletionFailure => completion::post_effect_failure(cohort),
        }?;
        replacement::observe(cohort, observed)
    }
}

pub(super) struct Observed {
    pub actor: usize,
    pub work_ref: String,
    pub next_tick: u64,
    pub value: preserves::IOValue,
}

pub(super) struct Session {
    pub nodes: Vec<super::Node>,
    pub member: super::operations::Member,
    pub intent: super::operations::Intent,
    pub work: super::operations::Work,
    pub claim_ref: String,
    pub native_status: preserves::IOValue,
}

impl Session {
    /// The builder receives only actual diagnostic data, never a current-state authority marker.
    pub(super) fn open<F>(cohort: &crate::producer::Cohort, build: F) -> crate::producer::Result<Self>
    where F: FnOnce(
            &crate::authority::Issuer,
            molten::collaboration_scope::native_runtime::Observation,
        ) -> crate::producer::Result<super::operations::Intent> {
        let nodes = start_nodes(cohort, None)?;
        let (actor, tick, observation) = super::scope::select(&nodes, cohort, &[0, 1, 2])?;
        let (member, native_status) = super::status::membership(&nodes[actor], &cohort.nodes[actor], tick + 1)?;
        let intent = build(&nodes[actor].issuer, observation)?;
        for fixture in &cohort.nodes {
            super::scope::publish_command(fixture, &intent)?;
        }
        let work = super::flow::submit(&nodes[actor], &cohort.nodes[actor], &member, &intent, member.tick + 1)?;
        let claim_ref = super::flow::readmit(&nodes[actor], &cohort.nodes[actor], &work, work.submit_tick + 1)?;
        let mut session = Self {
            nodes,
            member,
            intent,
            work,
            claim_ref,
            native_status,
        };
        session.require_live()?;
        Ok(session)
    }

    pub(super) fn delivery<'a>(&'a self, cohort: &'a crate::producer::Cohort) -> super::flow::Delivery<'a> {
        let actor = self.work.actor;
        super::flow::Delivery {
            node: &self.nodes[actor],
            fixture: &cohort.nodes[actor],
            member: &self.member,
            work: &self.work,
            intent: &self.intent,
            claim_ref: &self.claim_ref,
            tick: self.work.submit_tick + 2,
        }
    }

    pub(super) fn require_live(&mut self) -> crate::producer::Result<()> {
        for node in &mut self.nodes {
            if let Some(status) = node.child.try_wait()? {
                return Err(format!("node {} exited during its owned consumer session: {status}", node.index).into());
            }
        }
        Ok(())
    }

    pub(super) fn finish(&mut self) -> crate::producer::Result<()> {
        for node in &mut self.nodes {
            super::finish(node)?;
        }
        Ok(())
    }
}

pub(super) fn retain(
    observation: &preserves::IOValue,
    root: Option<&molten::node_state::DirectoryRoot>,
) -> crate::producer::Result<String> {
    let bytes = molten::preserves_rail::canonical_bytes(observation)?;
    let reference = molten::preserves_rail::content_ref_from_bytes(&bytes);
    if let Some(root) = root {
        root.write_new(&molten::node_state::RelativePath::parse(&format!("{}.preserves", &reference[7..]))?, &bytes)?;
    }
    Ok(reference)
}

/// Each case selects resources explicitly before its first actual serving process starts.
pub(super) fn start_nodes(
    cohort: &crate::producer::Cohort,
    unconfigured: Option<usize>,
) -> crate::producer::Result<Vec<super::Node>> {
    let mut prepared = Vec::with_capacity(cohort.nodes.len());
    for (position, node) in cohort.nodes.iter().enumerate() {
        let index = u64::try_from(position).map_err(|_| "cohort position exceeds the admitted bound")?;
        let issuer = crate::authority::Issuer::new(super::operations::seed(index)?)?;
        super::setup::prepare(&cohort.signed_main, node, &issuer, index, 1)?;
        if unconfigured == Some(position) {
            node.root.remove_regular_file(&molten::node_state::RelativePath::parse(crate::producer::EFFECTS)?)?;
        }
        prepared.push((node, index, issuer));
    }
    let mut nodes = Vec::with_capacity(prepared.len());
    for (node, index, issuer) in prepared {
        nodes.push(super::start(&cohort.signed_main, node, index, issuer)?);
    }
    for node in &mut nodes {
        super::await_ready(node)?;
    }
    Ok(nodes)
}
