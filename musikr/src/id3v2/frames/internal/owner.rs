use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::ParseError;
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
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
                                01012020\0\
                                \x53\x00\x65\x00\x6c\x00\x6c\x00\x65\x00\x72\x00";

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
}
