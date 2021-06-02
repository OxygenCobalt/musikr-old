pub mod frame;
pub mod header;
mod util;

pub use header::TagHeader;
pub use header::ExtendedHeader;
use frame::Id3Frame;
use crate::file::File;
use std::io::{self, Error, ErrorKind, Read, Seek, SeekFrom};

// TODO: ID3v2.2 Support

pub struct Id3Tag<'a> {
    header: TagHeader,
    extended_header: Option<ExtendedHeader>,
    frames: Vec<Box<dyn Id3Frame + 'a>>,
}

impl<'a> Id3Tag<'a> {
    pub fn new<'b>(file: &'b mut File) -> io::Result<Id3Tag<'a>> {
        // TODO: ID3v2 tags can technically be anywhere in a while, so we have to iterate instead of
        // check the beginning

        // Seek to the beginning, just in case.
        file.handle.seek(SeekFrom::Start(0))?;

        let mut header_raw = [0; 10];
        file.handle.read_exact(&mut header_raw)?;

        let header = TagHeader::from(&header_raw).ok_or(
            Error::new(ErrorKind::InvalidData, "No ID3 Header")
        )?;

        // Read out our raw tag data.
        let mut data = vec![0; header.tag_size];
        file.handle.read_exact(&mut data)?;

        // ID3 tags can also have an extended header, which we need to account for
        let extended_header = if header.has_ext_header() { 
            ExtendedHeader::from(&data[4..])
        } else {
            None
        };

        // Begin parsing our frames, we need to adjust our frame position to account
        // for the extended header if it exists.
        let mut frames = Vec::new();
        let mut frame_pos = 0;

        if let Some(ext_header) = &extended_header {
            frame_pos += ext_header.size;
        }

        while frame_pos < header.tag_size {
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

        // Everything is parsed, so no need to keep the data vec around now.

        return Ok(Id3Tag { header, extended_header, frames });
    }

    pub fn version(&self) -> (u8, u8) {
        return (self.header.major, self.header.minor);
    }

    pub fn size(&self) -> usize {
        return self.header.tag_size;
    }

    pub fn extended_header(&self) -> &Option<ExtendedHeader> {
        return &self.extended_header;
    }

    pub fn frames(&self) -> &Vec<Box<dyn Id3Frame + 'a>> {
        return &self.frames;
    }
}
