use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    NotEnoughData,
    InvalidData,
    InvalidEncoding,
    Unsupported,
    NotFound,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ParseError {
    // Nothing to implement
}

pub type ParseResult<T> = Result<T, ParseError>;
