pub mod frames;
pub mod header;
mod search;
mod syncdata;

pub(crate) use header::TagHeader;
pub use header::{ExtendedHeader, TagFlags};
pub(crate) use search::*;

use crate::file::File;
use frames::FrameMap;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Error, ErrorKind};

// TODO: ID3v2.2 Conversions

pub struct Tag {
    header: TagHeader,
    extended_header: Option<ExtendedHeader>,
    frames: FrameMap,
}

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    NotEnoughData,
    InvalidData,
    InvalidEncoding,
    Unsupported,
    NotFound,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for ParseError {}

impl Tag {
    pub fn new(file: &mut File, offset: u64) -> io::Result<Tag> {
        file.seek(offset).ok();

        // Read and parse the possible ID3 header
        let mut header_raw = [0; 10];
        file.read_into(&mut header_raw)?;

        let mut header =
            TagHeader::parse(&header_raw).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

        let tag_size = header.size();
        let major = header.major();
        let flags = header.flags_mut();

        // Ensure that this file is large enough to even contain this tag.
        if tag_size as u64 > file.metadata().len() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                ParseError::NotEnoughData,
            ));
        }

        let mut data = file.read_bytes(tag_size)?;

        // Decode unsynced tag data if it exists
        if flags.unsync {
            data = syncdata::decode(&data);
        }

        let mut frames = FrameMap::new();
        let mut frame_pos = 0;
        let mut frame_size = tag_size;

        let extended_header = if flags.extended {
            match ExtendedHeader::parse(major, &data[4..]) {
                Ok(header) => Some(header),
                Err(_) => {
                    // Flag was incorrectly set. Correct the flag and move on.
                    flags.extended = false;
                    None
                }
            }
        } else {
            None
        };

        if flags.footer {
            frame_size -= 10;
        }

        if let Some(ext_header) = &extended_header {
            frame_pos += ext_header.size();
        }

        while frame_pos < frame_size {
            // Its assumed the moment we've hit a zero, we've reached the padding
            if data[frame_pos] == 0 {
                break;
            }

            let frame = match frames::new(&header, &data[frame_pos..]) {
                Ok(frame) => frame,
                Err(_) => break,
            };

            // Add our new frame. Duplicate protection should be enforced with
            // the Id3Frame::key method and FrameMap::insert
            frame_pos += frame.size() + 10;
            frames.add(frame);
        }

        Ok(Tag {
            header,
            extended_header,
            frames,
        })
    }

    pub fn version(&self) -> (u8, u8) {
        (self.header.major(), self.header.minor())
    }

    pub fn size(&self) -> usize {
        self.header.size()
    }

    pub fn flags(&self) -> &TagFlags {
        self.header.flags()
    }

    pub fn flags_mut(&mut self) -> &mut TagFlags {
        self.header.flags_mut()
    }

    pub fn frames(&self) -> &FrameMap {
        &self.frames
    }

    pub fn frames_mut(&mut self) -> &mut FrameMap {
        &mut self.frames
    }

    pub fn extended_header(&self) -> &Option<ExtendedHeader> {
        &self.extended_header
    }
}
