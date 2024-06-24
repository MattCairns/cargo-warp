use cargo_metadata::Message;
use std::process::{Command, Stdio};

pub enum BuildType {
    Cargo,
    Cross,
}

pub fn cargo_build(project: Option<&str>, target: Option<&str>, build_type: BuildType) -> Vec<std::path::PathBuf> {
    let mut args: Vec<&str> = vec!["build", "--message-format=json-render-diagnostics"];

    if let Some(project) = project {
        args.push("--project");
        args.push(project)
    }

    if let Some(target) = target {
        args.push("--target");
        args.push(target)
    }

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
                files.push(
                    artifact
                        .executable
                        .unwrap()
                        .into_os_string()
                        .into_string()
                        .unwrap()
                        .into(),
                );
            }
        }
    }

    let _output = command.wait().expect("Couldn't get cargo's exit status");

    files
}
