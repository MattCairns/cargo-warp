use indicatif::ProgressBar;
use owo_colors::OwoColorize;
use regex::Regex;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::delta::{create_patch_for_saved_binary, save_last_build};

fn transfer_file(file: std::path::PathBuf, destination: &str, delta: bool) {
    println!();
    println!(
        "{} {:?} -> {}",
        "Transfer".green().bold(),
        file,
        destination
    );

    let file = if delta {
        create_patch_for_saved_binary(&file).unwrap()
    } else {
        file
    };
    save_last_build(&file);

    let filesize = std::fs::metadata(file.clone()).unwrap().len();

    let bar = ProgressBar::new(filesize);
    let mut command = Command::new("rsync")
        .args(["-vaz", "--progress", (file.to_str().unwrap()), destination])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = command.stdout.take().unwrap();
    let lines = BufReader::new(stdout).split(b'\r');
    for line in lines {
        let progress = parse_progress_bytes(&String::from_utf8_lossy(&line.unwrap()));
        if let Some(progress) = progress {
            bar.set_position(progress);
        }
    }
    bar.finish();
}

pub fn transfer_files(files: Vec<std::path::PathBuf>, destination: &str, delta: bool) {
    for file in &files {
        transfer_file(file.to_path_buf(), destination, delta);
    }
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
        let input = "21,236,688 32%    2.82GB/s    0:00:00 (xfr#1, to-chk=0/1)";
        assert_eq!(parse_progress_bytes(input), Some(21_236_688));
        let input = "236,688 100%    2.82GB/s    0:00:00 (xfr#1, to-chk=0/1)";
        assert_eq!(parse_progress_bytes(input), Some(236_688));
        let input = "688 100%    2.82GB/s    0:00:00 (xfr#1, to-chk=0/1)";
        assert_eq!(parse_progress_bytes(input), Some(688));
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
