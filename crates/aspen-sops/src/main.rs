//! aspen-sops CLI entry point.

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod cli;

use cli::Cli;
use cli::Commands;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("aspen_sops=debug,info")
    } else {
        EnvFilter::new("aspen_sops=info,warn")
    };

    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();

    let result = run(cli.command).await;

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run(command: Commands) -> aspen_sops::Result<()> {
    match command {
        Commands::Encrypt {
            file,
            cluster_ticket,
            transit_key,
            transit_mount,
            age_recipients,
            encrypted_regex,
            in_place,
        } => {
            let config = aspen_sops::EncryptConfig {
                input_path: file,
                cluster_ticket,
                transit_key,
                transit_mount,
                age_recipients,
                encrypted_regex,
                in_place,
            };
            let output = aspen_sops::encrypt_file(&config).await?;
            if !config.in_place {
                print!("{output}");
            }
        }

        Commands::Decrypt {
            file,
            cluster_ticket,
            output,
            extract,
            #[cfg(feature = "age-fallback")]
            age_identity,
        } => {
            let config = aspen_sops::DecryptConfig {
                input_path: file,
                cluster_ticket,
                output_path: output,
                extract_path: extract,
                #[cfg(feature = "age-fallback")]
                age_identity,
            };
            let decrypted = aspen_sops::decrypt_file(&config).await?;
            if config.output_path.is_none() {
                print!("{decrypted}");
            }
        }

        Commands::Edit {
            file,
            cluster_ticket,
            editor,
            transit_key,
            transit_mount,
        } => {
            let config = aspen_sops::edit::EditConfig {
                input_path: file,
                cluster_ticket,
                editor,
                transit_key,
                transit_mount,
            };
            aspen_sops::edit::edit_file(&config).await?;
        }

        Commands::Rotate {
            file,
            cluster_ticket,
            in_place,
        } => {
            let config = aspen_sops::rotate::RotateConfig {
                input_path: file,
                cluster_ticket,
                in_place,
            };
            let output = aspen_sops::rotate::rotate_file(&config).await?;
            if !config.in_place {
                print!("{output}");
            }
        }

        Commands::UpdateKeys {
            file,
            cluster_ticket,
            transit_key,
            transit_mount,
            add_age,
            remove_age,
            in_place,
        } => {
            let config = aspen_sops::updatekeys::UpdateKeysConfig {
                input_path: file,
                cluster_ticket,
                transit_key,
                transit_mount,
                add_age,
                remove_age,
                in_place,
            };
            let output = aspen_sops::updatekeys::update_keys(&config).await?;
            if !config.in_place {
                print!("{output}");
            }
        }

        #[cfg(feature = "keyservice")]
        Commands::Keyservice {
            cluster_ticket,
            transit_key,
            transit_mount,
            socket,
        } => {
            let config = aspen_sops::keyservice::KeyserviceConfig {
                cluster_ticket,
                transit_key,
                transit_mount,
                socket_path: socket,
            };
            aspen_sops::keyservice::start_keyservice(&config).await?;
        }
    }

    Ok(())
}
