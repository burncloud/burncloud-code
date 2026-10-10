use bam::{
    config::{Config, DEFAULT_CONFIG},
    storage,
};
use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(name = "bam", version, about = "BurnCloud Agent Manager F0 foundation")]
struct Cli {
    #[arg(long, default_value = ".bam/config.toml", global = true)]
    config: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Doctor,
    Status,
    Config {
        #[command(subcommand)]
        action: ConfigCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Check,
}

fn resolve_db(config_path: &Path, configured: &str) -> PathBuf {
    let path = PathBuf::from(configured);
    if path.is_absolute() {
        return path;
    }
    // Resolve relative paths against working directory, not the config file.
    let _ = config_path;
    path
}

fn execute(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Init => {
            if cli.config.exists() {
                return Err(format!(
                    "configuration already exists: {}",
                    cli.config.display()
                ));
            }
            let parsed = Config::parse(DEFAULT_CONFIG)?;
            if let Some(parent) = cli.config.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let config_file = &cli.config;
            fs::write(config_file, DEFAULT_CONFIG).map_err(|e| e.to_string())?;
            let database = resolve_db(config_file, &parsed.storage.database_path);
            storage::open(&database)?;
            println!("BAM initialized: {}", config_file.display());
            Ok(())
        }
        Commands::Doctor => {
            let config = Config::from_path(&cli.config)?;
            println!("Configuration: valid");
            println!("Local compilation: forbidden");
            println!("Local testing: forbidden");
            println!("Automatic merge: forbidden");
            println!("GitHub access and Codex availability: NOT VERIFIED (F1/F2)");
            println!(
                "Target: {}/{} project #{}",
                config.github.owner, config.github.repository, config.github.project_number
            );
            Ok(())
        }
        Commands::Status => {
            let config = Config::from_path(&cli.config)?;
            let database = resolve_db(&cli.config, &config.storage.database_path);
            if !database.exists() {
                return Err("state database does not exist; run bam init".into());
            }
            let conn = storage::open(&database)?;
            println!(
                "tasks: {}\nphase: F0 foundation\nscheduler: NOT IMPLEMENTED",
                storage::task_count(&conn)?
            );
            Ok(())
        }
        Commands::Config {
            action: ConfigCommand::Check,
        } => {
            Config::from_path(&cli.config)?;
            println!("configuration valid");
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bam: {error}");
            ExitCode::FAILURE
        }
    }
}
