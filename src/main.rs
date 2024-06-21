use cargo_metadata::Message;
use indicatif::ProgressBar;
use owo_colors::OwoColorize;
use regex::Regex;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
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
                .arg(clap::arg!(-p - -project))
                .arg(clap::arg!(-t - -target))
                .arg(clap::arg!(<DESTINATION> "Destination address to copy binary to.")),
        );
    let matches = cmd.get_matches();
    let matches = match matches.subcommand() {
        Some(("warp", matches)) => matches,
        _ => unreachable!("clap should ensure we don't get here"),
    };

    let destination = matches
        .get_one::<String>("DESTINATION")
        .expect("Destination is required");

    let mut command = Command::new("cargo")
        .args(["build", "--message-format=json-render-diagnostics"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let reader = std::io::BufReader::new(command.stdout.take().unwrap());
    let mut executable = String::new();
    let mut executable_name = String::new();
    for message in cargo_metadata::Message::parse_stream(reader) {
        if let Message::CompilerArtifact(artifact) = message.unwrap() {
            if artifact.executable.is_some() {
                let executable_name = artifact.target.name;
                executable = artifact
                    .executable
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap();
            }
        }
    }

    let _output = command.wait().expect("Couldn't get cargo's exit status");

    let filesize = std::fs::metadata(executable.clone()).unwrap().len();
    let bar = ProgressBar::new(filesize);

    println!();
    println!(
        "{} {} -> {}",
        "Rsync".green().bold(),
        executable_name,
        destination
    );

    let mut command = Command::new("rsync")
        .args(["-vaz", "--progress", &executable, destination])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = command.stdout.take().unwrap();
    let lines = BufReader::new(stdout).split(b'\r');
    for line in lines {
        let progress = parse_progress_bytes(&String::from_utf8_lossy(&line.unwrap()));
        if progress.is_some() {
            bar.inc(progress.unwrap());
        }
    }
    bar.finish();
    println!("{} completed succesfully", "Rsync".green().bold());
}

fn parse_progress_bytes(input: &str) -> Option<u64> {
    let re = Regex::new(r"^\s*(\d{1,3}(,\d{3})*(\.\d+)?).*%").unwrap();
    if let Some(cap) = re.captures(input) {
        if let Some(matched) = cap.get(1) {
            return matched.as_str().trim().replace(',', "").parse::<u64>().ok();
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bytes_with_commas() {
        let input = "21,236,688 100%    2.82GB/s    0:00:00 (xfr#1, to-chk=0/1)";
        assert_eq!(parse_progress_bytes(input), Some(21_236_688));
    }

    #[test]
    fn test_parse_progress_bytes_no_number() {
        let input = "No numbers here!";
        assert_eq!(parse_progress_bytes(input), None);
    }

    #[test]
    fn test_parse_progress_bytes_only_number() {
        let input = "42";
        assert_eq!(parse_progress_bytes(input), None);
    }

    #[test]
    fn test_parse_progress_bytes_with_other_text() {
        let input = "The first number is 3,000 followed by other text.";
        assert_eq!(parse_progress_bytes(input), None);
    }
}
//
// -c / --cross
// -p / --project
// -t / --targetk22k
// location

// cargo warp --cross -p nmea2000 -t aarch64-unknown-linux-gnu dx4-jetson:/tmp/.
