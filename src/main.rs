mod build;
mod transfer;
use build::{cargo_build, BuildType};
use clap::{Parser, Subcommand};
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
        #[arg(short, long)]
        cross: bool,

        #[arg(short, long)]
        package: Option<String>,

        #[arg(short, long)]
        target: Option<String>,

        #[arg(short, long)]
        release: bool,

        #[arg(value_name = "DESTINATION")]
        destination: String,
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
            let files = cargo_build(
                package.as_deref(),
                target.as_deref(),
                release,
                if cross {
                    BuildType::Cross
                } else {
                    BuildType::Cargo
                },
            )?;
            transfer_files(files, &destination)?;
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
                assert!(!cross);
                assert_eq!(package, None);
                assert_eq!(target, None);
                assert!(!release);
                assert_eq!(destination, "user@host:/tmp/bin");
            }
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
                assert!(cross);
                assert_eq!(package, Some("cargo-warp".to_string()));
                assert_eq!(target, Some("aarch64-unknown-linux-gnu".to_string()));
                assert!(release);
                assert_eq!(destination, "user@host:/tmp/bin");
            }
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
}
