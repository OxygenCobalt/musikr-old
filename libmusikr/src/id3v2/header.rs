use crate::id3v2::{syncdata, ParseError};
use crate::raw;

pub struct TagHeader {
    pub major: u8,
    pub minor: u8,
    pub tag_size: usize,
    pub flags: TagFlags,
}

impl TagHeader {
    pub(crate) fn parse(data: &[u8]) -> Result<Self, ParseError> {
        // Verify that this header has a valid ID3 Identifier
        if !data[0..3].eq(b"ID3") {
            return Err(ParseError::InvalidData);
        }

        let major = data[3];
        let minor = data[4];

        if major == 0xFF || minor == 0xFF {
            // Versions must be less than 0xFF
            return Err(ParseError::InvalidData);
        }

        // Read flags
        let flags = data[5];
        let unsync = raw::bit_at(0, flags);
        let extended = raw::bit_at(1, flags);
        let experimental = raw::bit_at(2, flags);
        let footer = raw::bit_at(3, flags);

        let tag_size = syncdata::to_size(&data[6..10]);

        // A size of zero is invalid, as id3 tags must have at least one frame.
        if tag_size == 0 {
            return Err(ParseError::NotEnoughData);
        }

        Ok(TagHeader {
            major,
            minor,
            tag_size,
            flags: TagFlags {
                unsync,
                extended,
                experimental,
                footer,
            },
        })
    }
}

pub struct TagFlags {
    pub unsync: bool,
    pub extended: bool,
    pub experimental: bool,
    pub footer: bool,
}

pub struct ExtendedHeader {
    pub size: usize,
    pub data: Vec<u8>,
}

impl ExtendedHeader {
    pub(crate) fn parse(data: &[u8]) -> Result<Self, ParseError> {
        // We don't exactly care about parsing the extended header, but we do
        // keep it around when it's time to write new tag information
        let size = syncdata::to_size(&data[0..4]);

        // Validate that this header is valid.
        if size == 0 && (size + 4) > data.len() {
            return Err(ParseError::InvalidData);
        }

        let data = data[4..size].to_vec();

        Ok(ExtendedHeader { size, data })
    }
}