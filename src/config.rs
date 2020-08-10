use super::error::RssDumpError;
use super::utils::*;
use std::path::{self, Path, PathBuf};

#[derive(Debug)]
pub struct DumpConfig<'input_life> {
    pub(super) output: PathBuf,
    pub(super) n_downloads: usize,
    feed: &'input_life str,
}

impl<'input_life> DumpConfig<'input_life> {
    pub fn new(output_path: &str, n_downloads: usize, feed: &'input_life str) -> Self {
        let output = PathBuf::from(output_path);
        DumpConfig {
            output,
            n_downloads,
            feed,
        }
    }

    pub fn does_output_dir_exist(&self) -> bool {
        does_dir_exist(&self.output)
    }

    pub async fn create_output_dir(&self) -> Result<(), Box<RssDumpError>> {
        Ok(create_directory(&self.output).await?)
    }

    pub fn is_output_dir_read(&self) -> Result<bool, Box<RssDumpError>> {
        info!("Checking read permission...");
        is_path_readable(&self.output)
    }

    pub fn is_output_dir_write(&self) -> Result<bool, Box<RssDumpError>> {
        info!("Checking write permission...");
        is_path_writable(&self.output)
    }

    pub fn get_output_display(&self) -> path::Display {
        self.output.display()
    }

    pub fn get_output(&self) -> &Path {
        self.output.as_path()
    }

    pub fn get_n_downloads(&self) -> usize {
        self.n_downloads
    }

    pub fn get_feed(&self) -> &str {
        self.feed
    }
}
