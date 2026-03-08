use anyhow::{Context, Result};
use indicatif::ProgressBar;
use owo_colors::OwoColorize;
use regex::Regex;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::LazyLock,
};

static PROGRESS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(\d{1,3}(,\d{3})*(\.\d+)?).*%").expect("progress regex is invalid")
});

fn transfer_file(file: &PathBuf, destination: &str) -> Result<()> {
    println!();
    println!(
        "{} {:?} -> {}",
        "Transfer".green().bold(),
        file,
        destination
    );

    let filesize = std::fs::metadata(file)
        .with_context(|| format!("failed to read metadata for '{}'", file.display()))?
        .len();

    let file_str = file.to_str().context("file path is not valid UTF-8")?;

    let bar = ProgressBar::new(filesize);
    let mut command = Command::new("rsync")
        .args(["-vaz", "--progress", file_str, destination])
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to spawn rsync — is it installed?")?;

    let stdout = command
        .stdout
        .take()
        .context("failed to capture rsync stdout")?;
    let lines = BufReader::new(stdout).split(b'\r');
    for line in lines {
        let line = line.context("failed to read rsync output")?;
        let progress = parse_progress_bytes(&String::from_utf8_lossy(&line));
        if let Some(progress) = progress {
            bar.set_position(progress);
        }
    }
    bar.finish();

    Ok(())
}

pub fn transfer_files(files: Vec<PathBuf>, destination: &str) -> Result<()> {
    for file in &files {
        transfer_file(file, destination)?;
    }
    Ok(())
}

fn parse_progress_bytes(input: &str) -> Option<u64> {
    if let Some(cap) = PROGRESS_RE.captures(input) {
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
