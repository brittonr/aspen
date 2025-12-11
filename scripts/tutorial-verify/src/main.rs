use anyhow::{bail, Context, Result};
use clap::Parser;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebouncedEventKind};
use nu_ansi_term::Color;
use std::{
    path::PathBuf,
    process::Command,
    sync::mpsc::channel,
    time::Duration,
};

#[derive(Parser)]
#[command(name = "tutorial-verify")]
#[command(about = "Verify Aspen S3 tutorial lab solutions")]
struct Cli {
    /// Lab number (1-based)
    lab: u32,

    /// Rust toolchain to use
    #[arg(long, default_value = "nightly")]
    toolchain: String,

    /// Watch for file changes and re-run
    #[arg(long)]
    watch: bool,

    /// Repository root (defaults to current directory)
    #[arg(long, default_value = ".")]
    repo_root: PathBuf,
}

/// Map lab numbers to their verification commands.
/// Each lab focuses on a specific S3 API enhancement.
fn lab_commands(lab: u32) -> Result<Vec<(&'static str, Vec<&'static str>)>> {
    match lab {
        1 => Ok(vec![
            // Lab 1: List Buckets - Test basic S3 service setup
            (".", vec!["cargo", "test", "--test", "s3_integration_test", "test_list_buckets"]),
        ]),
        2 => Ok(vec![
            // Lab 2: Bucket Versioning - Extend bucket metadata
            (".", vec!["cargo", "test", "--test", "s3_integration_test", "test_bucket_versioning"]),
        ]),
        3 => Ok(vec![
            // Lab 3: Object Tagging - Extend object operations
            (".", vec!["cargo", "test", "--test", "s3_integration_test", "test_object_tagging"]),
        ]),
        4 => Ok(vec![
            // Lab 4: Multipart Upload - Add new S3 feature
            (".", vec!["cargo", "test", "--test", "s3_integration_test", "test_multipart_upload_initiate"]),
        ]),
        5 => Ok(vec![
            // Lab 5: S3 Metrics - Wire observability
            (".", vec!["cargo", "test", "--test", "s3_integration_test", "test_s3_metrics"]),
        ]),
        6 => Ok(vec![
            // Lab 6: Rate Limiting - Add middleware
            (".", vec!["cargo", "test", "--test", "s3_integration_test", "test_rate_limiting"]),
        ]),
        _ => bail!("Unknown lab: {lab}. Valid labs are 1-6."),
    }
}

fn get_lab_description(lab: u32) -> &'static str {
    match lab {
        1 => "List Buckets Support",
        2 => "Bucket Versioning Metadata",
        3 => "Object Tagging Support",
        4 => "Multipart Upload Initiation",
        5 => "S3 Error Metrics",
        6 => "S3 Rate Limiting",
        _ => "Unknown Lab",
    }
}

fn run_verification(cli: &Cli) -> Result<bool> {
    let commands = lab_commands(cli.lab)?;
    let toolchain_arg = format!("+{}", cli.toolchain);

    println!("\n{} {}: {}",
        Color::Cyan.bold().paint("Lab"),
        cli.lab,
        get_lab_description(cli.lab)
    );
    println!("{}", Color::Blue.paint("─".repeat(50)));

    for (work_dir, args) in commands {
        let full_path = cli.repo_root.join(work_dir);
        let mut cmd_args: Vec<&str> = vec![args[0], &toolchain_arg];
        cmd_args.extend(&args[1..]);

        println!("\n{} {}",
            Color::Blue.bold().paint("Running:"),
            cmd_args.join(" ")
        );

        let status = Command::new(cmd_args[0])
            .args(&cmd_args[1..])
            .current_dir(&full_path)
            .status()
            .with_context(|| format!("Failed to run command: {}", cmd_args.join(" ")))?;

        if !status.success() {
            println!("\n{} Lab {} verification failed",
                Color::Red.bold().paint("✗"),
                cli.lab
            );
            println!("{}", Color::Yellow.paint("Hint: Check your implementation and try again."));
            return Ok(false);
        }
    }

    println!("\n{} Lab {} verified successfully!",
        Color::Green.bold().paint("✓"),
        cli.lab
    );

    // Show next lab hint
    if cli.lab < 6 {
        println!("\n{} Ready for Lab {}? Run: {}",
            Color::Cyan.paint("→"),
            cli.lab + 1,
            Color::White.dimmed().paint(format!("cargo run --manifest-path scripts/tutorial-verify/Cargo.toml -- {}", cli.lab + 1))
        );
    } else {
        println!("\n{} Congratulations! You've completed all labs!",
            Color::Green.bold().paint("🎉")
        );
    }

    Ok(true)
}

fn watch_loop(cli: &Cli) -> Result<()> {
    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;

    let watch_path = cli.repo_root.clone();
    debouncer.watcher().watch(&watch_path, RecursiveMode::Recursive)?;

    println!("{} Watching for changes in {} (Ctrl+C to exit)...",
        Color::Cyan.bold().paint("👀"),
        watch_path.display()
    );

    // Run initial verification
    let _ = run_verification(cli);

    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Filter out .git and target directory changes
                let relevant_events: Vec<_> = events.into_iter().filter(|e| {
                    let path_str = e.path.to_string_lossy();
                    !path_str.contains("/.git/") &&
                    !path_str.contains("/target/") &&
                    !path_str.contains("/.DS_Store") &&
                    (path_str.ends_with(".rs") || path_str.ends_with(".toml"))
                }).collect();

                if !relevant_events.is_empty() {
                    if relevant_events.iter().any(|e| matches!(e.kind, DebouncedEventKind::Any)) {
                        println!("\n{} File changed, re-verifying...",
                            Color::Yellow.paint("⟳")
                        );
                        let _ = run_verification(cli);
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {e:?}"),
            Err(e) => eprintln!("Channel error: {e}"),
        }
    }
}

fn main() -> Result<()> {
    let mut cli = Cli::parse();

    // Allow environment variable override for toolchain
    if let Ok(toolchain) = std::env::var("TUTORIAL_TOOLCHAIN") {
        cli.toolchain = toolchain;
    }

    // Show welcome message
    if !cli.watch {
        println!("{}", Color::Cyan.bold().paint("═══════════════════════════════════════════════"));
        println!("{}", Color::Cyan.bold().paint("  Aspen S3 Tutorial - Lab Verifier"));
        println!("{}", Color::Cyan.bold().paint("═══════════════════════════════════════════════"));
    }

    if cli.watch {
        watch_loop(&cli)
    } else {
        let success = run_verification(&cli)?;
        std::process::exit(if success { 0 } else { 1 });
    }
}