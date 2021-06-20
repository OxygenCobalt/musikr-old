use crate::err::{ParseError, ParseResult};
use crate::id3v2::TagHeader;
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::string::{self, Encoding};
use std::fmt::{self, Display, Formatter};

pub struct OwnershipFrame {
    header: FrameHeader,
    encoding: Encoding,
    price_paid: String,
    purchase_date: String,
    seller: String,
}

impl OwnershipFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("OWNE", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        OwnershipFrame {
            header,
            encoding: Encoding::default(),
            price_paid: String::new(),
            purchase_date: String::new(),
            seller: String::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
        let encoding = Encoding::parse(data)?;

        if data.len() < encoding.nul_size() + 9 {
            // Must be at least an empty price & seller string and 8 bytes for a date.
            return Err(ParseError::NotEnoughData);
        }

        let price = string::get_terminated(Encoding::Latin1, &data[1..]);
        let purchase_date =
            string::get_string(Encoding::Latin1, &data[price.size + 1..price.size + 9]);
        let seller = string::get_string(encoding, &data[price.size + 9..]);

        Ok(OwnershipFrame {
            header,
            encoding,
            price_paid: price.string,
            purchase_date,
            seller,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn price_paid(&self) -> &String {
        &self.price_paid
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

    pub fn price_paid_mut(&mut self) -> &mut String {
        &mut self.price_paid
    }

    pub fn purchase_date_mut(&mut self) -> &mut String {
        &mut self.purchase_date
    }

    pub fn seller_mut(&mut self) -> &mut String {
        &mut self.seller
    }
}

impl Frame for OwnershipFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        self.id().clone()
    }

    fn is_empty(&self) -> bool {
        false // Can never be empty.
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = self.encoding.map_id3v2(tag_header.major());
        result.push(encoding.render());

        result.extend(string::render_terminated(Encoding::Latin1, &self.price_paid));

        if self.purchase_date.len() == 8 {
            result.extend(string::render_string(Encoding::Latin1, &self.purchase_date))
        } else {
            // Invalid date, just default to the unix epoch.
            result.extend(b"01011970");
        }

        result.extend(string::render_string(encoding, &self.seller));
    
        result
    }
}

impl Display for OwnershipFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.seller.is_empty() {
            write![f, "{} [", self.seller]?;

            if !self.price_paid.is_empty() {
                write![f, "{}, ", self.price_paid]?;
            }

            write![f, "{}]", self.purchase_date]?;
        } else {
            if !self.price_paid.is_empty() {
                write![f, "{}, ", self.price_paid]?;
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
    lang: String,
    text: String,
}

impl TermsOfUseFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("USER", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        TermsOfUseFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            text: String::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> ParseResult<Self> {
        if data.len() < 4 {
            // Must be at least one encoding byte, three bytes for language, and one
            // byte for text
            return Err(ParseError::NotEnoughData);
        }

        let encoding = Encoding::new(data[0])?;
        let lang = string::get_string(Encoding::Latin1, &data[1..4]);
        let text = string::get_string(encoding, &data[4..]);

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

    pub fn lang(&self) -> &String {
        &self.lang
    }

    pub fn text(&self) -> &String {
        &self.text
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn lang_mut(&mut self) -> &mut String {
        &mut self.lang
    }

    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }
}

impl Frame for TermsOfUseFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}", self.text, self.lang]
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, tag_header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = self.encoding.map_id3v2(tag_header.major());
        result.push(encoding.render());

        if self.lang.len() == 3 {
            result.extend(string::render_string(Encoding::Latin1, &self.lang))
        } else {
            result.extend(b"xxx")
        }

        result.extend(string::render_string(encoding, &self.text));

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
                                01012020\
                                \xFF\xFE\x53\x00\x65\x00\x6c\x00\x6c\x00\x65\x00\x72\x00";

    const USER_DATA: &[u8] = b"\x02\
                                eng\
                                \x00\x32\x00\x30\x00\x32\x00\x30\x00\x20\x00\x54\x00\x65\x00\x72\x00\
                                \x6d\x00\x73\x00\x20\x00\x6f\x00\x66\x00\x20\x00\x75\x00\x73\x00\x65";

    #[test]
    fn parse_owne() {
        let frame = OwnershipFrame::parse(FrameHeader::new("OWNE"), ONWE_DATA).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16);
        assert_eq!(frame.price_paid(), "$19.99");
        assert_eq!(frame.purchase_date(), "01012020");
        assert_eq!(frame.seller(), "Seller");
    }

    #[test]
    fn parse_user() {
        let frame = TermsOfUseFrame::parse(FrameHeader::new("USER"), USER_DATA).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16Be);
        assert_eq!(frame.lang(), "eng");
        assert_eq!(frame.text(), "2020 Terms of use")
    }

    #[test]
    fn render_owne() {
        let mut frame = OwnershipFrame::new();

        *frame.encoding_mut() = Encoding::Utf16;
        frame.price_paid_mut().push_str("$19.99");
        frame.purchase_date_mut().push_str("01012020");
        frame.seller_mut().push_str("Seller");

        assert_eq!(frame.render(&TagHeader::with_version(4)), ONWE_DATA); 
    }

    #[test]
    fn render_user() {
        let mut frame = TermsOfUseFrame::new();

        *frame.encoding_mut() = Encoding::Utf16Be;
        frame.lang_mut().push_str("eng");
        frame.text_mut().push_str("2020 Terms of use");

        assert_eq!(frame.render(&TagHeader::with_version(4)), USER_DATA); 
    }
}
