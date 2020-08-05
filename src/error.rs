use std::fmt;
use std::path::PathBuf;
use tokio::io as tokio_io;

use super::query::error::QueryError;

#[derive(Debug)]
pub enum RssDumpError {
    TokioIo(tokio_io::Error),
    NotEnoughFreeSpace { required: u64, available: u64 },
    Rss(rss::Error),
    ParseInt(std::num::ParseIntError),
    OutputDirIsNotReadable(PathBuf),
    OutputDirIsNotWritable(PathBuf),
    Query(QueryError),
}

impl std::error::Error for RssDumpError {}

impl fmt::Display for RssDumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RssDumpError::TokioIo(e) => writeln!(f, "TokioIo Error: {}", e)?,
            RssDumpError::NotEnoughFreeSpace {
                required,
                available,
            } => {
                writeln!(
                    f,
                    "Output Directory Error: There is not enough space in the target device."
                )?;
                writeln!(f, "\tRequired: {}B ({}GiB)", required, required / (1 << 30))?;
                writeln!(
                    f,
                    "\tAvailable: {}B ({}GiB)",
                    available,
                    available / (1 << 30)
                )?;
            }
            RssDumpError::Rss(e) => writeln!(f, "Rss Error: {}", e)?,
            RssDumpError::ParseInt(e) => writeln!(f, "ParseInt Error: {}", e)?,
            RssDumpError::OutputDirIsNotReadable(o) => writeln!(
                f,
                "Output Directory Error: {} is not readable by the current user",
                o.display()
            )?,
            RssDumpError::OutputDirIsNotWritable(o) => writeln!(
                f,
                "Output Directory Error: {} is not writable by the current user",
                o.display()
            )?,
            RssDumpError::Query(e) => writeln!(f, "Query Error: {}", e)?,
        }

        Ok(())
    }
}

impl From<tokio_io::Error> for RssDumpError {
    fn from(error: tokio_io::Error) -> Self {
        RssDumpError::TokioIo(error)
    }
}
impl From<tokio_io::Error> for Box<RssDumpError> {
    fn from(error: tokio_io::Error) -> Self {
        Box::new(RssDumpError::TokioIo(error))
    }
}

impl From<rss::Error> for RssDumpError {
    fn from(error: rss::Error) -> Self {
        RssDumpError::Rss(error)
    }
}
impl From<rss::Error> for Box<RssDumpError> {
    fn from(error: rss::Error) -> Self {
        Box::new(RssDumpError::Rss(error))
    }
}

impl From<std::num::ParseIntError> for RssDumpError {
    fn from(error: std::num::ParseIntError) -> Self {
        RssDumpError::ParseInt(error)
    }
}
impl From<std::num::ParseIntError> for Box<RssDumpError> {
    fn from(error: std::num::ParseIntError) -> Self {
        Box::new(RssDumpError::ParseInt(error))
    }
}

impl From<QueryError> for RssDumpError {
    fn from(error: QueryError) -> Self {
        RssDumpError::Query(error)
    }
}
impl From<QueryError> for Box<RssDumpError> {
    fn from(error: QueryError) -> Self {
        Box::new(RssDumpError::Query(error))
    }
}
