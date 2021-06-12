pub mod frames;
pub mod header;
mod syncdata;

use crate::file::File;
use frames::FrameMap;
pub use header::ExtendedHeader;
pub use header::TagFlags;
pub use header::TagHeader;
use std::io::{self, Error, ErrorKind, Read, Seek, SeekFrom};

// TODO: ID3v2.2 Conversions

pub struct Tag {
    header: TagHeader,
    extended_header: Option<ExtendedHeader>,
    frames: FrameMap,
}

impl Tag {
    pub fn new(file: &mut File) -> io::Result<Tag> {
        // TODO: ID3 tags can actually be in multiple places, you'll need to do this:
        // - Look for a starting tag initially
        // - Use SEEK frames to find more information
        // - Look backwards for an appended tag
        // Also split this off into seperate functions.

        // Seek to the beginning, just in case.
        file.handle.seek(SeekFrom::Start(0)).ok();

        // Then read and parse the possible ID3 header
        let mut header_raw = [0; 10];
        file.handle.read_exact(&mut header_raw)?;

        let header = TagHeader::parse(&header_raw)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Malformed Header"))?;

        // Read out our raw tag data.
        let mut data = vec![0; header.tag_size];
        file.handle.read_exact(&mut data)?;

        // Decode unsynced tag data if it exists
        if header.flags.unsync {
            data = syncdata::decode(&data);
        }

        // ID3 tags can also have an extended header, which we will not fully parse but still account for.
        // We don't need to do this for the footer as it's just a clone of the header
        let extended_header = if header.flags.extended {
            ExtendedHeader::parse(&data[4..])
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
