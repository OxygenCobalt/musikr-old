use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{FrameHeader, Id3Frame};
use std::fmt::{self, Display, Formatter};

pub struct RawFrame {
    header: FrameHeader,
    data: Vec<u8>,
}

impl RawFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Self {
        let data = data.to_vec();

        RawFrame { header, data }
    }

    pub fn default() -> Self {
        RawFrame {
            header: FrameHeader {
                frame_id:"TIPL".to_string(),
                frame_size: 0,
                tag_should_discard: false,
                file_should_discard: false,
                read_only: false,
                has_group: false,
                compressed: false,
                encrypted: false,
                unsync: false,
                has_data_len: false,                
            },
            data: vec![]
        }
    }

    fn raw(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn from(frame: &dyn Id3Frame) -> Option<&Self> {
        frame.downcast_ref()
    }
}

impl Id3Frame for RawFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        self.id().clone()
    }
}

impl Display for RawFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_vec_hexstream(f, &self.data)
    }
}

pub struct PrivateFrame {
    header: FrameHeader,
    owner: String,
    data: Vec<u8>,
}

impl PrivateFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }

        let (owner, owner_size) = string::get_terminated_string(Encoding::Utf8, data);
        let data = data[owner_size..].to_vec();

        Some(PrivateFrame {
            header,
            owner,
            data,
        })
    }

    pub fn from(frame: &dyn Id3Frame) -> Option<&Self> {
        frame.downcast_ref()
    }

    pub fn owner(&self) -> &String {
        &self.owner
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl Id3Frame for PrivateFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.owner]
    }
}

impl Display for PrivateFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.owner]
    }
}

pub struct FileIdFrame {
    header: FrameHeader,
    owner: String,
    identifier: Vec<u8>,
}

impl FileIdFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }

        let (owner, owner_size) = string::get_terminated_string(Encoding::Utf8, data);
        let identifier = data[owner_size..].to_vec();

        Some(FileIdFrame {
            header,
            owner,
            identifier,
        })
    }

    pub fn from(frame: &dyn Id3Frame) -> Option<&Self> {
        frame.downcast_ref()
    }

    pub fn owner(&self) -> &String {
        &self.owner
    }

    pub fn identifier(&self) -> &Vec<u8> {
        &self.identifier
    }
}

impl Id3Frame for FileIdFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.owner]
    }
}

impl Display for FileIdFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.owner]
    }
}

fn fmt_vec_hexstream(f: &mut Formatter, vec: &[u8]) -> fmt::Result {
    let data = if vec.len() > 64 {
        // Truncate the hex data to 64 bytes
        &vec[0..64]
    } else {
        vec
    };

    for byte in data {
        write![f, "{:02x}", byte]?;
    }

    Ok(())
}