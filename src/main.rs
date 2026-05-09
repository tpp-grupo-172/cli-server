use clap::{Parser, Subcommand};
use std::path::PathBuf;
use config::Config;

mod config;
mod analysis;

#[derive(Parser)]
#[command(name = "cli-test")]
#[command(about = "Analyzes dependencies in a code project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true, help = "Output as JSON instead of human-readable text")]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    Unused {
        path: PathBuf,
    },
    Antipatterns {
        path: PathBuf,
    },
    All {
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let config = match &cli.config {
        Some(config_path) => match Config::load(config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error loading config: {}", e);
                std::process::exit(1);
            }
        },
        None => {
            let default_path = PathBuf::from("config.toml");
            if default_path.exists() {
                match Config::load(&default_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error loading config.toml: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                Config::load_default()
            }
        }
    };

    match cli.command {
        Commands::Unused { path } => analysis::unused::run(&path, cli.json),
        Commands::Antipatterns { path } => analysis::antipatterns::run(&path, &config, cli.json),
        Commands::All { path } => {
            analysis::unused::run(&path, cli.json);
            analysis::antipatterns::run(&path, &config, cli.json);
        }
    }
}
