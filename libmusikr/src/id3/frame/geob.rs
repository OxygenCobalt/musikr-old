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
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        let encoding = Encoding::new(*data.get(0)?);

        if data.len() < (encoding.nul_size() * 2) + 3 {
            return None;
        }

        let (mime, mime_size) = string::get_terminated_string(encoding, &data[1..]);
        let mut pos = mime_size + 1;

        let (filename, fn_size) = string::get_terminated_string(encoding, &data[pos..]);
        pos += fn_size;

        let (desc, desc_size) = string::get_terminated_string(encoding, &data[pos..]);
        pos += desc_size;

        let data = data[pos..].to_vec();

        Some(GeneralObjectFrame {
            header,
            encoding,
            mime,
            filename,
            desc,
            data,
        })
    }
    
    pub fn from(frame: &dyn Id3Frame) -> Option<&Self> {
        frame.downcast_ref()
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
