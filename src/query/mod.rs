use crate::feed::Feed;
use rss::Item;
use std::convert::TryFrom;

pub mod error;
pub mod options;
pub mod parser;
pub mod rangeset;

use self::error::*;
use self::options::*;

pub type QueryOp<'a> = Box<dyn Fn((&Item, usize, &Feed)) -> bool + 'a + Send + Sync>;
pub const RANGE_DELIMITER: char = ':';

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

#[cfg(test)]
mod tests {
    use super::rangeset::{Range, RangeOrSet, Set};
    use super::*;
    use std::collections::HashSet;

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
