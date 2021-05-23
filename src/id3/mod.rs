use std::io;
use std::io::{Error, ErrorKind};
use std::io::SeekFrom;
use std::io::prelude::*;

pub struct ID3Tag {
    pub major: u8,
    pub minor: u8,
    pub flags: u8,
    pub size: usize,
    pub data: Vec<u8>
}

pub fn new(mut file: musikr::File) -> io::Result<ID3Tag> {
    // Seek to the beginning, just in case.
    file.handle.seek(SeekFrom::Start(0)).unwrap();

    // Read out our header
    let mut header = [0; 10];
    
    file.handle.read(&mut header)?;

    // Validate that this tag data begins with "ID3"
    if !header[0..3].eq(b"ID3") {
        return Err(Error::new(ErrorKind::InvalidData, "No ID3 id"));
    }

    let major = header[3];
    let minor = header[4];
    let flags = header[5];
    let mut size = compute_size_usync(&header[6..10]);

    // ID3 headers can also contain an extended header with more information.
    // We dont care about this, so we will skip it and update the size if it exists
    if is_extended(flags) {
        let mut ext_size_raw = [0; 4];

        file.handle.read(&mut ext_size_raw)?;

        let ext_size = compute_size_usync(&ext_size_raw);

        size -= ext_size + 4
    }

    // Now we can read out our raw tag data.
    let mut data = Vec::with_capacity(size);

    file.handle.read(&mut data)?;

    return Ok(ID3Tag {
        major, minor, flags, size, data
    });
}

fn is_extended(flags: u8) -> bool {
    return ((flags >> 1) & 1) == 1;
}

fn compute_size_usync(raw: &[u8]) -> usize {
    return (raw[0] as usize) << 21 | 
           (raw[1] as usize) << 14 |
           (raw[2] as usize) << 7 |
           (raw[3] as usize);
}