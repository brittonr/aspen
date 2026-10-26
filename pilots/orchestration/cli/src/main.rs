// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;

#[derive(Parser)]
#[command(version, about = "Single-controller, fenced pure-component batch orchestration")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}
#[derive(Clone, Copy, ValueEnum)]
enum RoleArg {
    Controller,
    Worker,
    Client,
}
#[derive(Clone, Copy, ValueEnum)]
enum KindArg {
    Component,
    Data,
}
#[derive(Args)]
struct Client {
    #[arg(long)]
    root: PathBuf,
    #[arg(long, help = "64-hex-public-key@IP:UDP-port")]
    target: String,
}
#[derive(Subcommand)]
enum Cmd {
    NewPolicy {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        allow_pure_components: bool,
    },
    Init {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, value_enum)]
        role: RoleArg,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long, default_value = "127.0.0.1:0")]
        bind: SocketAddr,
        #[arg(long)]
        controller: Option<String>,
        #[arg(long, default_value_t = 2000)]
        cpu_millis: u64,
        #[arg(long, default_value_t = 536870912)]
        memory_bytes: u64,
        #[arg(long, default_value_t = 2)]
        slots: u32,
    },
    Identity {
        #[arg(long)]
        root: PathBuf,
    },
    AllowOperator {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        peer: String,
    },
    AddWorker {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        peer: String,
        #[arg(long)]
        address: SocketAddr,
        #[arg(long, default_value_t = 2000)]
        cpu_millis: u64,
        #[arg(long, default_value_t = 536870912)]
        memory_bytes: u64,
        #[arg(long, default_value_t = 2)]
        slots: u32,
    },
    Revoke {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        component: String,
        #[arg(long)]
        undo: bool,
    },
    Serve {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        resume: bool,
    },
    Put {
        #[command(flatten)]
        client: Client,
        #[arg(long, value_enum)]
        kind: KindArg,
        #[arg(long)]
        file: PathBuf,
    },
    Get {
        #[command(flatten)]
        client: Client,
        #[arg(long)]
        id: String,
        #[arg(long)]
        out: PathBuf,
    },
    Submit {
        #[command(flatten)]
        client: Client,
        #[arg(long)]
        key: String,
        #[arg(long)]
        file: PathBuf,
    },
    Status {
        #[command(flatten)]
        client: Client,
        #[arg(long)]
        run: Option<String>,
        #[arg(long)]
        raw: bool,
    },
    Plan {
        #[command(flatten)]
        client: Client,
    },
    Cancel {
        #[command(flatten)]
        client: Client,
        #[arg(long)]
        run: String,
    },
    Drain {
        #[command(flatten)]
        client: Client,
        #[arg(long)]
        node: String,
        #[arg(long)]
        resume: bool,
    },
}
fn parse_id(s: &str) -> Result<Id> {
    Id::parse(s).map_err(|e| anyhow::anyhow!("{e}"))
}
fn read_file(path: &Path, max: usize) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    if !file.metadata()?.is_file() {
        bail!("input is not a regular file");
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max {
        bail!("input exceeds bound");
    }
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn root(path: &Path) -> Result<CapabilityRoot> {
    Ok(CapabilityRoot::open(path)?)
}
fn emit(value: serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
fn peer(s: &str) -> Result<PeerAddress> {
    let (id, address) = s.split_once('@').context("target requires key@IP:port")?;
    Ok(PeerAddress {
        id: parse_id(id)?,
        address: address.parse()?,
    })
}
async fn call(c: &Client, request: Request) -> Result<Reply> {
    let r = root(&c.root)?;
    let config: NodeConfig = r.read_json("config.json")?;
    config.validate()?;
    let endpoint = bind_endpoint(secret(&r)?, config.bind).await?;
    let result = rpc(&endpoint, &peer(&c.target)?, &request, Duration::from_millis(config.rpc_timeout_ms)).await;
    endpoint.close().await;
    Ok(result?)
}
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::signal;
        use tokio::signal::unix::SignalKind;
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {_=tokio::signal::ctrl_c()=>{},_=term.recv()=>{}}
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::NewPolicy {
            out,
            allow_pure_components,
        } => {
            // Public identity supplies a fresh namespace, not a signing capability.
            let key = iroh_key_for_namespace();
            let policy = Policy {
                profile: PROFILE.into(),
                cluster: key,
                generation: 1,
                allow_any_pure_component: allow_pure_components,
                allowed_components: BTreeSet::new(),
                max_memory_bytes: 268435456,
                max_fuel: 1_000_000_000,
                max_timeout_ms: 60_000,
                max_output_bytes: MAX_CONTENT_BYTES as u64,
            };
            write_new(&out, &serde_json::to_vec_pretty(&policy)?)?;
            emit(serde_json::json!({"cluster":policy.cluster,"policy_ref":policy_id(&policy)?}))?;
        }
        Cmd::Init {
            root: dir,
            role,
            policy,
            bind,
            controller,
            cpu_millis,
            memory_bytes,
            slots,
        } => {
            let policy: Policy = serde_json::from_slice(&read_file(&policy, 1024 * 1024)?)?;
            let role = match role {
                RoleArg::Controller => Role::Controller,
                RoleArg::Worker => Role::Worker,
                RoleArg::Client => Role::Client,
            };
            if role != Role::Client && bind.port() == 0 {
                bail!("servers require an explicit UDP port");
            }
            let config = NodeConfig {
                role,
                bind,
                cluster: policy.cluster,
                generation: policy.generation,
                controller: controller.as_deref().map(parse_id).transpose()?,
                operators: Vec::new(),
                workers: Vec::new(),
                capacity: Resources {
                    cpu_millis,
                    memory_bytes,
                    slots,
                },
                rpc_timeout_ms: 10_000,
                tick_ms: 200,
                freshness_ticks: 3,
            };
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder
                .create(&dir)
                .context("root must be a new directory; initialization never replaces existing state")?;
            let id = initialize(&root(&dir)?, &config, &policy)?;
            emit(serde_json::json!({"id":id,"address":bind,"role":role}))?;
        }
        Cmd::Identity { root: dir } => emit(serde_json::json!({"id":public_id(&root(&dir)?)?}))?,
        Cmd::AllowOperator { root: dir, peer } => {
            let r = root(&dir)?;
            let mut config: NodeConfig = r.read_json("config.json")?;
            if config.role != Role::Controller {
                bail!("operator enrollment belongs to controller root");
            }
            let id = parse_id(&peer)?;
            if !config.operators.contains(&id) {
                config.operators.push(id);
                config.operators.sort();
            }
            config.validate()?;
            r.replace("config.json", &serde_json::to_vec_pretty(&config)?)?;
            emit(serde_json::json!({"restart_required":true}))?;
        }
        Cmd::AddWorker {
            root: dir,
            peer,
            address,
            cpu_millis,
            memory_bytes,
            slots,
        } => {
            let r = root(&dir)?;
            let mut config: NodeConfig = r.read_json("config.json")?;
            let id = parse_id(&peer)?;
            if config.role != Role::Controller || config.workers.iter().any(|w| w.peer.id == id) {
                bail!("wrong role or duplicate worker");
            }
            config.workers.push(Enrollment {
                peer: PeerAddress { id, address },
                capacity: Resources {
                    cpu_millis,
                    memory_bytes,
                    slots,
                },
                labels: BTreeMap::new(),
            });
            config.validate()?;
            r.replace("config.json", &serde_json::to_vec_pretty(&config)?)?;
            emit(serde_json::json!({"restart_required":true}))?;
        }
        Cmd::Revoke {
            root: dir,
            component,
            undo,
        } => {
            let r = root(&dir)?;
            let mut revoked: BTreeSet<Id> = r.read_json("revoked.json")?;
            let id = parse_id(&component)?;
            if undo {
                revoked.remove(&id);
            } else {
                revoked.insert(id);
            }
            r.replace("revoked.json", &serde_json::to_vec_pretty(&revoked)?)?;
            emit(serde_json::json!({"revoked":!undo,"component":id,"active_work_requires_explicit_cancel":true}))?;
        }
        Cmd::Serve { root: dir, resume } => {
            let running = open_service(root(&dir)?, resume).await?;
            eprintln!("orchestrator ready id={}", endpoint_id(&running.endpoint));
            let (tx, rx) = tokio::sync::watch::channel(false);
            tokio::spawn(async move {
                shutdown_signal().await;
                let _ = tx.send(true);
            });
            running.run(rx).await?;
        }
        Cmd::Put { client, kind, file } => {
            let kind = match kind {
                KindArg::Component => ContentKind::Component,
                KindArg::Data => ContentKind::Data,
            };
            let c = content(kind, read_file(&file, MAX_CONTENT_BYTES)?)?;
            let id = content_id(&c)?;
            let reply = call(&client, Request::Put {
                id,
                content: WireContent::from_content(c)?,
            })
            .await?;
            match reply {
                Reply::Id(got) if got == id => emit(serde_json::json!({"id":id}))?,
                _ => bail!("unexpected put response"),
            }
        }
        Cmd::Get { client, id, out } => {
            let id = parse_id(&id)?;
            match call(&client, Request::Get { id }).await? {
                Reply::Content(c) => {
                    let c = c.content()?;
                    if content_id(&c)? != id {
                        bail!("content identity mismatch");
                    }
                    write_new(&out, &c.bytes)?;
                    emit(serde_json::json!({"id":id,"bytes":c.bytes.len()}))?;
                }
                _ => bail!("unexpected get response"),
            }
        }
        Cmd::Submit { client, key, file } => {
            let spec: JobSpec = serde_json::from_slice(&read_file(&file, 1024 * 1024)?)?;
            match call(&client, Request::Submit { key, spec }).await? {
                Reply::Id(run) => emit(serde_json::json!({"run":run}))?,
                _ => bail!("unexpected submit response"),
            }
        }
        Cmd::Status { client, run, raw } => {
            let Reply::State(state) = call(&client, Request::Status).await? else {
                bail!("unexpected status response")
            };
            if raw {
                emit(serde_json::to_value(state)?)?;
            } else if let Some(id) = run {
                let r = state.runs.get(&parse_id(&id)?).context("run not found")?;
                emit(serde_json::json!({"run":r,"phase":r.phase()}))?;
            } else {
                emit(
                    serde_json::json!({"revision":state.revision,"logical_tick":state.tick,"nodes":state.nodes,"runs":state.runs.values().map(|r|serde_json::json!({"id":r.id,"name":r.spec.name,"phase":r.phase()})).collect::<Vec<_>>(),"pending":explain(&state)}),
                )?;
            }
        }
        Cmd::Plan { client } => {
            let Reply::State(state) = call(&client, Request::Status).await? else {
                bail!("unexpected status response")
            };
            emit(serde_json::json!({"pending":explain(&state),"advisory_only":true}))?;
        }
        Cmd::Cancel { client, run } => {
            let reply = call(&client, Request::CancelRun { run: parse_id(&run)? }).await?;
            emit(serde_json::to_value(reply)?)?;
        }
        Cmd::Drain { client, node, resume } => {
            let reply = call(&client, Request::DrainNode {
                node: parse_id(&node)?,
                draining: !resume,
            })
            .await?;
            emit(serde_json::to_value(reply)?)?;
        }
    }
    Ok(())
}
