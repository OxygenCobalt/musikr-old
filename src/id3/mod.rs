mod util;
pub mod frame;

use std::io;
use std::io::{Error, ErrorKind};
use std::io::SeekFrom;
use std::io::prelude::*;

use frame::ID3Frame;

// TODO: ID3v4 Support
// TODO: ID3v2 Support
// TODO: iTunes support

pub struct ID3Tag {
    pub major: u8,
    pub minor: u8,
    pub flags: u8,
    pub size: usize,
    pub data: Vec<u8>
}

pub fn new(file: &mut musikr::File) -> io::Result<ID3Tag> {
    // Seek to the beginning, just in case.
    file.handle.seek(SeekFrom::Start(0)).unwrap();

    // Read out our header
    let mut header = [0; 10];
    
    file.handle.read_exact(&mut header)?;

    // Validate that this tag data begins with "ID3"
    if !header[0..3].eq(b"ID3") {
        return Err(Error::new(ErrorKind::InvalidData, "No ID3 ID"));
    }

    let major = header[3];
    let minor = header[4];
    let flags = header[5];

    if major == 0xFF || minor == 0xFF {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid Version"))
    }

    let mut size = match util::syncsafe_decode(&header[6..10]) {
        Some(size) => size,
        None => return Err(Error::new(ErrorKind::InvalidData, "Invalid Size"))
    };

    // ID3 headers can also contain an extended header with more information.
    // We dont care about this, so we will try skip it and update the size to reflect it.
    if util::has_ext_header(flags) {
        match skip_ext_header(file, size) {
            Ok(ext_size) => {
                // Update the size to reflect the skipped header
                size -= ext_size + 4;
            }

            Err(err) => if err.kind() != ErrorKind::InvalidData {
                // We can recover from a bad extended header, but not from another error
                return Err(err); 
            }
        }
    }

    // Now we can read out our raw tag data.
    let mut data = vec![0; size];

    file.handle.read_exact(&mut data)?;

    return Ok(ID3Tag {
        major, minor, flags, size, data
    });
}

pub fn read_frames<'a>(tag: &'a ID3Tag) -> Vec<Box<dyn ID3Frame + 'a>> {
    let mut frames: Vec<Box<dyn ID3Frame>> = Vec::new();
    let mut pos: usize = 0;

    while pos < tag.size {
        // Its assumed the moment we've hit a zero, we've reached the padding
        if tag.data[pos] == 0 {
            break;
        }

        let frame = match frame::new(tag, pos) {
            Some(frame) => frame,
            None => break
        };

        // Add our new frame and then move on
        pos += frame.size() + 10; 
        frames.push(frame);
    }

    return frames;
}

fn skip_ext_header(file: &mut musikr::File, metadata_size: usize) -> io::Result<usize> {
    let mut ext_size_raw = [0; 4];

    file.handle.read_exact(&mut ext_size_raw)?;

    let ext_size = util::syncsafe_decode(&ext_size_raw).unwrap_or(0);

    // Our header size should not be 0 or above the total metadata size
    if ext_size == 0 || (ext_size + 4) > metadata_size {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid ExtHeader"));
    }

    return Ok(ext_size);
}