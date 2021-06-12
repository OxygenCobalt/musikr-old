pub mod frames;
pub mod header;
mod search;
mod syncdata;

pub use header::{ExtendedHeader, TagFlags, TagHeader};
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

#[derive(Debug)]
pub enum ParseError {
    NotEnoughData,
    InvalidData,
    InvalidEncoding,
    Unsupported,
    NotFound
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for ParseError {}

impl Tag {
    pub fn new(file: &mut File, offset: u64) -> io::Result<Tag> {
        // TODO: ID3 tags can actually be in multiple places, you'll need to do this:
        // - Look for a starting tag initially
        // - Use SEEK frames to find more information
        // - Look backwards for an appended tag
        // Also split this off into seperate functions.

        file.seek(offset).ok();

        // Read and parse the possible ID3 header
        let mut header_raw = [0; 10];
        file.read_into(&mut header_raw)?;

        let header =
            TagHeader::parse(&header_raw).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

        let mut data = file.read_bytes(header.tag_size)?;

        // Decode unsynced tag data if it exists
        if header.flags.unsync {
            data = syncdata::decode(&data);
        }

        // ID3 tags can also have an extended header, which we will not fully parse but still account for.
        // We don't need to do this for the footer as it's just a clone of the header
        // Since its very possible that the extended header flag was accidentally flipped, we will default
        // to an None if this fails.
        let extended_header = if header.flags.extended {
            ExtendedHeader::parse(&data[4..]).ok()
        } else {
            None
        };

        let mut frames = FrameMap::new();
        let mut frame_pos = 0;
        let mut frame_size = header.tag_size;

        // Begin parsing our frames, we need to adjust our frame data to account
        // for the extended header and footer [if they exist]
        if header.flags.footer {
            frame_size -= 10;
        }

        if let Some(ext_header) = &extended_header {
            frame_pos += ext_header.size;
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
        (self.header.major, self.header.minor)
    }

    pub fn size(&self) -> usize {
        self.header.tag_size
    }

    pub fn flags(&self) -> &TagFlags {
        &self.header.flags
    }

    pub fn flags_mut(&mut self) -> &mut TagFlags {
        &mut self.header.flags
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
