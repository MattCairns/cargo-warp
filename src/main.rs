mod build;
mod transfer;
use build::cargo_build;
use clap::value_parser;
use transfer::transfer_files;

fn main() {
    let cmd = clap::Command::new("cargo")
        .bin_name("cargo")
        .subcommand_required(true)
        .subcommand(
            clap::command!("warp")
                .arg(
                    clap::arg!(-c --cross <PATH>)
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(
                    clap::arg!(-p --project <PROJECT> "Project to build")
                        .required(false),
                )
                .arg(
                    clap::arg!(-t --target <TARGET> "Target to build")
                        .required(false),
                )
                .arg(
                    clap::arg!(<DESTINATION> "Destination address to copy binary to.")
                        .required(true),
                ),
        );
    let matches = cmd.get_matches();
    let matches = match matches.subcommand() {
        Some(("warp", matches)) => matches,
        _ => unreachable!("clap should ensure we don't get here"),
    };

    let mut p: Option<String> = None;
    if let Some(project) = matches.get_one::<String>("project") {
        p = Some(project.to_string());
    }

    let destination = matches
        .get_one::<String>("DESTINATION")
        .expect("Destination is required");

    transfer_files(cargo_build(p.as_deref()), destination)
}
