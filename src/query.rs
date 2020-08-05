use chrono::NaiveDate;

use super::feed::Feed;
use super::utils::create_file_path;
use error::*;

use std::cmp::Ord;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::iter::FromIterator;

use rss::Item;

pub mod error {
    use chrono::NaiveDate;
    use std::fmt;
    use std::str::FromStr;

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum QueryError {
        InvalidQueryOption(String),
        Number(ParserError<u64>),
        Date(ParserError<NaiveDate>),
        Str(ParserError<String>),
    }

    impl std::error::Error for QueryError {}

    impl fmt::Display for QueryError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                QueryError::InvalidQueryOption(q) => writeln!(
                    f,
                    "Invalid Query Error: '{}' is not a valid query option",
                    q
                )?,
                QueryError::Number(n) => writeln!(f, "Number Query Error: {}", n)?,
                QueryError::Date(n) => writeln!(f, "Date Query Error: {}", n)?,
                QueryError::Str(n) => writeln!(f, "String Query Error: {}", n)?,
            }

            Ok(())
        }
    }

    impl From<ParserError<u64>> for QueryError {
        fn from(error: ParserError<u64>) -> Self {
            QueryError::Number(error)
        }
    }

    impl From<ParserError<NaiveDate>> for QueryError {
        fn from(error: ParserError<NaiveDate>) -> Self {
            QueryError::Date(error)
        }
    }

    impl From<ParserError<String>> for QueryError {
        fn from(error: ParserError<String>) -> Self {
            QueryError::Str(error)
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum ParserError<T>
    where
        T: Clone + Eq + PartialEq,
    {
        ParseInt(std::num::ParseIntError),
        ParseDate(chrono::ParseError),
        EndLessThanStart { start: T, end: T },
        EndEqualToStart { start: T, end: T },
        MissingRangeDelimiter(String),
        MissingStart(String),
        MissingEnd(String),
        Unfinished(String),
        EmptySetElement(String),
        Recursion(String),
        EmptyInput,
    }

    impl<T: Ord + fmt::Display + Clone + fmt::Debug + FromStr + std::hash::Hash> std::error::Error
        for ParserError<T>
    {
    }

    impl<T: fmt::Display + fmt::Debug + Ord + std::hash::Hash + Clone + FromStr> fmt::Display
        for ParserError<T>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ParserError::ParseDate(q) => writeln!(f, "Parser Error: {}", q)?,
                ParserError::ParseInt(q) => writeln!(f, "Parser Error: {}", q)?,
                ParserError::EndEqualToStart { start, end } => writeln!(
                    f,
                    "Parser Error: end ({}) is the same as start ({})",
                    end, start
                )?,
                ParserError::EndLessThanStart { start, end } => writeln!(
                    f,
                    "Parser Error: end ({}) is the greater than start ({})",
                    end, start
                )?,
                ParserError::MissingRangeDelimiter(q) => {
                    writeln!(f, "Parser Error: missing range delimiter - in '{}'", q)?
                }
                ParserError::MissingStart(q) => {
                    writeln!(f, "Parser Error: missing the starter in '{}'", q)?
                }
                ParserError::MissingEnd(q) => {
                    writeln!(f, "Parser Error: missing the terminator in '{}'", q)?
                }
                ParserError::Unfinished(q) => writeln!(
                    f,
                    "Parser Error: input wasn't terminated or started correctly in '{}'",
                    q
                )?,
                ParserError::EmptySetElement(q) => {
                    writeln!(f, "Parser Error: set has an empty element in '{}'", q)?
                }
                ParserError::Recursion(q) => writeln!(
                    f,
                    "Parser Error: sets within sets are not allowed ('{}')",
                    q
                )?,
                ParserError::EmptyInput => writeln!(f, "Number Parser Error: input is empty",)?,
            }

            Ok(())
        }
    }

    impl From<std::num::ParseIntError> for ParserError<u64> {
        fn from(error: std::num::ParseIntError) -> Self {
            ParserError::ParseInt(error)
        }
    }

    impl From<chrono::ParseError> for ParserError<NaiveDate> {
        fn from(error: chrono::ParseError) -> Self {
            ParserError::ParseDate(error)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum QueryOperationOptions {
    Date(RangeOrSet<NaiveDate>),
    Title(RangeOrSet<String>),
    Description(RangeOrSet<String>),
    Number(RangeOrSet<u64>),
    NotExists,
}

pub type QueryOp<'a> = Box<dyn Fn((&Item, usize, &Feed)) -> bool + 'a>;

impl<'input> QueryOperationOptions {
    fn build_func(self) -> QueryOp<'input> {
        let func: QueryOp = match self {
            QueryOperationOptions::Date(ros) => {
                Box::new(move |(i, _, _): (&Item, usize, &Feed)| -> bool {
                    let item_date = i.pub_date().unwrap();

                    let date: NaiveDate = match chrono::DateTime::parse_from_rfc2822(item_date) {
                        Ok(d) => d.date().naive_local(),
                        Err(_) => {
                            info!("Failed to parse item date. {}", item_date);
                            return false;
                        }
                    };

                    match &ros {
                        RangeOrSet::Range(range) => {
                            if let Some(end) = range.end {
                                date >= range.start && date <= end
                            } else {
                                date == range.start
                            }
                        }
                        RangeOrSet::Set(set) => {
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
                        }
                    }
                })
            }
            QueryOperationOptions::Number(ros) => {
                Box::new(move |(_, n, _): (&Item, usize, &Feed)| -> bool {
                    let n = n as u64;
                    match &ros {
                        RangeOrSet::Range(range) => {
                            if let Some(end) = range.end {
                                n >= range.start && n <= end
                            } else {
                                n == range.start
                            }
                        }
                        RangeOrSet::Set(set) => {
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
                        }
                    }
                })
            }
            QueryOperationOptions::Title(ros) => {
                Box::new(move |(i, _, _): (&Item, usize, &Feed)| -> bool {
                    match &ros {
                        RangeOrSet::Set(set) => {
                            for value in set.contents.iter() {
                                if i.title().unwrap().contains(&value.start) {
                                    return true;
                                }
                            }
                        }
                        _ => unreachable!(),
                    }

                    false
                })
            }
            QueryOperationOptions::Description(ros) => {
                Box::new(move |(i, _, _): (&Item, usize, &Feed)| -> bool {
                    match &ros {
                        RangeOrSet::Set(set) => {
                            for value in set.contents.iter() {
                                if i.description().unwrap().contains(&value.start) {
                                    return true;
                                }
                            }
                        }
                        _ => unreachable!(),
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Range<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    start: T,
    end: Option<T>,
}

#[derive(Debug, Clone)]
pub struct Set<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    contents: HashSet<Range<T>>,
}

impl<T> PartialEq for Set<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    fn eq(&self, other: &Self) -> bool {
        let diff: HashSet<_> = self.contents.difference(&other.contents).collect();
        diff.is_empty()
    }
}

impl<T> Eq for Set<T> where T: Clone + Eq + PartialEq + std::hash::Hash {}

#[derive(Debug, Clone)]
pub enum RangeOrSet<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    Range(Range<T>),
    Set(Set<T>),
}

impl<T> PartialEq for RangeOrSet<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    fn eq(&self, other: &Self) -> bool {
        match self {
            RangeOrSet::Range(r) => match other {
                RangeOrSet::Range(other_r) => r == other_r,
                _ => false,
            },
            RangeOrSet::Set(s) => match other {
                RangeOrSet::Set(other_s) => s == other_s,
                _ => false,
            },
        }
    }
}

impl<T> Eq for RangeOrSet<T> where T: Clone + Eq + PartialEq + std::hash::Hash {}

#[derive(Debug)]
pub struct Query<'input> {
    options: &'input str,
    op: QueryOperationOptions,
}

impl<'input> Query<'input> {
    pub fn new(options: &'input str) -> Result<Self, QueryError> {
        let op = QueryOperationOptions::try_from(options)?;
        Ok(Self { options, op })
    }

    pub fn build_query_op(self) -> QueryOp<'input> {
        self.op.build_func()
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

pub const RANGE_DELIMITER: char = ':';

pub trait Parser<T>
where
    T: Clone + Eq + std::hash::Hash,
{
    /// Parse the numeric option present in the query.
    /// A numeric query is of the form: "number:[xx:yy]" (range), "number:xx" (scalar),
    /// or "number:{xx,[zz:yy],ww}" (set).
    fn parse(input: &str) -> Result<RangeOrSet<T>, ParserError<T>> {
        let input = input.trim();

        if input.is_empty() {
            return Err(ParserError::EmptyInput);
        }

        // Check if the range is properly terminated or started
        if (input.starts_with('{') && !input.ends_with('}'))
            || (!input.starts_with('{') && input.ends_with('}'))
        {
            return Err(ParserError::Unfinished(input.to_owned()));
        }
        let set = input.starts_with('{') && input.ends_with('}');

        if !set {
            Ok(Self::parse_range(input)?)
        } else {
            Ok(Self::parse_set(input)?)
        }
    }
    fn parse_range(input: &str) -> Result<RangeOrSet<T>, ParserError<T>>;
    fn parse_set(input: &str) -> Result<RangeOrSet<T>, ParserError<T>>;
}

impl Parser<String> for RangeOrSet<String> {
    fn parse_range(input: &str) -> Result<RangeOrSet<String>, ParserError<String>> {
        Ok(RangeOrSet::Range(Range {
            start: input.to_owned(),
            end: None,
        }))
    }

    fn parse_set(input: &str) -> Result<RangeOrSet<String>, ParserError<String>> {
        // Set construction
        let str_owned = input.to_owned();
        let str_trimmed = str_owned.trim_end_matches('}').trim_start_matches('{');

        if str_trimmed.contains('{') || str_trimmed.contains('}') {
            return Err(ParserError::Recursion(input.to_owned()));
        }

        let list_of_strs = str_trimmed
            .split(',')
            .map(|s| Range {
                start: s.trim().to_owned(),
                end: None,
            })
            .collect::<Vec<Range<String>>>();

        if list_of_strs.iter().any(|s| s.start.is_empty()) {
            return Err(ParserError::EmptySetElement(input.to_owned()));
        }

        let set = HashSet::from_iter(list_of_strs.into_iter());
        Ok(RangeOrSet::Set(Set { contents: set }))
    }
}

impl Parser<u64> for RangeOrSet<u64> {
    fn parse_range(input: &str) -> Result<RangeOrSet<u64>, ParserError<u64>> {
        // Check if the range is properly terminated or started
        if (input.starts_with('[') && !input.ends_with(']'))
            || (!input.starts_with('[') && input.ends_with(']'))
        {
            return Err(ParserError::Unfinished(input.to_owned()));
        }
        let range = input.starts_with('[') && input.ends_with(']');

        // Range or scalar construction
        let start = if range {
            // Get the first number. Note that a user may spam the range delimiter or [ for general foolery.
            let maybe_number = input.split(RANGE_DELIMITER).take(1).collect::<String>();
            let maybe_number = maybe_number.trim_start_matches('[').trim();

            if maybe_number.contains(']') {
                return Err(ParserError::MissingRangeDelimiter(input.to_owned()));
            } else if maybe_number.is_empty() {
                return Err(ParserError::MissingStart(input.to_owned()));
            } else {
                maybe_number.parse()?
            }
        } else {
            input.trim().parse()?
        };

        let end = if range {
            // Get the first number. Note that a user may spam range delimiter or ] for general foolery.
            let maybe_number = input
                .split(RANGE_DELIMITER)
                .skip(1)
                .take(1)
                .collect::<String>();
            let maybe_number = maybe_number.trim_end_matches(']').trim();

            if maybe_number.is_empty() {
                return Err(ParserError::MissingEnd(input.to_owned()));
            }

            let number = maybe_number.parse::<u64>()?;

            match number.cmp(&start) {
                Ordering::Greater => Some(number),
                Ordering::Less => {
                    return Err(ParserError::EndLessThanStart { start, end: number });
                }
                Ordering::Equal => {
                    return Err(ParserError::EndEqualToStart { start, end: number });
                }
            }
        } else {
            None
        };

        Ok(RangeOrSet::Range(Range { start, end }))
    }

    fn parse_set(input: &str) -> Result<RangeOrSet<u64>, ParserError<u64>> {
        // Set construction
        let str_trimmed = input.trim_end_matches('}').trim_start_matches('{');

        if str_trimmed.contains('{') || str_trimmed.contains('}') {
            return Err(ParserError::Recursion(input.to_owned()));
        }

        let list_of_str_numbers = str_trimmed
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<&str>>();

        if list_of_str_numbers
            .iter()
            .any(|number_str| number_str.is_empty())
        {
            return Err(ParserError::EmptySetElement(input.to_owned()));
        }

        // Call `parse_range` since a set is composed of scalars and ranges.
        // There can't be a set error when calling `parse_range` since we already checked for
        // every other set error.
        let numbers_parsed: Vec<Result<RangeOrSet<u64>, ParserError<u64>>> = list_of_str_numbers
            .iter()
            .map(|e| Self::parse_range(e))
            .collect();

        if let Some(error) = numbers_parsed.iter().find(|res| res.is_err()) {
            return Err(error.as_ref().err().unwrap().clone());
        }

        // Per last comment, we already know it's a scalar or range. Therefore, we can unwrap
        // safely and set all other code paths to unreachable
        let mut numbers: Vec<Range<u64>> = numbers_parsed
            .iter()
            .map(|res| match res.as_ref().unwrap() {
                RangeOrSet::Range(ref contents) => *contents,
                _ => unreachable!(),
            })
            .collect();

        // Optimize ranges and scalars where possible
        let mut eviction_ids: HashSet<usize> = HashSet::new();
        let mut update_ids: Vec<(usize, u64)> = vec![];

        for (ri1, range1) in numbers.iter().enumerate() {
            for (ri2, range2) in numbers.iter().enumerate().skip(ri1 + 1) {
                if range1 != range2 {
                    // Both are ranges
                    if range1.end.is_some() && range2.end.is_some() {
                        // Range2 contains Range1
                        if range1.start >= range2.start
                            && range1.end.unwrap() <= range2.end.unwrap()
                        {
                            // Evict range1
                            eviction_ids.insert(ri1);
                        }
                        // Range2 partially contains Range1
                        else if range1.start < range2.start
                            && range1.end.unwrap() >= range2.start
                            && range1.end.unwrap() <= range2.end.unwrap()
                        {
                            // Evict range1
                            eviction_ids.insert(ri1);
                            // Update range2
                            update_ids.push((ri2, range1.start));
                        }
                        // Range1 contains Range2
                        else if range1.start < range2.start
                            && range1.end.unwrap() > range2.end.unwrap()
                        {
                            // Evict range2
                            eviction_ids.insert(ri2);
                        }
                        // Range1 partially contains Range2
                        else if range1.start >= range2.start
                            && range1.start <= range2.end.unwrap()
                            && range1.end.unwrap() > range2.end.unwrap()
                        {
                            // Evict range2
                            eviction_ids.insert(ri2);
                            // Update range1
                            update_ids.push((ri1, range2.start));
                        }
                        // Ranges don't overlap
                        else if range1.end.unwrap() < range2.start
                            || range2.end.unwrap() < range1.start
                        {
                        } else {
                            unimplemented!(
                                "Condition was not implemented:\nRange1:{:?}\nRange2:{:?}",
                                range1,
                                range2
                            );
                        }
                    }
                    // Range1 is a range and Range2 is not
                    else if range1.end.is_some()
                        && range2.end.is_none()
                        && range2.start >= range1.start
                        && range2.start <= range1.end.unwrap()
                    {
                        // Evict range2
                        eviction_ids.insert(ri2);
                    }
                    // Range1 is not a range and Range2 is
                    else if range1.end.is_none()
                        && range2.end.is_some()
                        && range1.start >= range2.start
                        && range1.start <= range2.end.unwrap()
                    {
                        // Evict range1
                        eviction_ids.insert(ri1);
                    }
                }
            }
        }

        for (update_id, new_start) in update_ids {
            numbers[update_id].start = new_start;
        }

        let mut eviction_ids = eviction_ids.iter().copied().collect::<Vec<usize>>();
        eviction_ids.sort();
        eviction_ids.reverse();

        for evict_index in eviction_ids {
            numbers.remove(evict_index);
        }

        let set = HashSet::from_iter(numbers.iter().map(|n| *n));
        Ok(RangeOrSet::Set(Set { contents: set }))
    }
}

impl Parser<NaiveDate> for RangeOrSet<NaiveDate> {
    fn parse_range(input: &str) -> Result<RangeOrSet<NaiveDate>, ParserError<NaiveDate>> {
        // Check if the range is properly terminated or started
        if (input.starts_with('[') && !input.ends_with(']'))
            || (!input.starts_with('[') && input.ends_with(']'))
        {
            return Err(ParserError::Unfinished(input.to_owned()));
        }
        let range = input.starts_with('[') && input.ends_with(']');

        // Range or scalar construction
        let start = if range {
            // Get the first number. Note that a user may spam the range delimiter or [ for general foolery.
            let maybe_number = input.split(RANGE_DELIMITER).take(1).collect::<String>();
            let maybe_number = maybe_number.trim_start_matches('[').trim();

            if maybe_number.contains(']') {
                return Err(ParserError::MissingRangeDelimiter(input.to_owned()));
            } else if maybe_number.is_empty() {
                return Err(ParserError::MissingStart(input.to_owned()));
            } else {
                maybe_number.parse()?
            }
        } else {
            input.trim().parse()?
        };

        let end = if range {
            // Get the first number. Note that a user may spam range delimiter or ] for general foolery.
            let maybe_number = input
                .split(RANGE_DELIMITER)
                .skip(1)
                .take(1)
                .collect::<String>();
            let maybe_number = maybe_number.trim_end_matches(']').trim();

            if maybe_number.is_empty() {
                return Err(ParserError::MissingEnd(input.to_owned()));
            }

            let number = maybe_number.parse::<NaiveDate>()?;

            match number.cmp(&start) {
                Ordering::Greater => Some(number),
                Ordering::Less => {
                    return Err(ParserError::EndLessThanStart { start, end: number });
                }
                Ordering::Equal => {
                    return Err(ParserError::EndEqualToStart { start, end: number });
                }
            }
        } else {
            None
        };

        Ok(RangeOrSet::Range(Range { start, end }))
    }

    fn parse_set(input: &str) -> Result<RangeOrSet<NaiveDate>, ParserError<NaiveDate>> {
        // Set construction
        let str_trimmed = input.trim_end_matches('}').trim_start_matches('{');

        if str_trimmed.contains('{') || str_trimmed.contains('}') {
            return Err(ParserError::Recursion(input.to_owned()));
        }

        let list_of_str_numbers = str_trimmed
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<&str>>();

        if list_of_str_numbers
            .iter()
            .any(|number_str| number_str.is_empty())
        {
            return Err(ParserError::EmptySetElement(input.to_owned()));
        }

        // Call `parse_range` since a set is composed of scalars and ranges.
        // There can't be a set error when calling `parse_range` since we already checked for
        // every other set error.
        let numbers_parsed: Vec<Result<RangeOrSet<NaiveDate>, ParserError<NaiveDate>>> =
            list_of_str_numbers
                .iter()
                .map(|e| Self::parse_range(e))
                .collect();

        if let Some(error) = numbers_parsed.iter().find(|res| res.is_err()) {
            return Err(error.as_ref().err().unwrap().clone());
        }

        // Per last comment, we already know it's a scalar or range. Therefore, we can unwrap
        // safely and set all other code paths to unreachable
        let mut numbers: Vec<Range<NaiveDate>> = numbers_parsed
            .iter()
            .map(|res| match res.as_ref().unwrap() {
                RangeOrSet::Range(ref contents) => *contents,
                _ => unreachable!(),
            })
            .collect();

        // Optimize ranges and scalars where possible
        let mut eviction_ids: HashSet<usize> = HashSet::new();
        let mut update_ids: Vec<(usize, NaiveDate)> = vec![];

        for (ri1, range1) in numbers.iter().enumerate() {
            for (ri2, range2) in numbers.iter().enumerate().skip(ri1 + 1) {
                if range1 != range2 {
                    // Both are ranges
                    if range1.end.is_some() && range2.end.is_some() {
                        // Range2 contains Range1
                        if range1.start >= range2.start
                            && range1.end.unwrap() <= range2.end.unwrap()
                        {
                            // Evict range1
                            eviction_ids.insert(ri1);
                        }
                        // Range2 partially contains Range1
                        else if range1.start < range2.start
                            && range1.end.unwrap() >= range2.start
                            && range1.end.unwrap() <= range2.end.unwrap()
                        {
                            // Evict range1
                            eviction_ids.insert(ri1);
                            // Update range2
                            update_ids.push((ri2, range1.start));
                        }
                        // Range1 contains Range2
                        else if range1.start < range2.start
                            && range1.end.unwrap() > range2.end.unwrap()
                        {
                            // Evict range2
                            eviction_ids.insert(ri2);
                        }
                        // Range1 partially contains Range2
                        else if range1.start >= range2.start
                            && range1.start <= range2.end.unwrap()
                            && range1.end.unwrap() > range2.end.unwrap()
                        {
                            // Evict range2
                            eviction_ids.insert(ri2);
                            // Update range1
                            update_ids.push((ri1, range2.start));
                        }
                        // Ranges don't overlap
                        else if range1.end.unwrap() < range2.start
                            || range2.end.unwrap() < range1.start
                        {
                        } else {
                            unimplemented!(
                                "Condition was not implemented:\nRange1:{:?}\nRange2:{:?}",
                                range1,
                                range2
                            );
                        }
                    }
                    // Range1 is a range and Range2 is not
                    else if range1.end.is_some()
                        && range2.end.is_none()
                        && range2.start >= range1.start
                        && range2.start <= range1.end.unwrap()
                    {
                        // Evict range2
                        eviction_ids.insert(ri2);
                    }
                    // Range1 is not a range and Range2 is
                    else if range1.end.is_none()
                        && range2.end.is_some()
                        && range1.start >= range2.start
                        && range1.start <= range2.end.unwrap()
                    {
                        // Evict range1
                        eviction_ids.insert(ri1);
                    }
                }
            }
        }

        for (update_id, new_start) in update_ids {
            numbers[update_id].start = new_start;
        }

        let mut eviction_ids = eviction_ids.iter().copied().collect::<Vec<usize>>();
        eviction_ids.sort();
        eviction_ids.reverse();

        for evict_index in eviction_ids {
            numbers.remove(evict_index);
        }

        let set = HashSet::from_iter(numbers.iter().map(|n| *n));
        Ok(RangeOrSet::Set(Set { contents: set }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_query_valid() {
        assert_eq!(
            Query::new(&format!("number:[15{}20]", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: Some(20)
            }))
        );
        assert_eq!(
            Query::new("number:15").unwrap().op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: None
            }))
        );
        assert_eq!(
            Query::new(&format!("number:[ 15{}20]", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: Some(20)
            }))
        );
        assert_eq!(
            Query::new(&format!("number:[15{}20 ]", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: Some(20)
            }))
        );
        assert_eq!(
            Query::new(&format!("number:[ 15{}20 ]", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: Some(20)
            }))
        );
        assert_eq!(
            Query::new(&format!("number:[ 15 {}20 ]", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: Some(20)
            }))
        );
        assert_eq!(
            Query::new(&format!("number:[ 15{} 20 ]", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: Some(20)
            }))
        );
        assert_eq!(
            Query::new(&format!("number:[ 15 {} 20 ]", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: Some(20)
            }))
        );
        assert_eq!(
            Query::new("number: 15").unwrap().op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: None
            }))
        );
        assert_eq!(
            Query::new("number:15 ").unwrap().op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: None
            }))
        );
        assert_eq!(
            Query::new("number: 15 ").unwrap().op,
            QueryOperationOptions::Number(RangeOrSet::Range(Range {
                start: 15,
                end: None
            }))
        );
    }

    #[test]
    fn number_query_error_unit_with_range() {
        assert_eq!(
            Query::new("number:[15]").err().unwrap(),
            QueryError::Number(ParserError::MissingRangeDelimiter("[15]".to_owned()))
        );

        assert_eq!(
            Query::new(&format!("number:[{}15]", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::MissingStart(format!(
                "[{}15]",
                RANGE_DELIMITER
            )))
        );

        assert_eq!(
            Query::new(&format!("number:[15{}]", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::MissingEnd(format!("[15{}]", RANGE_DELIMITER)))
        );
    }

    #[test]
    fn number_query_error_range_missing_brackets() {
        assert_eq!(
            Query::new("number:[15").err().unwrap(),
            QueryError::Number(ParserError::Unfinished("[15".to_owned()))
        );

        assert_eq!(
            Query::new("number:15]").err().unwrap(),
            QueryError::Number(ParserError::Unfinished("15]".to_owned()))
        );

        assert_eq!(
            Query::new(&format!("number:[{}15", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::Unfinished(format!("[{}15", RANGE_DELIMITER)))
        );

        assert_eq!(
            Query::new(&format!("number:{}15]", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::Unfinished(format!("{}15]", RANGE_DELIMITER)))
        );

        assert_eq!(
            Query::new(&format!("number:[15{}", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::Unfinished(format!("[15{}", RANGE_DELIMITER)))
        );

        assert_eq!(
            Query::new(&format!("number:15{}]", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::Unfinished(format!("15{}]", RANGE_DELIMITER)))
        );
    }

    #[test]
    fn number_query_error_end_less_than_start() {
        assert_eq!(
            Query::new(&format!("number:[15{}10]", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::EndLessThanStart { start: 15, end: 10 })
        );
    }

    #[test]
    fn number_query_error_end_equal_start() {
        assert_eq!(
            Query::new(&format!("number:[15{}15]", RANGE_DELIMITER))
                .err()
                .unwrap(),
            QueryError::Number(ParserError::EndEqualToStart { start: 15, end: 15 })
        );
    }

    #[test]
    fn number_query_valid_sets() {
        assert_eq!(
            Query::new(&format!("number:{{[15{}20]}}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: Some(20),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new("number:{15}").unwrap().op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: None,
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{15,[20{}25]}}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: None,
                    });
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{15 ,[20{}25]}}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: None,
                    });
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{[20{}25], 15}}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: None,
                    });
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{ [20{}25], 15}}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: None,
                    });
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{ [20{}25] , 15}}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: None,
                    });
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{ [20{}25] , 15 }}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 15,
                        end: None,
                    });
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
    }

    #[test]
    fn number_query_repeated_numbers_in_sets() {
        assert_eq!(
            Query::new(&format!("number:{{ [20{}25] , 20 }}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{ [20{}25] , 21,  22 }}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!("number:{{ [20{}25] , 20,  25 }}", RANGE_DELIMITER))
                .unwrap()
                .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
    }

    #[test]
    fn number_query_overlapping_ranges_in_sets() {
        assert_eq!(
            Query::new(&format!(
                "number:{{ [20{}25] , [19{}20] }}",
                RANGE_DELIMITER, RANGE_DELIMITER
            ))
            .unwrap()
            .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 19,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!(
                "number:{{ [20{}25] , [21{}22] }}",
                RANGE_DELIMITER, RANGE_DELIMITER
            ))
            .unwrap()
            .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 20,
                        end: Some(25),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!(
                "number:{{ [20{}25] , [23{}26] }}",
                RANGE_DELIMITER, RANGE_DELIMITER
            ))
            .unwrap()
            .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 20,
                        end: Some(26),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!(
                "number:{{ [20{}25] , [25{}30] }}",
                RANGE_DELIMITER, RANGE_DELIMITER
            ))
            .unwrap()
            .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 20,
                        end: Some(30),
                    });
                    set
                }
            }))
        );
        assert_eq!(
            Query::new(&format!(
                "number:{{ [20{}25] , [10{}30] }}",
                RANGE_DELIMITER, RANGE_DELIMITER
            ))
            .unwrap()
            .op,
            QueryOperationOptions::Number(RangeOrSet::Set(Set {
                contents: {
                    let mut set = HashSet::new();
                    set.insert(Range {
                        start: 10,
                        end: Some(30),
                    });
                    set
                }
            }))
        );
    }
}
