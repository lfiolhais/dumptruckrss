#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]

#[macro_use]
extern crate log;

use chrono::prelude::*;
use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgGroup, SubCommand,
};
use rss::{Channel, ChannelBuilder};
use tokio::fs::{self, File};
use tokio::io as tokio_io;

use dumptruckrss::config::DumpConfig;
use dumptruckrss::error::RssDumpError;
use dumptruckrss::feed::Feed;
use dumptruckrss::query::{Query, QueryOp, RANGE_DELIMITER};

use std::io::BufReader;
use std::path::PathBuf;

#[allow(clippy::too_many_lines)]
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
        // TODO: move ndownloads to download subcommand
        .arg(
            Arg::with_name("ndownloads")
                .short("d")
                .long("ndownloads")
                .value_name("NDOWNLOADS")
                .help("Maximum number of concurrent downloads")
                .default_value("1")
                .takes_value(true),
        )
        // TODO: move timeout to download subcommand
        .arg(
            Arg::with_name("timeout")
                .short("t")
                .long("timeout")
                .value_name("TIMEOUT")
                .help("Timeout between failures in ms")
                .default_value("300")
                .takes_value(true),
        )
        // TODO: add support for multiple queries
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
                .default_value("notexists")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("download")
                .about("Download queried items in this feed to the specified folder")
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("OUTPUT")
                        .help("Output location to download contents")
                        .takes_value(true)
                        .required(true),
                )
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("Check query results"),
        )
        .subcommand(
            SubCommand::with_name("create")
                .about("Create a feed from query results")
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("OUTPUT")
                        .help("Output location to save the feed")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("title")
                        .short("t")
                        .long("title")
                        .value_name("TITLE")
                        .help("Name of the feed to be created")
                        .takes_value(true)
                )
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
        unreachable!();
    };

    let timeout: usize = if let Some(timeout) = matches.value_of("timeout") {
        timeout.parse()?
    } else {
        unreachable!();
    };

    let query_ops: Vec<QueryOp> = if let Some(query_str) = matches.value_of("query") {
        let query = Query::new(query_str)?;
        let queries = vec![query];
        queries
            .into_iter()
            .map(dumptruckrss::query::Query::build_query_op)
            .collect()
    } else {
        unreachable!();
    };

    // Download Subcommand
    if let Some(matches) = matches.subcommand_matches("download") {
        let config = DumpConfig::new_output_is_dir(
            matches.value_of("output").unwrap(),
            n_downloads,
            rss_feed,
            timeout,
        );
        let mut feed = Feed::new(channel, &config).await;

        // Create directory if necessary
        config.create_output_dir().await?;

        println!(
            "You are about to download the contents of the feed: {}",
            feed.title()
        );

        info!("{} contains {} items", feed.title(), feed.total_items(),);

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

        let mut download_list = feed.build_list_from_query(&query_ops)?;

        let mut loops = 0_usize;
        let not_done;

        loop {
            let failed_downs = feed.download_items(&download_list).await;

            let has_failed_downs = {
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

                failed_items.is_empty()
            };

            download_list = failed_downs.into_iter().map(|(item, _, _)| item).collect();
            loops += 1;

            if has_failed_downs || loops >= 10 {
                not_done = !has_failed_downs;
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
    else if matches.subcommand_matches("check").is_some() {
        let config = DumpConfig::new_output_is_dir(
            matches.value_of("output").unwrap(),
            n_downloads,
            rss_feed,
            timeout,
        );
        let mut feed = Feed::new(channel, &config).await;

        let download_list = feed.build_list_from_query(&query_ops)?;

        if download_list.is_empty() {
            println!(
                "Didn't find any matches with query: {}",
                matches.value_of("query").unwrap()
            );
        } else {
            println!("The following files match the query:");

            for item in download_list {
                let item_access = item.upgrade().unwrap();
                println!(
                    "\t{}\n\t\tURL: {}\n\t\tDate: {}",
                    item_access.title().unwrap(),
                    item_access.enclosure().unwrap().url(),
                    item_access.pub_date().unwrap()
                );
            }

            println!(
                "\nTo download these files run:\n\tdumptruckrss -u {} -o {} -d {}{} download",
                config.get_feed(),
                config.get_output_display(),
                config.get_n_downloads(),
                if !query_ops.is_empty() && matches.value_of("query").is_some() {
                    format!(" -q '{}'", matches.value_of("query").unwrap())
                } else {
                    "".to_string()
                }
            );
        }
    }
    // create Subcommand
    else if let Some(matches) = matches.subcommand_matches("create") {
        let config = DumpConfig::new_output_is_file(
            matches.value_of("output").unwrap(),
            n_downloads,
            rss_feed,
            timeout,
        )?;
        let mut feed = Feed::new(channel, &config).await;

        // Create directory if necessary
        config.create_output_dir().await?;

        let download_list = feed.build_list_from_query(&query_ops)?;

        let now = Local::now().to_rfc2822();

        let title = if let Some(title) = matches.value_of("title") {
            title.to_string()
        } else {
            format!("{}-{}", feed.title(), matches.value_of("query").unwrap())
        };

        let new_channel = ChannelBuilder::default()
            .title(title)
            .link(feed.link())
            .description(feed.description())
            .language(if let Some(l) = feed.language() {
                l.to_owned()
            } else {
                format!("")
            })
            .copyright(if let Some(c) = feed.copyright() {
                c.to_owned()
            } else {
                format!("")
            })
            .managing_editor(if let Some(me) = feed.managing_editor() {
                me.to_owned()
            } else {
                format!("")
            })
            .pub_date(if let Some(p) = feed.pub_date() {
                p.to_owned()
            } else {
                format!("")
            })
            .last_build_date(now)
            .categories(feed.categories())
            .generator(Some(crate_name!().to_owned()))
            .items(
                download_list
                    .iter()
                    .map(|item| (*item.upgrade().unwrap()).clone())
                    .collect::<Vec<rss::Item>>(),
            )
            .build()?;

        let mut file = File::create(config.get_output()).await?;

        tokio_io::copy(&mut new_channel.to_string().as_ref(), &mut file).await?;
    } else {
        unreachable!();
    }

    Ok(())
}
