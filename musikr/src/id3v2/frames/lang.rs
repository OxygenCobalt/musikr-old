use crate::id3v2::{ParseError, ParseResult};
use std::fmt::{self, Display, Formatter};
use std::iter::IntoIterator;
use std::str::{self, FromStr};

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy)]
pub struct Language {
    code: [u8; 3],
}

impl Language {
    pub fn new(in_code: &[u8; 3]) -> Self {
        Self::parse(in_code).expect("invalid lang: can only be ASCII alphabetic characters")
    }

    pub fn parse(in_code: &[u8; 3]) -> ParseResult<Self> {
        let mut code = [0; 3];

        for (i, byte) in in_code.iter().enumerate() {
            // ISO-639-2 language codes are always alphabetic ASCII chars.
            if !byte.is_ascii_alphabetic() {
                return Err(ParseError::MalformedData);
            }

            // Certain taggers might write the language code as uppercase chars.
            // For simplicity, we make them lowercase.
            code[i] = byte.to_ascii_lowercase();
        }

        Ok(Language { code })
    }

    pub fn code(&self) -> &[u8; 3] {
        &self.code
    }

    pub fn as_str(&self) -> &str {
        // We've asserted that this is completely ascii, so we can unwrap
        str::from_utf8(&self.code).unwrap()
    }
}

impl IntoIterator for Language {
    type Item = u8;
    type IntoIter = std::array::IntoIter<u8, 3>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self.code)
    }
}

impl<'a> IntoIterator for &'a Language {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.code.iter()
    }
}

impl FromStr for Language {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lang = [0; 3];

        if s.len() != 3 {
            return Err(ParseError::MalformedData);
        }

        for (i, ch) in s.chars().enumerate() {
            if !ch.is_ascii() {
                return Err(ParseError::MalformedData);
            }

            lang[i] = ch as u8;
        }

        Language::parse(&lang)
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.as_str()]
    }
}

impl Default for Language {
    fn default() -> Self {
        // Spec says that language codes should be "xxx" by default
        Language { code: [b'x'; 3] }
    }
}
