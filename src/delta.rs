use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

pub fn save_last_build(path: &Path) {
    let crate_name = env!("CARGO_PKG_NAME");
    let tmp_dir = format!("/tmp/{}", crate_name);

    if let Some(file_name) = path.file_name() {
        let file_path = std::path::Path::new(&tmp_dir).join(file_name);

        if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
            eprintln!("Failed to create temp directory: {}", e);
            return;
        }

        if let Err(e) = std::fs::copy(path, file_path) {
            eprintln!("Failed to copy file: {}", e);
        }
    } else {
        eprintln!("Failed to extract file name from the given path.");
    }
}

pub fn create_patch(old_file: &Path, new_file: &Path) -> Option<PathBuf> {
    println!("old: {:?}", old_file);
    println!("new: {:?}", new_file);

    // Read the old and new files into byte vectors
    let old_data = std::fs::read(old_file).expect("Failed to read old file");
    let new_data = std::fs::read(new_file).expect("Failed to read new file");

    // Define the output file
    let output_file = old_file.with_extension("xdelta");
    let output = std::fs::File::create(&output_file).expect("Failed to create patch file");
    let mut writer = std::io::BufWriter::new(output);

    let mut patch = Vec::new();
    qbsdiff::Bsdiff::new(&old_data, &new_data).compare(&mut patch);
    writer
        .write_all(&patch)
        .expect("Failed to write patch to file");

    Some(output_file)
}

pub fn create_patch_for_saved_binary(new_binary: &Path) -> Option<PathBuf> {
    let crate_name = env!("CARGO_PKG_NAME");
    let tmp_dir = format!("/tmp/{}", crate_name);

    if let Some(file_name) = new_binary.file_name() {
        let old_file = Path::new(&tmp_dir).join(file_name);

        if old_file.exists() {
            create_patch(&old_file, new_binary)
        } else {
            eprintln!("Old file not found in the temp directory.");
            None
        }
    } else {
        eprintln!("Failed to extract file name from the new binary path.");
        None
    }
}
