type Outcome<T> = molten::error::Result<T>;
type FilePath = std::path::PathBuf;
#[derive(Debug, Default, clap::Args)]
pub(crate) struct SourceArgs {
    #[arg(long, requires_all = ["receiver_state_root", "session_generation"])]
    pub sender_state_root: Option<FilePath>,
    #[arg(long, requires_all = ["sender_state_root", "session_generation"])]
    pub receiver_state_root: Option<FilePath>,
    #[arg(long, requires_all = ["sender_state_root", "receiver_state_root"])]
    pub session_generation: Option<u64>,
}

/// Explicit local recording, never a network receiver acknowledgement.
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Admit(Roots),
    Commit {
        #[command(flatten)]
        roots: Roots,
        #[arg(long)]
        generation: u64,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        payload: FilePath,
    },
    Close {
        #[command(flatten)]
        roots: Roots,
        #[arg(long)]
        generation: u64,
    },
    Receive {
        #[arg(long)]
        receiver_state_root: FilePath,
        #[arg(long)]
        generation: u64,
        #[arg(long)]
        envelope: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}

#[derive(Debug, clap::Args)]
pub(crate) struct Roots {
    #[arg(long)]
    sender_state_root: FilePath,
    #[arg(long)]
    receiver_state_root: FilePath,
    #[arg(long)]
    topic: String,
    #[arg(long)]
    receipt_out: Option<FilePath>,
}

pub(crate) struct Source {
    store: molten::remote_dataspace::RecordedSessionStore,
    sender: molten::node_state::DirectoryRoot,
    generation: u64,
}

impl Source {
    pub(crate) fn view(&self, topic: &str) -> Outcome<molten::remote_dataspace::SenderSessionView<'_>> {
        self.store.sender_view(&self.sender, topic, self.generation)
    }

    pub(crate) fn receive(
        &self,
        envelope: &molten::remote_dataspace::Envelope,
    ) -> Outcome<molten::remote_dataspace::RecordedSessionOutcome> {
        self.store.receive_with_evidence(self.generation, envelope)
    }
}

pub(crate) fn source(args: &SourceArgs) -> Outcome<Option<Source>> {
    match (&args.sender_state_root, &args.receiver_state_root, args.session_generation) {
        (None, None, None) => Ok(None),
        (Some(sender), Some(receiver), Some(generation)) => {
            let sender = molten::node_state::DirectoryRoot::open_existing(sender)?;
            let receiver = molten::node_state::DirectoryRoot::open_existing(receiver)?;
            let store = molten::remote_dataspace::RecordedSessionStore::open(&receiver)?;
            Ok(Some(Source {
                store,
                sender,
                generation,
            }))
        }
        _ => Err(molten::error::Failure::invalid_harness(
            "sender-state-root, receiver-state-root and session-generation must be supplied together",
        )),
    }
}

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Admit(roots) => {
            let (sender, store) = open(&roots)?;
            emit_commit(store.admit_local(&sender, &roots.topic)?, roots.receipt_out.as_ref())
        }
        Command::Commit {
            roots,
            generation,
            operation,
            payload,
        } => {
            let (sender, store) = open(&roots)?;
            let sender_id = identity(&sender)?;
            let payload = super::io::read_preserves_file(&payload)?;
            let operation = super::ops::parse_remote_operation(&operation)?;
            let outcome = store.commit_with_evidence(&sender_id, &roots.topic, generation, operation, &payload)?;
            emit_outcome(outcome, roots.receipt_out.as_ref())
        }
        Command::Close { roots, generation } => {
            let (sender, store) = open(&roots)?;
            let sender_id = identity(&sender)?;
            emit_commit(store.close(&sender_id, &roots.topic, generation)?, roots.receipt_out.as_ref())
        }
        Command::Receive {
            receiver_state_root,
            generation,
            envelope,
            receipt_out,
        } => {
            let receiver = molten::node_state::DirectoryRoot::open_existing(&receiver_state_root)?;
            let store = molten::remote_dataspace::RecordedSessionStore::open(&receiver)?;
            let value = super::io::read_preserves_file(&envelope)?;
            let envelope = molten::remote_dataspace::parse_envelope(&value)?;
            emit_outcome(store.receive_with_evidence(generation, &envelope)?, receipt_out.as_ref())
        }
    }
}

fn open(roots: &Roots) -> Outcome<(molten::node_state::DirectoryRoot, molten::remote_dataspace::RecordedSessionStore)> {
    let sender = molten::node_state::DirectoryRoot::open_existing(&roots.sender_state_root)?;
    let receiver = molten::node_state::DirectoryRoot::open_existing(&roots.receiver_state_root)?;
    Ok((sender, molten::remote_dataspace::RecordedSessionStore::open(&receiver)?))
}

fn identity(root: &molten::node_state::DirectoryRoot) -> Outcome<String> {
    let relative = molten::node_state::RelativePath::parse("identity.preserves")?;
    let text = root.read_to_string(&relative, 1_048_576)?;
    let value = molten::preserves_rail::parse_text(&text)?;
    Ok(molten::node_identity::parse_identity(&value)?.node_id)
}

fn emit_commit(commit: molten::remote_dataspace::RecordedSessionCommit, out: Option<&FilePath>) -> Outcome<()> {
    super::io::emit_named_receipt(out, "local recorded session receipt", &commit.receipt_value)?;
    println!(
        "local recorded session={} generation={} observation={}",
        commit.session_ref, commit.generation, commit.observation_ref
    );
    Ok(())
}

fn emit_outcome(outcome: molten::remote_dataspace::RecordedSessionOutcome, out: Option<&FilePath>) -> Outcome<()> {
    match outcome {
        molten::remote_dataspace::RecordedSessionOutcome::Committed(commit) => emit_commit(commit, out),
        molten::remote_dataspace::RecordedSessionOutcome::Denied {
            receipt_value,
            diagnostics,
        } => {
            super::io::emit_named_receipt(out, "local recorded session denial", &receipt_value)?;
            Err(molten::error::Failure::invalid_harness(diagnostics.join("; ")))
        }
    }
}
