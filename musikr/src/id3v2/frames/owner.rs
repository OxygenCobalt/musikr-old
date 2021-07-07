use crate::core::io::BufStream;
use crate::id3v2::frames::lang::Language;
use crate::id3v2::frames::{encoding, Frame, FrameId};
use crate::id3v2::{ParseError, ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};
use std::str::{self, FromStr};

#[derive(Debug, Clone)]
pub struct OwnershipFrame {
    pub encoding: Encoding,
    pub price: String,
    pub purchase_date: Date,
    pub seller: String,
}

impl OwnershipFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let price = string::read_terminated(Encoding::Latin1, stream);
        let purchase_date = Date::parse(&stream.read_array()?).unwrap_or_default();
        let seller = string::read(encoding, stream);

        Ok(Self {
            encoding,
            price,
            purchase_date,
            seller,
        })
    }
}

impl Frame for OwnershipFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"OWNE")
    }

    fn key(&self) -> String {
        String::from("OWNE")
    }

    fn is_empty(&self) -> bool {
        false // Can never be empty.
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(Encoding::Latin1, &self.price));
        result.extend(self.purchase_date);
        result.extend(string::render(encoding, &self.seller));

        result
    }
}

impl Display for OwnershipFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.seller.is_empty() {
            write![f, "{} [", self.seller]?;

            if !self.price.is_empty() {
                write![f, "{}, ", self.price]?;
            }

            write![f, "{}]", self.purchase_date]?;
        } else {
            if !self.price.is_empty() {
                write![f, "{}, ", self.price]?;
            }

            write![f, "{}", self.purchase_date]?;
        }

        Ok(())
    }
}

impl Default for OwnershipFrame {
    fn default() -> Self {
        Self {
            encoding: Encoding::default(),
            price: String::new(),
            purchase_date: Date::default(),
            seller: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TermsOfUseFrame {
    pub encoding: Encoding,
    pub lang: Language,
    pub text: String,
}

impl TermsOfUseFrame {
    pub(crate) fn parse(stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let lang = Language::parse(&stream.read_array()?).unwrap_or_default();
        let text = string::read(encoding, stream);

        Ok(Self {
            encoding,
            lang,
            text,
        })
    }
}

impl Frame for TermsOfUseFrame {
    fn id(&self) -> FrameId {
        FrameId::new(b"USER")
    }

    fn key(&self) -> String {
        format!("USER:{}", self.lang)
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.version());
        result.push(encoding::render(self.encoding));
        result.extend(&self.lang);
        result.extend(string::render(encoding, &self.text));

        result
    }
}

impl Display for TermsOfUseFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

impl Default for TermsOfUseFrame {
    fn default() -> Self {
        Self {
            encoding: Encoding::default(),
            lang: Language::default(),
            text: String::new(),
        }
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy)]
pub struct Date([u8; 8]);

impl Date {
    pub fn new(in_date: &[u8; 8]) -> Self {
        Self::parse(in_date).expect("Dates can only be numeric ASCII characters.")
    }

    pub fn parse(in_date: &[u8; 8]) -> ParseResult<Self> {
        let mut date = [0; 8];

        for (i, &byte) in in_date.iter().enumerate() {
            // Dates must be a numeric 8-character string in YYYYMMDD format.
            if !byte.is_ascii_digit() {
                return Err(ParseError::MalformedData);
            }

            date[i] = byte;
        }

        Ok(Date(date))
    }

    pub fn inner(&self) -> &[u8; 8] {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        // We've asserted that this frame is ASCII, so we can unwrap.
        str::from_utf8(&self.0).unwrap()
    }
}

impl IntoIterator for Date {
    type Item = u8;
    type IntoIter = std::array::IntoIter<u8, 8>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self.0)
    }
}

impl<'a> IntoIterator for &'a Date {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromStr for Date {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut date = [0; 8];

        if s.len() != 8 {
            return Err(ParseError::MalformedData);
        }

        for (i, ch) in s.chars().enumerate() {
            if !ch.is_ascii() {
                return Err(ParseError::MalformedData);
            }

            date[i] = ch as u8;
        }

        Date::parse(&date)
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.as_str()]
    }
}

impl Default for Date {
    fn default() -> Self {
        Date::new(b"19700101")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNE_DATA: &[u8] = b"OWNE\x00\x00\x00\x1E\x00\x00\
                               \x01\
                               $19.99\0\
                               20200101\
                               \xFF\xFE\x53\x00\x65\x00\x6c\x00\x6c\x00\x65\x00\x72\x00";

    const USER_DATA: &[u8] = b"USER\x00\x00\x00\x26\x00\x00\
                               \x02\
                               eng\
                               \x00\x32\x00\x30\x00\x32\x00\x30\x00\x20\x00\x54\x00\x65\x00\x72\x00\
                               \x6d\x00\x73\x00\x20\x00\x6f\x00\x66\x00\x20\x00\x75\x00\x73\x00\x65";

    #[test]
    fn parse_owne() {
        crate::make_frame!(OwnershipFrame, OWNE_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf16);
        assert_eq!(frame.price, "$19.99");
        assert_eq!(frame.purchase_date.inner(), b"20200101");
        assert_eq!(frame.seller, "Seller");
    }

    #[test]
    fn parse_user() {
        crate::make_frame!(TermsOfUseFrame, USER_DATA, frame);

        assert_eq!(frame.encoding, Encoding::Utf16Be);
        assert_eq!(frame.lang.code(), b"eng");
        assert_eq!(frame.text, "2020 Terms of use")
    }

    #[test]
    fn render_owne() {
        let frame = OwnershipFrame {
            encoding: Encoding::Utf16,
            price: String::from("$19.99"),
            purchase_date: Date::new(b"20200101"),
            seller: String::from("Seller"),
        };

        crate::assert_render!(frame, OWNE_DATA);
    }

    #[test]
    fn render_user() {
        let frame = TermsOfUseFrame {
            encoding: Encoding::Utf16Be,
            lang: Language::new(b"eng"),
            text: String::from("2020 Terms of use"),
        };

        crate::assert_render!(frame, USER_DATA);
    }
}
