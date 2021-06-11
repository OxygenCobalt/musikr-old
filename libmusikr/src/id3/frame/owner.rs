use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{FrameHeader, Id3Frame};
use std::fmt::{self, Display, Formatter};

pub struct OwnershipFrame {
    header: FrameHeader,
    encoding: Encoding,
    price_paid: String,
    purchase_date: String,
    seller: String,
}

impl OwnershipFrame {
    pub fn new(header: FrameHeader) -> Self {
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

impl Id3Frame for OwnershipFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        self.id().clone()
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ()> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 9 {
            return Err(()); // Not enough data
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

pub struct TermsOfUseFrame {
    header: FrameHeader,
    encoding: Encoding,
    lang: String,
    text: String,
}

impl TermsOfUseFrame {
    pub fn new(header: FrameHeader) -> Self {
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

impl Id3Frame for TermsOfUseFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        format!["{}:{}", self.text, self.lang]
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ()> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < 4 {
            return Err(()); // Not enough data
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
