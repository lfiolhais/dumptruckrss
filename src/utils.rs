use super::error::RssDumpError;
use super::ext;

use tokio::fs;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn does_dir_exist(file: &Path) -> bool {
    if file.exists() {
        file.is_dir()
    } else {
        false
    }
}

pub async fn create_directory(file: &Path) -> Result<(), Box<RssDumpError>> {
    Ok(fs::create_dir_all(file).await?)
}

pub fn is_path_readable(path: &Path) -> Result<bool, Box<RssDumpError>> {
    let meta = path.metadata()?;
    let permissions = meta.permissions();

    let mode = permissions.mode();
    info!("Permissions {:o}", mode);

    // Bit of magic
    Ok(((mode >> 8) & 0x1) == 1)
}

pub fn is_path_writable(path: &Path) -> Result<bool, Box<RssDumpError>> {
    let meta = path.metadata()?;
    let permissions = meta.permissions();

    let mode = permissions.mode();
    info!("Permissions {:o}", mode);

    // Bit of magic
    Ok(((mode >> 7) & 0x1) == 1)
}

pub fn create_file_path(file: &Path, mime_type: &str, title: &str) -> PathBuf {
    let extension = ext::AudioType::get_extension_from_mime(mime_type);
    let mut new_file = PathBuf::from(file);
    new_file.push(
        title
            // Replace ASCII '/' with '-'. We need to do this otherwise the OS will
            // infer from the '/' another directory.
            .replace("/", "-"),
    );
    new_file.set_extension(extension);
    new_file
}
