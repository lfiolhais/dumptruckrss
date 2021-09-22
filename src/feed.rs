use super::config::DumpConfig;
use super::error::RssDumpError;
use super::query::QueryOp;

use super::utils::create_file_path;
use futures::stream::{self, StreamExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use reqwest::header::{HeaderValue, CONTENT_LENGTH, RANGE};
use reqwest::StatusCode;
use tokio::fs::File;
use tokio::io as tokio_io;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use std::boxed::Box;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug)]
pub struct Feed<'config> {
    title: String,
    full_download_list: Vec<Arc<rss::Item>>,
    config: &'config DumpConfig<'config>,
}

impl<'config> Feed<'config> {
    pub async fn new(
        orig_channel: rss::Channel,
        config: &'config DumpConfig<'config>,
    ) -> Feed<'config> {
        Self {
            title: orig_channel.title().to_owned(),
            full_download_list: orig_channel
                .items()
                .iter()
                .map(|item| Arc::new(item.clone()))
                .collect(),
            config,
        }
    }

    async fn get_content_length(item: &rss::Enclosure) -> Result<u64, Box<dyn std::error::Error>> {
        let response = reqwest::Client::new().head(item.url()).send().await?;
        let length = response
            .headers()
            .get(CONTENT_LENGTH)
            .ok_or("response doesn't include the content length")?;
        let length =
            u64::from_str(length.to_str()?).map_err(|_| "invalid Content-Length header")?;

        Ok(length)
    }

    pub fn build_list_from_query<'a>(
        &mut self,
        queries: &[QueryOp<'a>],
    ) -> Result<Vec<Weak<rss::Item>>, Box<RssDumpError>> {
        Ok(self
            .full_download_list
            .par_iter()
            .enumerate()
            .filter(|(i, item)| {
                queries
                    .iter()
                    .map(|func| func((item, *i, self)))
                    .fold(true, |res, query_result| res & query_result)
            })
            .map(|(_, item)| Arc::downgrade(item))
            .collect())
    }

    pub async fn download_items(
        &self,
        download_list: &[Weak<rss::Item>],
    ) -> Vec<(Weak<rss::Item>, PathBuf, Box<dyn std::error::Error>)> {
        let failed_downs = Arc::new(Mutex::new(vec![]));

        let m = Arc::new(MultiProgress::new());
        let sty = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent:>3}% {msg}")
            .progress_chars("##-");
        let pb_main = Arc::new(m.add(ProgressBar::new(download_list.len() as u64)));
        pb_main.set_style(sty);
        pb_main.enable_steady_tick(1000);

        let m_sentinel = Arc::clone(&m);
        std::thread::spawn(move || m_sentinel.join_and_clear().unwrap());

        stream::iter(download_list.iter().rev())
            .for_each_concurrent(self.config.n_downloads, |epi| {
                let name = epi
                    .upgrade()
                    .unwrap()
                    .title()
                    .unwrap_or("Boilerplate Episode Title")
                    .to_owned();

                let new_file = create_file_path(
                    &self.config.output,
                    epi.upgrade().unwrap().enclosure().unwrap().mime_type(),
                    &name,
                );

                // Perform download
                let failed_downs = Arc::clone(&failed_downs);
                let local_m = Arc::clone(&m);
                let local_pb_main = Arc::clone(&pb_main);

                async move {
                    match self
                        .download_and_store_item(
                            epi.upgrade().unwrap().enclosure().unwrap(),
                            new_file.clone(),
                            local_m,
                            name,
                        )
                        .await
                    {
                        Ok(_) => {
                            local_pb_main.inc(1);
                            std::thread::sleep(std::time::Duration::from_millis(300));
                        }
                        Err(e) => {
                            failed_downs
                                .lock()
                                .unwrap()
                                .push((epi.clone(), new_file, e));
                        }
                    }
                }
            })
            .await;

        pb_main.finish_with_message("Downloads Complete!");

        if failed_downs.lock().unwrap().len() > 0 {
            error!("{} Failed Downloads", failed_downs.lock().unwrap().len());
            for (failed_item, _, error) in failed_downs.lock().unwrap().iter() {
                error!(
                    "\tURL: {:?}; Error: {:?}",
                    failed_item.upgrade().unwrap().enclosure().unwrap().url(),
                    error
                );
            }
        }

        Arc::try_unwrap(failed_downs).unwrap().into_inner().unwrap()
    }

    async fn download_and_store_item(
        &self,
        item: &rss::Enclosure,
        new_file: PathBuf,
        m: Arc<MultiProgress>,
        name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get file size
        let length = Feed::get_content_length(item).await.unwrap();

        // Create progress bar
        let pb = m.add(ProgressBar::new(length).with_message(name.clone()));
        let sty = ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {percent:>3}% {bytes_per_sec:>14} {msg}")
            .progress_chars("##-");
        pb.set_style(sty);
        pb.enable_steady_tick(1000);

        const CHUNK_SIZE: u32 = 5 * 1024 * 1024;
        let tries = 20;

        // Create file
        let mut output_file = File::create(new_file).await?;

        // Get file
        let client = reqwest::Client::new();
        for (range, chunk) in PartialRangeIter::new(0, length - 1, CHUNK_SIZE)? {
            let mut retry_counter = 1;
            pb.set_message(name.clone());

            loop {
                let response = client
                    .get(item.url())
                    .header(RANGE, range.clone())
                    .send()
                    .await?;

                let status = response.status();
                if !(status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT) {
                    pb.set_message(format!(
                        "Try {} of {}. Retrying in {}ms! Unexpected server response: {} ({})",
                        retry_counter,
                        tries,
                        retry_counter * 300,
                        status,
                        name
                    ));
                    retry_counter += 1;
                    std::thread::sleep(std::time::Duration::from_millis(retry_counter * 300));
                    if retry_counter > tries {
                        return Err(Box::new(futures::io::Error::new(
                            futures::io::ErrorKind::Other,
                            format!("Unexpected server response: {} ({})", status, name),
                        )));
                    }
                    continue;
                }

                pb.inc(chunk);

                // Write file to disk
                tokio_io::copy(
                    &mut response
                        .bytes_stream()
                        .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
                        .into_async_read()
                        .compat(),
                    &mut output_file,
                )
                .await?;

                break;
            }
        }

        pb.finish_and_clear();

        Ok(())
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn total_items(&self) -> usize {
        self.full_download_list.len()
    }

    pub fn get_config_output(&self) -> &Path {
        self.config.get_output()
    }
}

struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: u32,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: u32) -> Result<Self, &'static str> {
        if buffer_size == 0 {
            return Err("invalid buffer_size, give a value greater than zero.");
        }
        Ok(PartialRangeIter {
            start,
            end,
            buffer_size,
        })
    }
}

impl Iterator for PartialRangeIter {
    type Item = (HeaderValue, u64);
    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            Some((
                HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1))
                    .expect("string provided by format!"),
                self.start - 1 - prev_start,
            ))
        }
    }
}
