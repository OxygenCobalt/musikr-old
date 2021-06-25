use crate::core::io::BufStream;
use crate::id3v2::frames::lang::Language;
use crate::id3v2::frames::{encoding, Frame, FrameFlags, FrameHeader, Token};
use crate::id3v2::{ParseResult, TagHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct OwnershipFrame {
    header: FrameHeader,
    encoding: Encoding,
    price: String,
    purchase_date: String,
    seller: String,
}

impl OwnershipFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"OWNE", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        OwnershipFrame {
            header,
            encoding: Encoding::default(),
            price: String::new(),
            purchase_date: String::new(),
            seller: String::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let price = string::read_terminated(Encoding::Latin1, stream);
        let purchase_date = string::read_exact(Encoding::Latin1, stream, 8)?;
        let seller = string::read(encoding, stream);

        Ok(OwnershipFrame {
            header,
            encoding,
            price,
            purchase_date,
            seller,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn price(&self) -> &String {
        &self.price
    }

    pub fn purchase_date(&self) -> &String {
        &self.purchase_date
    }

    pub fn seller(&self) -> &String {
        &self.seller
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn price_mut(&mut self) -> &mut String {
        &mut self.price
    }

    pub fn purchase_date_mut(&mut self) -> &mut String {
        &mut self.purchase_date
    }

    pub fn seller_mut(&mut self) -> &mut String {
        &mut self.seller
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
        // overwriting the date.
        if purchase_date.len() == 8 {
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
        Self::with_flags(FrameFlags::default())
    }
}

pub struct TermsOfUseFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: Language,
    text: String,
}

impl TermsOfUseFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags(b"USER", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        TermsOfUseFrame {
            header,
            encoding: Encoding::default(),
            lang: Language::default(),
            text: String::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, stream: &mut BufStream) -> ParseResult<Self> {
        let encoding = encoding::parse(stream)?;
        let lang = Language::parse(stream)?;
        let text = string::read(encoding, stream);

        Ok(TermsOfUseFrame {
            header,
            encoding,
            lang,
            text,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn lang(&self) -> &Language {
        &self.lang
    }

    pub fn text(&self) -> &String {
        &self.text
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn lang_mut(&mut self) -> &mut Language {
        &mut self.lang
    }

    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
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
        Self::with_flags(FrameFlags::default())
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

        assert_eq!(frame.encoding(), Encoding::Utf16);
        assert_eq!(frame.price(), "$19.99");
        assert_eq!(frame.purchase_date(), "20200101");
        assert_eq!(frame.seller(), "Seller");
    }

    #[test]
    fn parse_user() {
        let frame =
            TermsOfUseFrame::parse(FrameHeader::new(b"USER"), &mut BufStream::new(USER_DATA))
                .unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16Be);
        assert_eq!(frame.lang(), "eng");
        assert_eq!(frame.text(), "2020 Terms of use")
    }

    #[test]
    fn render_owne() {
        let mut frame = OwnershipFrame::new();

        *frame.encoding_mut() = Encoding::Utf16;
        frame.price_mut().push_str("$19.99");
        frame.purchase_date_mut().push_str("20200101");
        frame.seller_mut().push_str("Seller");

        assert_eq!(frame.render(&TagHeader::with_version(4)), ONWE_DATA);
    }

    #[test]
    fn render_user() {
        let mut frame = TermsOfUseFrame::new();

        *frame.encoding_mut() = Encoding::Utf16Be;
        frame.lang_mut().set(b"eng").unwrap();
        frame.text_mut().push_str("2020 Terms of use");

        assert_eq!(frame.render(&TagHeader::with_version(4)), USER_DATA);
    }
}
