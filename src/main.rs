mod build;
mod config;
mod transfer;
use build::{cargo_build, BuildType};
use clap::{Parser, Subcommand};
use config::{init_config_file, load_config_file, resolve_warp_options, CliWarpOptions};
use transfer::transfer_files;

#[derive(Debug, Parser)]
#[command(name = "git")]
#[command(about = "A fictional versioning CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Warp {
        #[arg(
            short,
            long,
            default_missing_value = "true",
            num_args = 0..=1,
            require_equals = true
        )]
        cross: Option<bool>,

        #[arg(short, long)]
        package: Option<String>,

        #[arg(short, long)]
        target: Option<String>,

        #[arg(
            short,
            long,
            default_missing_value = "true",
            num_args = 0..=1,
            require_equals = true
        )]
        release: Option<bool>,

        #[arg(value_name = "DESTINATION")]
        destination: String,
    },
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Debug, Subcommand)]
enum ConfigCommands {
    Init {
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Cli::parse();
    execute(args)
}

fn execute(args: Cli) -> anyhow::Result<()> {
    match args.command {
        Commands::Warp {
            cross,
            package,
            target,
            release,
            destination,
        } => {
            let config = load_config_file()?;
            let resolved_options = resolve_warp_options(
                config.as_ref(),
                &destination,
                CliWarpOptions {
                    cross,
                    package,
                    target,
                    release,
                },
            );
            let files = cargo_build(
                resolved_options.package.as_deref(),
                resolved_options.target.as_deref(),
                resolved_options.release,
                if resolved_options.cross {
                    BuildType::Cross
                } else {
                    BuildType::Cargo
                },
            )?;
            transfer_files(files, &destination)?;
        }
        Commands::Config(ConfigCommands::Init { force }) => {
            let path = init_config_file(force)?;
            println!("Config written to {}", path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_warp_destination_only() {
        let cli = Cli::try_parse_from(["cargo-warp", "warp", "user@host:/tmp/bin"])
            .expect("Expected valid warp command with destination");

        match cli.command {
            Commands::Warp {
                cross,
                package,
                target,
                release,
                destination,
            } => {
                assert_eq!(cross, None);
                assert_eq!(package, None);
                assert_eq!(target, None);
                assert_eq!(release, None);
                assert_eq!(destination, "user@host:/tmp/bin");
            }
            _ => panic!("Expected warp command"),
        }
    }

    #[test]
    fn test_parse_warp_with_all_flags() {
        let cli = Cli::try_parse_from([
            "cargo-warp",
            "warp",
            "--cross",
            "--package",
            "cargo-warp",
            "--target",
            "aarch64-unknown-linux-gnu",
            "--release",
            "user@host:/tmp/bin",
        ])
        .expect("Expected valid warp command with all options");

        match cli.command {
            Commands::Warp {
                cross,
                package,
                target,
                release,
                destination,
            } => {
                assert_eq!(cross, Some(true));
                assert_eq!(package, Some("cargo-warp".to_string()));
                assert_eq!(target, Some("aarch64-unknown-linux-gnu".to_string()));
                assert_eq!(release, Some(true));
                assert_eq!(destination, "user@host:/tmp/bin");
            }
            _ => panic!("Expected warp command"),
        }
    }

    #[test]
    fn test_parse_warp_with_explicit_boolean_flags() {
        let cli = Cli::try_parse_from([
            "cargo-warp",
            "warp",
            "--cross=false",
            "--release=false",
            "user@host:/tmp/bin",
        ])
        .expect("Expected valid warp command with explicit boolean flags");

        match cli.command {
            Commands::Warp { cross, release, .. } => {
                assert_eq!(cross, Some(false));
                assert_eq!(release, Some(false));
            }
            _ => panic!("Expected warp command"),
        }
    }

    #[test]
    fn test_parse_warp_missing_destination_fails() {
        let result = Cli::try_parse_from(["cargo-warp", "warp", "--release"]);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_flag_fails() {
        let result = Cli::try_parse_from([
            "cargo-warp",
            "warp",
            "--not-a-real-flag",
            "user@host:/tmp/bin",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_config_init() {
        let cli = Cli::try_parse_from(["cargo-warp", "config", "init"])
            .expect("Expected valid config init command");

        match cli.command {
            Commands::Config(ConfigCommands::Init { force }) => {
                assert!(!force);
            }
            _ => panic!("Expected config init command"),
        }
    }

    #[test]
    fn test_parse_config_init_with_force() {
        let cli = Cli::try_parse_from(["cargo-warp", "config", "init", "--force"])
            .expect("Expected valid config init command with force");

        match cli.command {
            Commands::Config(ConfigCommands::Init { force }) => {
                assert!(force);
            }
            _ => panic!("Expected config init command"),
        }
    }
}
