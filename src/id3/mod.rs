pub mod frame;
mod util;

use std::io::{self, Error, ErrorKind, Read, Seek, SeekFrom};

use crate::file::File;
use frame::Id3Frame;

// TODO: ID3v4 Support
// TODO: ID3v2 Support
// TODO: iTunes support

// FIXME: Handle duplicate tags

pub struct Id3Tag<'a> {
    header: Id3TagHeader,
    frames: Vec<Box<dyn Id3Frame + 'a>>,
}

impl<'a> Id3Tag<'a> {
    pub fn new<'b>(file: &'b mut File) -> io::Result<Id3Tag<'a>> {
        // Seek to the beginning, just in case.
        file.handle.seek(SeekFrom::Start(0))?;

        let mut header_raw = [0; 10];
        file.handle.read_exact(&mut header_raw)?;

        let mut header = match Id3TagHeader::from(&header_raw) {
            Some(header) => header,
            None => return Err(Error::new(ErrorKind::InvalidData, "No ID3 header")),
        };

        // ID3 headers can also contain an extended header with more information.
        // We dont care about this, so we will skip it
        if header.has_extended_header() {
            let mut ext_size_raw = [0; 4];

            file.handle.read_exact(&mut ext_size_raw)?;

            let ext_size = util::syncsafe_decode(&ext_size_raw);

            // If our extended header is valid, we update the metadata size to reflect the fact
            // that we skipped it.
            if ext_size > 0 && (ext_size + 4) < header.size {
                header.size -= ext_size + 4;
            }
        }

        // No we can read out our raw tag data to parse.
        let mut data = vec![0; header.size];
        file.handle.read_exact(&mut data)?;

        let mut frames = Vec::new();
        let mut frame_pos: usize = 0;

        while frame_pos < header.size {
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

        // Frames are parsed, so no need to keep the data vec around now.

        return Ok(Id3Tag { header, frames });
    }

    pub fn header(&self) -> &Id3TagHeader {
        return &self.header;
    }

    pub fn frames(&self) -> &Vec<Box<dyn Id3Frame + 'a>> {
        return &self.frames;
    }
}

pub struct Id3TagHeader {
    pub major: u8,
    pub minor: u8,
    pub flags: u8,
    pub size: usize,
}

impl Id3TagHeader {
    fn from<'a>(data: &'a [u8]) -> Option<Id3TagHeader> {
        // Verify that this header has a valid ID3 Identifier
        if !data[0..3].eq(b"ID3") {
            return None;
        }

        let major = data[3];
        let minor = data[4];
        let flags = data[5];

        if major == 0xFF || minor == 0xFF {
            // Versions must be less than 0xFF
            return None;
        }

        // syncsafe_decode ensures that the size is valid
        let size = util::syncsafe_decode(&data[6..10]);

        // A size of zero is invalid, as id3 tags must have at least one frame.
        if size == 0 {
            return None;
        }

        return Some(Id3TagHeader {
            major,
            minor,
            flags,
            size,
        });
    }

    fn has_extended_header(&self) -> bool {
        return ((self.flags >> 1) & 1) == 1;
    }
}
