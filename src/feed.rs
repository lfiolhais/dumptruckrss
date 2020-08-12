use super::config::DumpConfig;
use super::error::RssDumpError;
use super::query::QueryOp;

use super::utils::create_file_path;
use futures::stream::{self, StreamExt, TryStreamExt};
use pbr::ProgressBar;
use rayon::prelude::*;
use std::boxed::Box;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, Weak};
use tokio::fs::File;
use tokio::io as tokio_io;
use tokio_util::compat::FuturesAsyncReadCompatExt;

#[derive(Debug)]
pub struct Feed<'config> {
    title: String,
    full_download_list: Vec<(Arc<rss::Item>, u64)>,
    total_size: u64,
    config: &'config DumpConfig<'config>,
}

impl<'config> Feed<'config> {
    pub async fn new(
        orig_channel: rss::Channel,
        config: &'config DumpConfig<'config>,
    ) -> Feed<'config> {
        let mut sizes_vec: Vec<(usize, u64)> = vec![];

        while orig_channel.items().len() - sizes_vec.len() > 0 {
            let buffer: Vec<(usize, u64)> = stream::iter(orig_channel.items().iter().enumerate())
                .map(|(i, item): (usize, &rss::Item)| async move {
                    match Self::get_content_length(item.enclosure().unwrap()).await {
                        Ok(l) => (i, l),
                        Err(_) => (i, 0),
                    }
                })
                .buffer_unordered(config.get_n_downloads())
                .collect()
                .await;

            let mut buffer = buffer
                .iter()
                .filter(|(_, size)| *size != 0)
                .map(|(i, size)| (*i, *size))
                .collect();
            sizes_vec.append(&mut buffer);
        }

        sizes_vec.sort_by_key(|(i, _)| *i);

        let total_size = sizes_vec.iter().map(|(_, size)| size).sum::<u64>();

        Self {
            title: orig_channel.title().to_owned(),
            full_download_list: orig_channel
                .items()
                .iter()
                .zip(sizes_vec)
                .map(|(item, (_, size))| (Arc::new(item.clone()), size))
                .collect(),
            total_size,
            config,
        }
    }

    async fn get_content_length(item: &rss::Enclosure) -> Result<u64, Box<dyn std::error::Error>> {
        let response = reqwest::Client::new().head(item.url()).send().await?;
        let length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .ok_or("response doesn't include the content length")?;
        let length =
            u64::from_str(length.to_str()?).map_err(|_| "invalid Content-Length header")?;

        Ok(length)
    }

    pub fn build_list_from_query<'a>(
        &mut self,
        queries: &[QueryOp<'a>],
    ) -> Result<Vec<(Weak<rss::Item>, u64)>, Box<RssDumpError>> {
        Ok(self
            .full_download_list
            .par_iter()
            .enumerate()
            .filter(|(i, (item, _))| {
                queries
                    .iter()
                    .map(|func| func((item, *i, self)))
                    .fold(true, |res, query_result| res & query_result)
            })
            .map(|(_, (item, size))| (Arc::downgrade(&item), *size))
            .collect())
    }

    pub async fn download_items(
        &self,
        download_list: &[(Weak<rss::Item>, u64)],
    ) -> Vec<(Weak<rss::Item>, PathBuf, Box<dyn std::error::Error>)> {
        let mut progress_bar = ProgressBar::new(download_list.len() as u64);
        progress_bar.tick_format("\\|/-");
        progress_bar.format("|#--|");

        let failed_downs = Arc::new(Mutex::new(vec![]));

        stream::iter(download_list.iter().rev())
            .for_each_concurrent(self.config.n_downloads, |(epi, _)| {
                progress_bar.inc();

                let new_file = create_file_path(
                    &self.config.output,
                    epi.upgrade().unwrap().enclosure().unwrap().mime_type(),
                    epi.upgrade()
                        .unwrap()
                        .title()
                        .unwrap_or_else(|| "Boilerplate Episode Title"),
                );

                // Perform download
                let failed_downs = Arc::clone(&failed_downs);
                async move {
                    match self
                        .download_and_store_item(
                            epi.upgrade().unwrap().enclosure().unwrap(),
                            new_file.clone(),
                        )
                        .await
                    {
                        Ok(_) => {}
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

        progress_bar.finish_print(&format!("{}: Download complete", self.title));

        Arc::try_unwrap(failed_downs).unwrap().into_inner().unwrap()
    }

    async fn download_and_store_item(
        &self,
        item: &rss::Enclosure,
        new_file: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get file
        let mut resp = reqwest::get(item.url())
            .await?
            .bytes_stream()
            .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
            .into_async_read()
            .compat();

        // Write file to disk
        let mut out = File::create(new_file).await?;
        tokio_io::copy(&mut resp, &mut out).await?;

        Ok(())
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn total_items(&self) -> usize {
        self.full_download_list.len()
    }

    pub fn total_feed_size(&self) -> u64 {
        self.total_size
    }

    pub fn get_config_output(&self) -> &Path {
        self.config.get_output()
    }
}
