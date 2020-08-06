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
