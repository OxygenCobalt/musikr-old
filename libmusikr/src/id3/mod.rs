pub mod frame;
pub mod header;
mod syncdata;

use crate::file::File;
use crate::id3::frame::Id3Frame;
pub use header::ExtendedHeader;
pub use header::TagHeader;
use std::io::{self, Error, ErrorKind, Read, Seek, SeekFrom};

// TODO: ID3v2.2 Support

pub struct Id3Tag<'a> {
    header: TagHeader,
    extended_header: Option<ExtendedHeader>,
    frames: Vec<Box<dyn Id3Frame + 'a>>,
}

impl<'a> Id3Tag<'a> {
    pub fn new<'b>(file: &'b mut File) -> io::Result<Id3Tag<'a>> {
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

        let header = TagHeader::new(&header_raw)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Malformed Header"))?;

        // Read out our raw tag data.
        let mut data = vec![0; header.tag_size];
        file.handle.read_exact(&mut data)?;

        // Decode unsynced tag data if it exists
        if header.unsync {
            data = syncdata::decode(&data);
        }

        // ID3 tags can also have an extended header, which we will not fully parse but still account for.
        // We don't need to do this for the footer as it's just a clone of the header
        let extended_header = if header.extended {
            ExtendedHeader::new(&data[4..])
        } else {
            None
        };

        // Begin parsing our frames, we need to adjust our frame data to account
        // for the extended header and footer [if they exist]
        let mut frames = Vec::new();
        let mut frame_pos = 0;
        let mut frame_size = header.tag_size;

        if header.footer {
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

            let frame = match frame::new(&header, &data[frame_pos..]) {
                Some(frame) => frame,
                None => break,
            };

            // Add our new frame and then move on
            frame_pos += frame.size() + 10;
            frames.push(frame);
        }

        Ok(Id3Tag {
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

    pub fn frames(&self) -> &Vec<Box<dyn Id3Frame + 'a>> {
        &self.frames
    }

    pub fn extended_header(&self) -> &Option<ExtendedHeader> {
        &self.extended_header
    }
}
