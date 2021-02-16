#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]

#[macro_use]
extern crate log;

use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgGroup, SubCommand,
};
use dumptruckrss::config::DumpConfig;
use dumptruckrss::error::RssDumpError;
use dumptruckrss::feed::Feed;
use dumptruckrss::query::{Query, QueryOp, RANGE_DELIMITER};
use rss::Channel;
use std::io::BufReader;
use std::path::PathBuf;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<RssDumpError>> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("url")
                .short("u")
                .long("url")
                .value_name("URL")
                .help("RSS feed url")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .value_name("FILE")
                .help("RSS feed File")
                .takes_value(true),
        )
        .group(
            ArgGroup::with_name("input")
                .args(&["url", "file"])
                .required(true),
        )
        .arg(
            Arg::with_name("ndownloads")
                .short("d")
                .long("ndownloads")
                .value_name("NDOWNLOADS")
                .help("Maximum number of concurrent downloads")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .help("Output location to download contents")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("query")
                .short("q")
                .long("query")
                .value_name("QUERY")
                .help(
                    &format!("Query items with the following patterns: \n\
                        [date | title | description | number | notexists | latest]\n\n\
                        Examples:\n\t\
                        Number: Select items in the feed with the following numbers. \n\t\t\
                        'number:[0{}12]' (range), 'number:20' (scalar), 'number:{{[0{}12], 20}}' (set)\n\t\
                        Title: Select items which contain the keyword \
                        \n\t\t'title:my_title' (value) or 'title:{{my_title, other_title}}' (set)\n\t\
                        Description: Select items where their description contain the keyword(s) \n\t\t\
                        'description:my_word' (value) or 'description:{{my_word, other_word}}' \
                        (set)\n\t\
                        Not Exists: Select items which are not present in the specified directory \n\t\t\
                        'notexists'\n\t\
                        Latest: Select the latest item in the feed\n\t\t\
                        'latest' downloads the most recent item or 'latest:N' to download the N most recent items",
                        RANGE_DELIMITER, RANGE_DELIMITER),
                )
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("download")
                .about("Download queried items in this feed to the specified folder")
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("Check query results"),
        )
        .get_matches();

    env_logger::init();

    // Get RSS feed from a url or a file
    let rss_feed = if let Some(url) = matches.value_of("url") {
        url
    } else if let Some(file) = matches.value_of("file") {
        file
    } else {
        unreachable!();
    };

    // Access feed
    let channel = if matches.value_of("url").is_some() {
        let content = reqwest::get(rss_feed).await?.bytes().await?;
        Channel::read_from(&content[..])?
    } else {
        let file = std::fs::File::open(rss_feed)?;
        Channel::read_from(BufReader::new(file))?
    };

    let n_downloads: usize = if let Some(n_downloads) = matches.value_of("ndownloads") {
        info!("Downloading {} items concurrently", n_downloads);
        n_downloads.parse()?
    } else {
        info!("Downloading 1 item at a time");
        1
    };

    let config = DumpConfig::new(matches.value_of("output").unwrap(), n_downloads, rss_feed);

    info!("Checking {}...", config.get_output_display());
    if config.does_output_dir_exist() {
        info!("{} exists and is a directory", config.get_output_display());
        let is_read = config.is_output_dir_read()?;
        if is_read {
            info!(
                "{} is readable by the current user",
                config.get_output_display()
            );
        } else {
            return Err(Box::new(RssDumpError::OutputDirIsNotReadable(
                config.get_output().to_path_buf(),
            )));
        }
    } else {
        info!(
            "{} does not exists and/or is not a directory. Creating...",
            config.get_output_display()
        );
        config.create_output_dir().await?;
    }

    let query_ops: Vec<QueryOp> = if let Some(query_str) = matches.value_of("query") {
        let query = Query::new(query_str)?;
        let queries = vec![query];
        queries
            .into_iter()
            .map(dumptruckrss::query::Query::build_query_op)
            .collect()
    } else {
        println!("No query provided. Selecting latest item in feed");
        let query = Query::new("latest")?;
        vec![query]
            .into_iter()
            .map(dumptruckrss::query::Query::build_query_op)
            .collect()
    };

    let mut feed = Feed::new(channel, &config).await;

    // Download Subcommand
    if matches.subcommand_matches("download").is_some() {
        println!(
            "You are about to download the contents of the feed: {}",
            feed.title()
        );

        info!(
            "{} contains {} items ({}GiB)",
            feed.title(),
            feed.total_items(),
            feed.total_feed_size() / (1 << 30)
        );

        let is_write = config.is_output_dir_write()?;
        if is_write {
            info!(
                "{} is writable by the current user",
                config.get_output_display()
            );
        } else {
            return Err(Box::new(RssDumpError::OutputDirIsNotWritable(
                config.get_output().to_path_buf(),
            )));
        }

        let download_list = feed.build_list_from_query(&query_ops)?;

        // Check available space
        let available_space_in_output = fs2::available_space(config.get_output())?;
        if available_space_in_output < feed.total_feed_size() {
            return Err(Box::new(RssDumpError::NotEnoughFreeSpace {
                required: feed.total_feed_size(),
                available: available_space_in_output,
            }));
        }
        info!(
            "Enough space available to store contents.\nRequired: {} ({}GiB)\nAvailable: {} ({}GiB)",
            feed.total_feed_size(), feed.total_feed_size() / (1<<30), available_space_in_output, available_space_in_output / (1<<30)
        );

        let mut loops = 0_usize;
        let not_done;

        loop {
            let failed_downs = feed.download_items(&download_list).await;

            // Build new download list
            let failed_items: Vec<&PathBuf> =
                failed_downs.iter().map(|(_, path, _)| path).collect();
            if !failed_items.is_empty() {
                println!(
                    "{} Downloads failed. Retrying with failed list",
                    failed_items.len()
                );
            }

            // Delete failed downloads, if they exist
            for item_to_delete in &failed_items {
                info!("Deleting {:?}", item_to_delete);
                fs::remove_file(item_to_delete).await?;
            }

            feed.build_list_from_query(&query_ops)?;
            loops += 1;

            if failed_items.is_empty() || loops >= 10 {
                not_done = !failed_items.is_empty();
                break;
            }
        }

        if not_done {
            println!("Download failed");
        } else {
            println!("Full Download Successfully Completed");
        }
    }

    // Check Subcommand
    if matches.subcommand_matches("check").is_some() {
        let download_list = feed.build_list_from_query(&query_ops)?;

        if download_list.is_empty() {
            println!(
                "Didn't find any matches with query: {}",
                matches.value_of("query").unwrap()
            );
        } else {
            println!(
                "In directory {}. The following files match the query:",
                config.get_output_display()
            );

            for (item, size) in download_list {
                let item_access = item.upgrade().unwrap();
                println!(
                    "\t{}\n\t\tSize: {}MiB\n\t\tURL: {}\n\t\tDate: {}",
                    item_access.title().unwrap(),
                    size / (1 << 20),
                    item_access.enclosure().unwrap().url(),
                    item_access.pub_date().unwrap()
                );
            }

            println!(
                "\nTo download these files run:\n\trss-dumper -u {} -o {} -d {}{} download",
                config.get_feed(),
                config.get_output_display(),
                config.get_n_downloads(),
                if !query_ops.is_empty() && matches.value_of("query").is_some() {
                    format!(" -q '{}'", matches.value_of("query").unwrap())
                } else {
                    "".to_string()
                }
            )
        }
    }

    Ok(())
}
