use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{FrameHeader, Id3Frame};
use std::fmt::{self, Display, Formatter};

pub struct GeneralObjectFrame {
    header: FrameHeader,
    encoding: Encoding,
    mime: String,
    filename: String,
    desc: String,
    data: Vec<u8>,
}

impl GeneralObjectFrame {
    pub fn new(header: FrameHeader) -> Self {
        GeneralObjectFrame {
            header,
            encoding: Encoding::default(),
            mime: String::new(),
            filename: String::new(),
            desc: String::new(),
            data: Vec::new(),
        }
    }

    fn mime(&self) -> &String {
        &self.mime
    }

    fn filename(&self) -> &String {
        &self.filename
    }

    fn desc(&self) -> &String {
        &self.desc
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl Id3Frame for GeneralObjectFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.desc]
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ()> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < (self.encoding.nul_size() * 2) + 3 {
            return Err(()); // Not enough data
        }

        let mime = string::get_terminated_string(self.encoding, &data[1..]);
        self.mime = mime.string;
        let mut pos = mime.size + 1;

        let filename = string::get_terminated_string(self.encoding, &data[pos..]);
        self.filename = filename.string;
        pos += filename.size;

        let desc = string::get_terminated_string(self.encoding, &data[pos..]);
        self.desc = desc.string;
        pos += desc.size;

        self.data = data[pos..].to_vec();

        Ok(())
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

        Ok(())
    }
}
