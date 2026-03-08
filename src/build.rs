use anyhow::{anyhow, bail, Context, Result};
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
    release: bool,
    build_type: BuildType,
) -> Result<Vec<PathBuf>> {
    let mut args: Vec<&str> = vec![
        "--color",
        "always",
        "build",
        "--message-format=json-render-diagnostics",
    ];

    if let Some(package) = package {
        args.push("--package");
        args.push(package)
    }

    if let Some(target) = target {
        args.push("--target");
        args.push(target)
    }

    if release {
        args.push("--release");
    }

    let path = match build_type {
        BuildType::Cargo => PathBuf::from("/"),
        BuildType::Cross => locate_project()?,
    };

    let build_tool = match build_type {
        BuildType::Cargo => "cargo",
        BuildType::Cross => "cross",
    };

    let mut command = Command::new(build_tool)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn '{build_tool}' — is it installed?"))?;

    let reader = std::io::BufReader::new(
        command
            .stdout
            .take()
            .context("failed to capture build stdout")?,
    );
    let mut files: Vec<PathBuf> = vec![];
    for message in cargo_metadata::Message::parse_stream(reader) {
        if let Message::CompilerArtifact(artifact) =
            message.context("failed to parse cargo build message")?
        {
            if let Some(executable) = artifact.executable {
                let exe_str = executable
                    .into_os_string()
                    .into_string()
                    .map_err(|_| anyhow!("executable path is not valid UTF-8"))?;
                let mut p: PathBuf = path.clone();
                p.push(
                    PathBuf::from(exe_str)
                        .strip_prefix("/")
                        .context("executable path doesn't start with '/'")?,
                );
                files.push(p);
            }
        }
    }

    let output = command
        .wait()
        .context("failed to get build tool's exit status")?;

    if !output.success() {
        bail!("{build_tool} build failed with status: {output}");
    }

    Ok(files)
}

#[derive(Deserialize)]
struct LocateProjectOutput {
    root: String,
}

fn locate_project() -> Result<PathBuf> {
    let output = Command::new("cargo")
        .arg("locate-project")
        .output()
        .context("failed to execute 'cargo locate-project' — is cargo installed?")?;

    if !output.status.success() {
        bail!("cargo locate-project failed with status: {}", output.status);
    }

    let locate_project_output: LocateProjectOutput = serde_json::from_slice(&output.stdout)
        .context("failed to parse JSON output from 'cargo locate-project'")?;

    let mut path = PathBuf::from(locate_project_output.root);
    path.pop();
    Ok(path)
}
