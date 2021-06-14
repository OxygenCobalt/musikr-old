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
        Self::with_header(FrameHeader::with_flags("OWNE", flags).unwrap())
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

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 9 {
            return Err(ParseError::NotEnoughData);
        }

        let price = string::get_terminated_string(Encoding::Utf8, &data[1..]);
        self.price_paid = price.string;

        self.purchase_date = string::get_string(Encoding::Utf8, &data[price.size..price.size + 9]);
        self.seller = string::get_string(self.encoding, &data[price.size + 9..]);

        Ok(())
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
        Self::with_header(FrameHeader::with_flags("USER", flags).unwrap())
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        TermsOfUseFrame {
            header,
            encoding: Encoding::default(),
            lang: String::new(),
            text: String::new(),
        }
    }

    pub fn text(&self) -> &String {
        &self.text
    }

    pub fn lang(&self) -> &String {
        &self.lang
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

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < 4 {
            return Err(ParseError::NotEnoughData);
        }

        self.lang = string::get_string(Encoding::Utf8, &data[1..4]);
        self.text = string::get_string(self.encoding, &data[4..]);

        Ok(())
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
