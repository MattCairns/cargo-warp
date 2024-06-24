use cargo_metadata::Message;
use serde::Deserialize;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub enum BuildType {
    Cargo,
    Cross,
}

pub fn cargo_build(
    package: Option<&str>,
    target: Option<&str>,
    build_type: BuildType,
) -> Vec<std::path::PathBuf> {
    let mut args: Vec<&str> = vec!["build", "--message-format=json-render-diagnostics"];

    if let Some(package) = package {
        args.push("--package");
        args.push(package)
    }

    if let Some(target) = target {
        args.push("--target");
        args.push(target)
    }

    let path = match build_type {
        BuildType::Cargo => std::path::PathBuf::from("/"),
        BuildType::Cross => locate_project(),
    };

    let mut command = Command::new(match build_type {
        BuildType::Cargo => "cargo",
        BuildType::Cross => "cross",
    })
    .args(args)
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();

    let reader = std::io::BufReader::new(command.stdout.take().unwrap());
    let mut files: Vec<std::path::PathBuf> = vec![];
    for message in cargo_metadata::Message::parse_stream(reader) {
        if let Message::CompilerArtifact(artifact) = message.unwrap() {
            if artifact.executable.is_some() {
                let mut p: std::path::PathBuf = path.clone();
                p.push(std::path::PathBuf::from(
                    artifact
                        .executable
                        .unwrap()
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                ).strip_prefix("/").unwrap());

                files.push(p.clone());
            }
        }
    }

    let _output = command.wait().expect("Couldn't get cargo's exit status");

    files
}

#[derive(Deserialize)]
struct LocateProjectOutput {
    root: String,
}

fn locate_project() -> PathBuf {
    let output = Command::new("cargo")
        .arg("locate-project")
        .output()
        .expect("Failed to execute cargo locate-project");

    if !output.status.success() {
        panic!("cargo locate-project failed with status: {}", output.status);
    }

    let locate_project_output: LocateProjectOutput = serde_json::from_slice(&output.stdout)
        .expect("Failed to parse JSON output from cargo locate-project");

    let mut path = PathBuf::from(locate_project_output.root);
    path.pop();
    path
}
