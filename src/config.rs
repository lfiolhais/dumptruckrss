use super::error::RssDumpError;
use super::utils::*;
use std::path::{self, Path, PathBuf};

#[derive(Debug)]
pub struct DumpConfig<'input_life> {
    pub(super) output: PathBuf,
    pub(super) n_downloads: usize,
    pub(super) timeout: usize,
    feed: &'input_life str,
    output_is_file: bool,
}

impl<'input_life> DumpConfig<'input_life> {
    pub fn new_output_is_dir(
        output_path: &str,
        n_downloads: usize,
        feed: &'input_life str,
        timeout: usize,
    ) -> Self {
        let output = PathBuf::from(output_path);
        DumpConfig {
            output,
            n_downloads,
            feed,
            timeout,
            output_is_file: false,
        }
    }

    pub fn new_output_is_file(
        output_path: &str,
        n_downloads: usize,
        feed: &'input_life str,
        timeout: usize,
    ) -> Result<Self, Box<RssDumpError>> {
        let output = PathBuf::from(output_path);

        if output.is_dir() {
            Err(Box::new(RssDumpError::OutputIsDirectory(output)))
        } else {
            Ok(DumpConfig {
                output,
                n_downloads,
                feed,
                timeout,
                output_is_file: true,
            })
        }
    }

    pub async fn create_output_dir(&self) -> Result<(), Box<RssDumpError>> {
        info!("Checking {}...", self.get_output_display());

        if !self.output_is_file {
            // Check if output is a directory and exists
            if self.output.is_dir() {
                info!("{} exists and is a directory", self.get_output_display());
                let is_read = self.is_output_dir_read()?;
                if is_read {
                    info!(
                        "{} is readable by the current user",
                        self.get_output_display()
                    );
                } else {
                    return Err(Box::new(RssDumpError::OutputDirIsNotReadable(
                        self.get_output().to_path_buf(),
                    )));
                }
            } else {
                info!(
                    "{} does not exist and/or is not a directory. Creating...",
                    self.get_output_display()
                );
                create_directory(&self.output).await?;
            }
        } else {
            // Check if parent is a directory and exists
            let parent = self.output.parent().unwrap();

            if parent.is_dir() {
                info!("{} exists and is a directory", parent.display());
                let is_read = is_path_readable(parent)?;
                if is_read {
                    info!("{} is readable by the current user", parent.display());
                } else {
                    return Err(Box::new(RssDumpError::OutputDirIsNotReadable(
                        parent.to_path_buf(),
                    )));
                }
            } else {
                info!(
                    "{} does not exist and/or is not a directory. Creating...",
                    parent.display()
                );
                create_directory(parent).await?;
            }
        }

        Ok(())
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
