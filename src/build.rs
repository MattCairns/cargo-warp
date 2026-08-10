use anyhow::{bail, Context, Result};
use cargo_metadata::Message;
use std::{
    io::BufRead,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildType {
    Cargo,
    Cross,
}

fn build_command_args(package: Option<&str>, target: Option<&str>, release: bool) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--color".to_string(),
        "always".to_string(),
        "build".to_string(),
        "--message-format=json-render-diagnostics".to_string(),
    ];

    if let Some(package) = package {
        args.push("--package".to_string());
        args.push(package.to_string());
    }

    if let Some(target) = target {
        args.push("--target".to_string());
        args.push(target.to_string());
    }

    if release {
        args.push("--release".to_string());
    }

    args
}

fn build_command_name(build_type: BuildType) -> &'static str {
    match build_type {
        BuildType::Cargo => "cargo",
        BuildType::Cross => "cross",
    }
}

fn executable_output_path(
    target_path: &Path,
    executable: &Path,
    build_type: BuildType,
) -> Result<PathBuf> {
    match build_type {
        BuildType::Cargo => Ok(executable.to_path_buf()),

        BuildType::Cross => {
            let relative_path = executable
                .strip_prefix("/target")
                .context("cross executable path doesn't start with '/target'")?;

            Ok(target_path.join(relative_path))
        }
    }
}

fn collect_executables_from_stream<R: BufRead>(
    reader: R,
    base_path: &Path,
    build_type: BuildType,
) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = vec![];

    for message in Message::parse_stream(reader) {
        if let Message::CompilerArtifact(artifact) =
            message.context("failed to parse cargo build message")?
        {
            if let Some(executable) = artifact.executable {
                files.push(executable_output_path(
                    base_path,
                    executable.as_std_path(),
                    build_type,
                )?);
            }
        }
    }

    Ok(files)
}

pub fn cargo_build(
    package: Option<&str>,
    target: Option<&str>,
    release: bool,
    build_type: BuildType,
) -> Result<Vec<PathBuf>> {
    let args: Vec<String> = build_command_args(package, target, release);
    let target_path = locate_target_folder()?;
    let build_tool = build_command_name(build_type);

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
    let files: Vec<PathBuf> = collect_executables_from_stream(reader, &target_path, build_type)?;

    let output = command
        .wait()
        .context("failed to get build tool's exit status")?;

    if !output.success() {
        bail!("{build_tool} build failed with status: {output}");
    }

    Ok(files)
}

fn locate_target_folder() -> Result<PathBuf> {
    let metadata = cargo_metadata::MetadataCommand::new().exec()?;
    Ok(metadata.target_directory.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_build_command_args_without_optional_flags() {
        let args = build_command_args(None, None, false);

        assert_eq!(
            args,
            vec![
                "--color",
                "always",
                "build",
                "--message-format=json-render-diagnostics",
            ]
        );
    }

    #[test]
    fn test_build_command_args_with_all_optional_flags() {
        let args = build_command_args(Some("cargo-warp"), Some("x86_64-unknown-linux-gnu"), true);

        assert_eq!(
            args,
            vec![
                "--color",
                "always",
                "build",
                "--message-format=json-render-diagnostics",
                "--package",
                "cargo-warp",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--release",
            ]
        );
    }

    #[test]
    fn test_build_command_args_with_package_only() {
        let args = build_command_args(Some("cargo-warp"), None, false);

        assert_eq!(
            args,
            vec![
                "--color",
                "always",
                "build",
                "--message-format=json-render-diagnostics",
                "--package",
                "cargo-warp",
            ]
        );
    }

    #[test]
    fn test_build_command_args_with_target_and_release_only() {
        let args = build_command_args(None, Some("x86_64-unknown-linux-gnu"), true);

        assert_eq!(
            args,
            vec![
                "--color",
                "always",
                "build",
                "--message-format=json-render-diagnostics",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--release",
            ]
        );
    }

    #[test]
    fn test_build_command_name_by_type() {
        assert_eq!(build_command_name(BuildType::Cargo), "cargo");
        assert_eq!(build_command_name(BuildType::Cross), "cross");
    }

    #[test]
    fn test_executable_output_path_for_cargo_build() {
        let executable = PathBuf::from("/tmp/target/debug/cargo-warp");
        let path = executable_output_path(Path::new("/"), &executable, BuildType::Cargo)
            .expect("Expected valid executable output path for cargo");

        assert_eq!(path, PathBuf::from("/tmp/target/debug/cargo-warp"));
    }

    #[test]
    fn test_executable_output_path_for_cross_build() {
        let executable = PathBuf::from("/target/aarch64-unknown-linux-gnu/release/cargo-warp");

        let path = executable_output_path(
            Path::new("/workspace/project/target"),
            &executable,
            BuildType::Cross,
        )
        .unwrap();

        assert_eq!(
            path,
            PathBuf::from("/workspace/project/target/aarch64-unknown-linux-gnu/release/cargo-warp")
        );
    }

    #[test]
    fn test_collect_executables_from_stream_with_single_artifact() {
        let stream = Cursor::new(
            r#"{"reason":"compiler-artifact","package_id":"cargo-warp 0.1.11 (path+file:///tmp/cargo-warp)","manifest_path":"/tmp/cargo-warp/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"cargo-warp","src_path":"/tmp/cargo-warp/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/tmp/cargo-warp/target/debug/cargo-warp"],"executable":"/tmp/cargo-warp/target/debug/cargo-warp","fresh":false}
"#,
        );

        let files = collect_executables_from_stream(stream, Path::new("/"), BuildType::Cargo)
            .expect("Expected artifact stream parsing to succeed");

        assert_eq!(
            files,
            vec![PathBuf::from("/tmp/cargo-warp/target/debug/cargo-warp")]
        );
    }

    #[test]
    fn test_collect_executables_from_stream_ignores_missing_executable() {
        let stream = Cursor::new(
            r#"{"reason":"compiler-artifact","package_id":"cargo-warp 0.1.11 (path+file:///tmp/cargo-warp)","manifest_path":"/tmp/cargo-warp/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"cargo-warp","src_path":"/tmp/cargo-warp/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/tmp/cargo-warp/target/debug/deps/cargo_warp-12345"],"fresh":false}
"#,
        );

        let files = collect_executables_from_stream(stream, Path::new("/"), BuildType::Cargo)
            .expect("Expected artifact stream parsing to succeed");

        assert_eq!(files, Vec::<PathBuf>::new());
    }
}
