use super::error::ParserError;
use super::parser::Parser;
use super::RANGE_DELIMITER;
use chrono::NaiveDate;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::iter::FromIterator;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Range<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    pub(super) start: T,
    pub(super) end: Option<T>,
}

impl<T> Range<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    pub fn get_start(&self) -> &T {
        &self.start
    }

    pub fn get_end(&self) -> &Option<T> {
        &self.end
    }
}

#[derive(Debug, Clone)]
pub struct Set<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    pub(super) contents: HashSet<Range<T>>,
}

impl<T> Set<T>
where
    T: Clone + Eq + PartialEq + std::hash::Hash,
{
    pub fn get_contents(&self) -> &HashSet<Range<T>> {
        &self.contents
    }
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
