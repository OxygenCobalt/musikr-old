use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct GeneralObjectFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    mime: String,
    filename: String,
    desc: String,
    data: Vec<u8>,
}

impl GeneralObjectFrame {
    pub fn from(header: Id3FrameHeader, data: &[u8]) -> GeneralObjectFrame {
        let encoding = Encoding::from(data[0]);

        let mime = string::get_nul_string(&Encoding::Utf8, &data[1..]).unwrap_or_default();
        let mut pos = mime.len() + 2;

        let filename = string::get_nul_string(&encoding, &data[pos..]).unwrap_or_default();
        pos += filename.len() + 1;

        let desc = string::get_nul_string(&encoding, &data[pos..]).unwrap_or_default();
        pos += desc.len() + 1;

        let data = data[pos..].to_vec();

        return GeneralObjectFrame {
            header,
            encoding,
            mime,
            filename,
            desc,
            data,
        };
    }

    fn mime(&self) -> &String {
        return &self.mime;
    }

    fn filename(&self) -> &String {
        return &self.filename;
    }

    fn desc(&self) -> &String {
        return &self.desc;
    }

    fn data(&self) -> &Vec<u8> {
        return &self.data;
    }
}

impl Id3Frame for GeneralObjectFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for GeneralObjectFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.mime.is_empty() {
            write![f, "{} ", self.mime]?;
        }

        if !self.filename.is_empty() {
            write![f, "\"{}\"", self.filename]?;
        }

        if !self.desc.is_empty() {
            write![f, " [{}]", self.desc]?;
        }

        return Ok(());
    }
}
