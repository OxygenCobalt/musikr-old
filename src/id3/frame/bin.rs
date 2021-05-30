use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct FileIdFrame {
    header: Id3FrameHeader,
    owner: String,
    identifier: Vec<u8>
}

impl FileIdFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> FileIdFrame {
        let owner = string::get_nul_string(&Encoding::Utf8, data).unwrap_or_default();

        let id_raw = &data[owner.len() + 1..];
        let mut identifier: Vec<u8> = vec![0; id_raw.len()];
        identifier.clone_from_slice(id_raw);

        return FileIdFrame {
            header,
            owner,
            identifier
        };
    }

    pub fn owner(&self) -> &String {
        return &self.owner;
    }

    pub fn identifier(&self) -> &Vec<u8> {
        return &self.identifier;
    }
}

impl Id3Frame for FileIdFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for FileIdFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_vec_hexstream(f, &self.identifier)?;
        write![f, " [{}]", self.owner]
    }    
}

pub struct RawFrame {
    header: Id3FrameHeader,
    raw: Vec<u8>
}

impl RawFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> RawFrame {
        let raw = data.to_vec();

        return RawFrame {
            header,
            raw
        }
    }

    fn raw(&self) -> &Vec<u8> {
        return &self.raw
    }
}

impl Id3Frame for RawFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for RawFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_vec_hexstream(f, &self.raw)
    }    
}

fn fmt_vec_hexstream(f: &mut Formatter, vec: &Vec<u8>) -> fmt::Result {
    for byte in vec {
        write![f, "{:02x}", byte]?;
    }

    return Ok(());
}