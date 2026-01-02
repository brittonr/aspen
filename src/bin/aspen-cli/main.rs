//! Aspen CLI - Command-line client for Aspen clusters
//!
//! Supports blob upload, job submission, and cluster management.

use anyhow::{Context, Result};
use aspen_client::AspenClient;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "aspen")]
#[command(about = "Aspen cluster command-line client")]
struct Cli {
    /// Cluster endpoint (Iroh address)
    #[arg(short, long, default_value = "iroh://127.0.0.1:7701")]
    endpoint: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Blob storage operations
    Blob {
        #[command(subcommand)]
        action: BlobAction,
    },

    /// Job management
    Job {
        #[command(subcommand)]
        action: JobAction,
    },

    /// VM-specific job operations
    Vm {
        #[command(subcommand)]
        action: VmAction,
    },

    /// Cluster management
    Cluster {
        #[command(subcommand)]
        action: ClusterAction,
    },
}

#[derive(Subcommand)]
enum BlobAction {
    /// Upload a file to blob storage
    Add {
        /// Path to file to upload
        file: PathBuf,

        /// Tag to protect blob from GC
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Get a blob by hash
    Get {
        /// Blob hash (BLAKE3)
        hash: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List blob information
    Info {
        /// Blob hash
        hash: String,
    },
}

#[derive(Subcommand)]
enum JobAction {
    /// Submit a generic job
    Submit {
        /// Job type
        #[arg(short = 't', long)]
        job_type: String,

        /// Job payload (JSON)
        #[arg(short, long)]
        payload: String,

        /// Job priority (0-255)
        #[arg(short = 'p', long, default_value = "128")]
        priority: u8,

        /// Timeout in seconds
        #[arg(long, default_value = "60")]
        timeout: u64,
    },

    /// Get job status
    Status {
        /// Job ID
        job_id: String,
    },

    /// List jobs
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Limit results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum VmAction {
    /// Submit a VM job with a binary
    Submit {
        /// Path to VM binary (will be uploaded to blob store)
        binary: PathBuf,

        /// Input data for the VM
        #[arg(short, long)]
        input: Option<String>,

        /// Binary format (elf, wasm, etc.)
        #[arg(short, long, default_value = "elf")]
        format: String,

        /// Job timeout in seconds
        #[arg(long, default_value = "60")]
        timeout: u64,

        /// Tag for the job
        #[arg(short = 't', long)]
        tag: Option<String>,
    },

    /// Submit a VM job with existing blob
    SubmitBlob {
        /// Blob hash of the binary
        hash: String,

        /// Size of the binary
        size: u64,

        /// Binary format
        #[arg(short, long, default_value = "elf")]
        format: String,

        /// Input data
        #[arg(short, long)]
        input: Option<String>,
    },
}

#[derive(Subcommand)]
enum ClusterAction {
    /// Get cluster status
    Status,

    /// List nodes in cluster
    Nodes,

    /// Get cluster metrics
    Metrics,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let cli = Cli::parse();

    // For now, we'll show what would be done
    // Full implementation requires AspenClient with blob support

    match cli.command {
        Commands::Blob { action } => handle_blob(action, &cli.endpoint).await,
        Commands::Job { action } => handle_job(action, &cli.endpoint).await,
        Commands::Vm { action } => handle_vm(action, &cli.endpoint).await,
        Commands::Cluster { action } => handle_cluster(action, &cli.endpoint).await,
    }
}

async fn handle_blob(action: BlobAction, endpoint: &str) -> Result<()> {
    println!("Connecting to cluster at {}", endpoint);

    match action {
        BlobAction::Add { file, tag } => {
            let data = fs::read(&file)
                .with_context(|| format!("Failed to read file: {:?}", file))?;

            println!("Uploading {} ({} bytes)", file.display(), data.len());

            // Calculate hash locally for display
            let hash = blake3::hash(&data);
            println!("BLAKE3 hash: {}", hash);

            if let Some(tag) = tag {
                println!("Protection tag: {}", tag);
            }

            // TODO: Actually upload via AspenClient
            println!("\n# To upload via client:");
            println!("let client = AspenClient::connect(\"{}\").await?;", endpoint);
            println!("let blob_store = client.blob_store();");
            println!("let result = blob_store.add_bytes(&data).await?;");
            println!("println!(\"Uploaded: {{}}\", result.blob_ref.hash);");
        }

        BlobAction::Get { hash, output } => {
            println!("Retrieving blob: {}", hash);

            if let Some(output) = output {
                println!("Output to: {:?}", output);
            }

            // TODO: Actually retrieve via AspenClient
            println!("\n# To retrieve via client:");
            println!("let hash = \"{}\".parse::<Hash>()?;", hash);
            println!("let bytes = blob_store.get_bytes(&hash).await?;");
        }

        BlobAction::Info { hash } => {
            println!("Getting info for blob: {}", hash);

            // TODO: Get blob metadata
            println!("\n# To get info via client:");
            println!("let info = blob_store.info(&hash).await?;");
        }
    }

    Ok(())
}

async fn handle_job(action: JobAction, endpoint: &str) -> Result<()> {
    println!("Connecting to cluster at {}", endpoint);

    match action {
        JobAction::Submit { job_type, payload, priority, timeout } => {
            println!("Submitting job:");
            println!("  Type: {}", job_type);
            println!("  Priority: {}", priority);
            println!("  Timeout: {}s", timeout);
            println!("  Payload: {}", payload);

            // TODO: Submit via AspenClient
            println!("\n# To submit via client:");
            println!("let job_spec = JobSpec::new(\"{}\", payload);", job_type);
            println!("let job_id = client.submit_job(job_spec).await?;");
        }

        JobAction::Status { job_id } => {
            println!("Getting status for job: {}", job_id);

            // TODO: Query job status
            println!("\n# To get status via client:");
            println!("let status = client.get_job_status(\"{}\").await?;", job_id);
        }

        JobAction::List { status, limit } => {
            println!("Listing jobs (limit: {})", limit);
            if let Some(status) = status {
                println!("  Filtering by status: {}", status);
            }

            // TODO: List jobs
            println!("\n# To list via client:");
            println!("let jobs = client.list_jobs(limit).await?;");
        }
    }

    Ok(())
}

async fn handle_vm(action: VmAction, endpoint: &str) -> Result<()> {
    println!("Connecting to cluster at {}", endpoint);

    match action {
        VmAction::Submit { binary, input, format, timeout, tag } => {
            let data = fs::read(&binary)
                .with_context(|| format!("Failed to read binary: {:?}", binary))?;

            println!("VM Job Submission:");
            println!("  Binary: {} ({} bytes)", binary.display(), data.len());
            println!("  Format: {}", format);
            println!("  Timeout: {}s", timeout);

            // Calculate hash
            let hash = blake3::hash(&data);
            println!("  Binary hash: {}", hash);

            if let Some(input) = &input {
                println!("  Input: {}", input);
            }

            if let Some(tag) = &tag {
                println!("  Tag: {}", tag);
            }

            // Show the two-step process
            println!("\n# Step 1: Upload binary to blob store");
            println!("let blob_result = blob_store.add_bytes(&binary_data).await?;");

            println!("\n# Step 2: Submit VM job with blob reference");
            println!("let job_spec = JobSpec::with_blob_binary(");
            println!("    blob_result.blob_ref.hash.to_string(),");
            println!("    blob_result.blob_ref.size,");
            println!("    \"{}\"", format);
            println!(");");

            if let Some(input) = input {
                println!("job_spec.payload[\"input\"] = json!(\"{}\");", input);
            }

            println!("let job_id = client.submit_job(job_spec).await?;");
        }

        VmAction::SubmitBlob { hash, size, format, input } => {
            println!("VM Job with existing blob:");
            println!("  Hash: {}", hash);
            println!("  Size: {} bytes", size);
            println!("  Format: {}", format);

            if let Some(input) = &input {
                println!("  Input: {}", input);
            }

            println!("\n# To submit via client:");
            println!("let job_spec = JobSpec::with_blob_binary(");
            println!("    \"{}\",", hash);
            println!("    {},", size);
            println!("    \"{}\"", format);
            println!(");");

            if let Some(input) = input {
                println!("job_spec.payload[\"input\"] = json!(\"{}\");", input);
            }
        }
    }

    Ok(())
}

async fn handle_cluster(action: ClusterAction, endpoint: &str) -> Result<()> {
    println!("Connecting to cluster at {}", endpoint);

    match action {
        ClusterAction::Status => {
            println!("Getting cluster status...");

            // TODO: Get cluster state
            println!("\n# To get status via client:");
            println!("let state = client.current_state().await?;");
        }

        ClusterAction::Nodes => {
            println!("Listing cluster nodes...");

            // TODO: List nodes
            println!("\n# To list nodes via client:");
            println!("let nodes = client.get_nodes().await?;");
        }

        ClusterAction::Metrics => {
            println!("Getting cluster metrics...");

            // TODO: Get metrics
            println!("\n# To get metrics via client:");
            println!("let metrics = client.get_metrics().await?;");
        }
    }

    Ok(())
}