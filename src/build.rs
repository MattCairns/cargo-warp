use cargo_metadata::Message;
use std::process::{Command, Stdio};

pub fn cargo_build(project: Option<&str>) -> Vec<std::path::PathBuf> {
    let mut args: Vec<&str> = vec!["build", "--message-format=json-render-diagnostics"];

    if let Some(project) = project {
        args.push("-p");
        args.push(project)
    }

    let mut command = Command::new("cargo")
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
