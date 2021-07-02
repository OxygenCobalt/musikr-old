use crate::core::io::BufStream;
use crate::id3v2::frames::lang::Language;
use crate::id3v2::frames::{encoding, Frame, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct OwnershipFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub price: String,
    pub purchase_date: String,
    pub seller: String,
}

impl OwnershipFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let price = string::read_terminated(Encoding::Latin1, stream);
        let purchase_date = string::read_exact(Encoding::Latin1, stream, 8)?;
        let seller = string::read(encoding, stream);

        Ok(Self {
            header,
            encoding,
            price,
            purchase_date,
            seller,
        })
    }
}

impl Frame for OwnershipFrame {
    fn key(&self) -> String {
        String::from("OWNE")
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        false // Can never be empty.
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.major());
        result.push(encoding::render(self.encoding));

        result.extend(string::render_terminated(Encoding::Latin1, &self.price));

        let purchase_date = string::render(Encoding::Latin1, &self.purchase_date);

        // The purchase date must be an 8-character date. If that fails, then we write the unix
        // epoch instead because that should probably cause less breakage than just 8 spaces or
        // writing a malformed date.
        if self.purchase_date.len() == 8 {
            result.extend(purchase_date)
        } else {
            result.extend(b"19700101");
        }

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
            header: FrameHeader::new(b"OWNE"),
            encoding: Encoding::default(),
            price: String::new(),
            purchase_date: String::new(),
            seller: String::new(),
        }
    }
}

pub struct TermsOfUseFrame {
    header: FrameHeader,
    pub encoding: Encoding,
    pub lang: Language,
    pub text: String,
}

impl TermsOfUseFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let lang = Language::parse(stream)?;
        let text = string::read(encoding, stream);

        Ok(Self {
            header,
            encoding,
            lang,
            text,
        })
    }
}

impl Frame for TermsOfUseFrame {
    fn key(&self) -> String {
        format!["{}:{}", self.text, self.lang]
    }

    fn header(&self) -> &FrameHeader {
        &self.header
    }

    fn header_mut(&mut self, _: Token) -> &mut FrameHeader {
        &mut self.header
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = encoding::check(self.encoding, tag_header.major());
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
            header: FrameHeader::new(b"USER"),
            encoding: Encoding::default(),
            lang: Language::default(),
            text: String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONWE_DATA: &[u8] = b"\x01\
                                $19.99\0\
                                20200101\
                                \xFF\xFE\x53\x00\x65\x00\x6c\x00\x6c\x00\x65\x00\x72\x00";

    const USER_DATA: &[u8] = b"\x02\
                                eng\
                                \x00\x32\x00\x30\x00\x32\x00\x30\x00\x20\x00\x54\x00\x65\x00\x72\x00\
                                \x6d\x00\x73\x00\x20\x00\x6f\x00\x66\x00\x20\x00\x75\x00\x73\x00\x65";

    #[test]
    fn parse_owne() {
        let frame =
            OwnershipFrame::parse(FrameHeader::new(b"OWNE"), &mut BufStream::new(ONWE_DATA))
                .unwrap();

        assert_eq!(frame.encoding, Encoding::Utf16);
        assert_eq!(frame.price, "$19.99");
        assert_eq!(frame.purchase_date, "20200101");
        assert_eq!(frame.seller, "Seller");
    }

    #[test]
    fn parse_user() {
        let frame =
            TermsOfUseFrame::parse(FrameHeader::new(b"USER"), &mut BufStream::new(USER_DATA))
                .unwrap();

        assert_eq!(frame.encoding, Encoding::Utf16Be);
        assert_eq!(frame.lang.as_str(), "eng");
        assert_eq!(frame.text, "2020 Terms of use")
    }

    #[test]
    fn render_owne() {
        let mut frame = OwnershipFrame::new();

        frame.encoding = Encoding::Utf16;
        frame.price.push_str("$19.99");
        frame.purchase_date.push_str("20200101");
        frame.seller.push_str("Seller");

        assert_eq!(frame.render(&TagHeader::with_version(4)), ONWE_DATA);
    }

    #[test]
    fn render_user() {
        let mut frame = TermsOfUseFrame::new();

        frame.encoding = Encoding::Utf16Be;
        frame.lang.set(b"eng").unwrap();
        frame.text.push_str("2020 Terms of use");

        assert_eq!(frame.render(&TagHeader::with_version(4)), USER_DATA);
    }
}
