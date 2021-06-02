use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};
use std::fmt::{self, Display, Formatter};

pub struct RawFrame {
    header: Id3FrameHeader,
    raw: Vec<u8>,
}

impl RawFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> RawFrame {
        let raw = data.to_vec();

        return RawFrame { header, raw };
    }

    fn raw(&self) -> &Vec<u8> {
        return &self.raw;
    }
}

impl Id3Frame for RawFrame {
    fn id(&self) -> &String {
        return &self.header.frame_id;
    }

    fn size(&self) -> usize {
        return self.header.frame_size;
    }
}

impl Display for RawFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_vec_hexstream(f, &self.raw)
    }
}

pub struct FileIdFrame {
    header: Id3FrameHeader,
    owner: String,
    identifier: Vec<u8>,
}

impl FileIdFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> FileIdFrame {
        let (owner, owner_size) = string::get_terminated_string(Encoding::Utf8, data);
        let identifier = data[owner_size..].to_vec();

        return FileIdFrame {
            header,
            owner,
            identifier,
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
    fn id(&self) -> &String {
        return &self.header.frame_id;
    }

    fn size(&self) -> usize {
        return self.header.frame_size;
    }
}

impl Display for FileIdFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_vec_hexstream(f, &self.identifier)?;
        write![f, " [{}]", self.owner]
    }
}

fn fmt_vec_hexstream(f: &mut Formatter, vec: &Vec<u8>) -> fmt::Result {
    let data = if vec.len() > 64 {
        // Truncate the hex data to 64 bytes
        &vec[0..64]
    } else {
        vec
    };

    for byte in data {
        write![f, "{:02x}", byte]?;
    }
    return Ok(());
}
