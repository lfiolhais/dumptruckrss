use super::error::ParserError;
use super::rangeset::RangeOrSet;

pub trait Parser<T>
where
    T: Clone + Eq + std::hash::Hash,
{
    /// Parse the T option present in the query.
    /// A T query is of the form: "T:[xx:yy]" (range), "T:xx" (scalar),
    /// or "T:{xx,[zz:yy],ww}" (set).
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
