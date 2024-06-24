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
        project: Option<String>,

        #[arg(short, long)]
        target: Option<String>,

        #[arg(value_name = "DESTINATION")]
        destination: String,
    },
}

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Warp {
            cross,
            project,
            target,
            destination,
        } => transfer_files(
            cargo_build(
                project.as_deref(),
                target.as_deref(),
                if cross {
                    BuildType::Cross
                } else {
                    BuildType::Cargo
                },
            ),
            &destination,
        ),
    }
}
