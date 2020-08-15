use super::error::{ParserError, QueryError};
use super::parser::Parser;
use super::rangeset::{Range, RangeOrSet};
use super::QueryOp;
use crate::feed::Feed;
use crate::utils::create_file_path;
use chrono::NaiveDate;
use rss::Item;
use std::convert::TryFrom;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum QueryOperationOptions {
    Date(RangeOrSet<NaiveDate>),
    Title(RangeOrSet<String>),
    Description(RangeOrSet<String>),
    Number(RangeOrSet<u64>),
    NotExists,
}

impl<'input> QueryOperationOptions {
    pub fn build_func(self) -> QueryOp<'input> {
        let func: QueryOp = match self {
            QueryOperationOptions::Date(ros) => {
                let func: Box<dyn Fn(NaiveDate) -> bool + Send + Sync> = match ros {
                    RangeOrSet::Range(range) => {
                        if let Some(end) = range.end {
                            Box::new(move |date: NaiveDate| -> bool {
                                date >= range.start && date <= end
                            })
                        } else {
                            Box::new(move |date: NaiveDate| -> bool { date == range.start })
                        }
                    }
                    RangeOrSet::Set(set) => Box::new(move |date: NaiveDate| -> bool {
                        for range in set.contents.iter() {
                            if let Some(end) = range.end {
                                if date >= range.start && date <= end {
                                    return true;
                                }
                            } else if date == range.start {
                                return true;
                            }
                        }

                        false
                    }),
                };

                Box::new(move |(i, _, _): (&Item, usize, &Feed)| -> bool {
                    let item_date = i.pub_date().unwrap();

                    let date: NaiveDate = match chrono::DateTime::parse_from_rfc2822(item_date) {
                        Ok(d) => d.date().naive_local(),
                        Err(_) => {
                            info!("Failed to parse item date. {}", item_date);
                            return false;
                        }
                    };

                    func(date)
                })
            }
            QueryOperationOptions::Number(ros) => {
                let func: Box<dyn Fn(u64) -> bool + Send + Sync> = match ros {
                    RangeOrSet::Range(range) => {
                        if let Some(end) = range.end {
                            Box::new(move |n: u64| -> bool { n >= range.start && n <= end })
                        } else {
                            Box::new(move |n: u64| -> bool { n == range.start })
                        }
                    }
                    RangeOrSet::Set(set) => Box::new(move |n: u64| -> bool {
                        for range in set.contents.iter() {
                            if let Some(end) = range.end {
                                if n >= range.start && n <= end {
                                    return true;
                                }
                            } else if n == range.start {
                                return true;
                            }
                        }

                        false
                    }),
                };

                Box::new(move |(_, n, _): (&Item, usize, &Feed)| -> bool { func(n as u64) })
            }
            QueryOperationOptions::Title(ros) => {
                Box::new(move |(i, _, _): (&Item, usize, &Feed)| -> bool {
                    match &ros {
                        RangeOrSet::Range(range) => {
                            if i.title().unwrap().contains(&range.start) {
                                return true;
                            }
                        }
                        RangeOrSet::Set(set) => {
                            for value in set.contents.iter() {
                                if i.title().unwrap().contains(&value.start) {
                                    return true;
                                }
                            }
                        }
                    }

                    false
                })
            }
            QueryOperationOptions::Description(ros) => {
                Box::new(move |(i, _, _): (&Item, usize, &Feed)| -> bool {
                    match &ros {
                        RangeOrSet::Range(range) => {
                            if i.description().unwrap().contains(&range.start) {
                                return true;
                            }
                        }
                        RangeOrSet::Set(set) => {
                            for value in set.contents.iter() {
                                if i.description().unwrap().contains(&value.start) {
                                    return true;
                                }
                            }
                        }
                    }

                    false
                })
            }
            QueryOperationOptions::NotExists => {
                Box::new(|(i, _, feed): (&Item, usize, &Feed)| -> bool {
                    let new_file = create_file_path(
                        feed.get_config_output(),
                        i.enclosure().unwrap().mime_type(),
                        i.title().unwrap(),
                    );

                    !new_file.exists()
                })
            }
        };

        func
    }
}

impl<'input> TryFrom<&'input str> for QueryOperationOptions {
    type Error = QueryError;

    fn try_from(options: &'input str) -> Result<QueryOperationOptions, Self::Error> {
        match options {
            _ if options.starts_with("number:") => {
                let range_or_set = RangeOrSet::parse(&options[7..])?;
                Ok(QueryOperationOptions::Number(range_or_set))
            }
            _ if options.starts_with("title:") => {
                let range_or_set = RangeOrSet::parse(&options[6..])?;
                Ok(QueryOperationOptions::Title(range_or_set))
            }
            _ if options.starts_with("description:") => {
                let range_or_set = RangeOrSet::parse(&options[12..])?;
                Ok(QueryOperationOptions::Description(range_or_set))
            }
            _ if options.starts_with("date:") => {
                let range_or_set = RangeOrSet::parse(&options[5..])?;
                Ok(QueryOperationOptions::Date(range_or_set))
            }
            "notexists" => Ok(QueryOperationOptions::NotExists),
            _ if options.starts_with("latest") => {
                let maybe_end_str = &options[6..].trim();

                let end = if maybe_end_str.is_empty() {
                    None
                } else if maybe_end_str.contains(':') {
                    let new_maybe_end_str = &maybe_end_str[1..].trim();

                    if !new_maybe_end_str.is_empty() {
                        Some(
                            new_maybe_end_str
                                .parse::<u64>()
                                .map_err(|e| QueryError::Number(ParserError::ParseInt(e)))?,
                        )
                    } else {
                        return Err(QueryError::Number(ParserError::EmptyInput));
                    }
                } else {
                    return Err(QueryError::InvalidQueryOption(options.to_string()));
                };

                Ok(QueryOperationOptions::Number(RangeOrSet::Range(Range {
                    start: 0,
                    end,
                })))
            }
            _ => Err(QueryError::InvalidQueryOption(options.to_string())),
        }
    }
}
