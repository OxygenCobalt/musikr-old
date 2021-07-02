use crate::core::io::BufStream;
use crate::id3v2::ParseResult;
use std::convert::TryInto;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::iter::IntoIterator;
use std::str;

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Language {
    code: [u8; 3],
}

impl Language {
    pub fn new(code: &[u8; 3]) -> Result<Self, InvalidLangError> {
        let mut lang = Language::default();
        lang.set(code)?;
        Ok(lang)
    }

    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        Ok(Self::new(&stream.read_array::<3>()?).unwrap_or_default())
    }

    pub fn code(&self) -> &[u8; 3] {
        &self.code
    }

    pub fn as_str(&self) -> &str {
        // We've asserted that this is completely ascii, so we can unwrap
        str::from_utf8(&self.code).unwrap()
    }

    pub fn set(&mut self, new_code: &[u8; 3]) -> Result<(), InvalidLangError> {
        let mut code = [0; 3];

        for (i, byte) in new_code.iter().enumerate() {
            // ISO-639-2 language codes are always alphabetic ASCII chars.
            if !byte.is_ascii_alphabetic() {
                return Err(InvalidLangError());
            }

            // Certain taggers might write the language code as uppercase chars.
            // For simplicity, we make them lowercase.
            code[i] = byte.to_ascii_lowercase();
        }

        self.code = code;

        Ok(())
    }

    pub fn set_str(&mut self, code: &str) -> Result<(), InvalidLangError> {
        if code.len() != 3 {
            return Err(InvalidLangError());
        }

        return self.set(&code.as_bytes().try_into().unwrap());
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

impl Display for Language {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.as_str()]
    }
}

impl Default for Language {
    fn default() -> Self {
        // By default language codes will be "xxx", which isnt actually defined in
        // ISO-639-2 but is used pretty much everywhere as a stand-in for "unknown".
        Language { code: [b'x'; 3] }
    }
}

#[derive(Debug)]
pub struct InvalidLangError();

impl Display for InvalidLangError {
    fn fmt(&self, _: &mut Formatter) -> fmt::Result {
        Ok(())
    }
}

impl Error for InvalidLangError {
    // Nothing to implement
}
