use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("GEOB", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
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

impl Frame for GeneralObjectFrame {
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
        format!["{}:{}", self.id(), self.desc]
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < (self.encoding.nul_size() * 2) + 3 {
            return Err(ParseError::NotEnoughData);
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

impl Default for GeneralObjectFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}
