use crate::id3v2::{syncdata, ParseError};
use crate::raw;

pub(crate) const ID_HEADER: &[u8] = b"ID3";

pub(crate) struct TagHeader {
    pub major: u8,
    pub minor: u8,
    pub tag_size: usize,
    pub flags: TagFlags,
}

impl TagHeader {
    pub(crate) fn parse(data: &[u8]) -> Result<Self, ParseError> {
        // Verify that this header has a valid ID3 Identifier
        if !data[0..3].eq(ID_HEADER) {
            return Err(ParseError::InvalidData);
        }

        let major = data[3];
        let minor = data[4];

        if major == 0xFF || minor == 0xFF {
            // Versions cannot be 0xFF
            return Err(ParseError::InvalidData);
        }

        if !(2..5).contains(&major) {
            // Versions must be 2.2, 2.3, or 2.4.
            return Err(ParseError::Unsupported);
        }

        // Check for invalid flags
        let flags = data[5];

        if (major == 4 && flags & 0x0F != 0) || (major == 3 && flags & 0x1F != 0) {
            return Err(ParseError::InvalidData);
        }

        let flags = TagFlags::parse(data[5]);
        let tag_size = syncdata::to_size(&data[6..10]);

        // ID3v2 tags must be at least 1 byte and never more than 256mb.
        if tag_size == 0 || tag_size > 256000000 {
            return Err(ParseError::InvalidData);
        }

        Ok(TagHeader {
            major,
            minor,
            tag_size,
            flags,
        })
    }
}

pub struct TagFlags {
    pub unsync: bool,
    pub extended: bool,
    pub experimental: bool,
    pub footer: bool,
}

impl TagFlags {
    fn new() -> Self {
        TagFlags {
            unsync: false,
            extended: false,
            experimental: false,
            footer: false,
        }
    }

    fn parse(flags: u8) -> Self {
        TagFlags {
            unsync: raw::bit_at(0, flags),
            extended: raw::bit_at(1, flags),
            experimental: raw::bit_at(2, flags),
            footer: raw::bit_at(3, flags),
        }
    }
}

pub struct ExtendedHeader {
    pub size: usize,
    pub data: Vec<u8>,
}

impl ExtendedHeader {
    pub(crate) fn parse(major: u8, data: &[u8]) -> Result<Self, ParseError> {
        match major {
            3 => read_ext_v3(data),
            4 => read_ext_v4(data),
            _ => Err(ParseError::Unsupported),
        }
    }
}

fn read_ext_v3(data: &[u8]) -> Result<ExtendedHeader, ParseError> {
    let size = raw::to_size(&data[0..4]);

    // The extended header should be 6 or 10 bytes
    if size != 6 && size != 10 {
        return Err(ParseError::InvalidData);
    }

    let data = data[4..size + 4].to_vec();

    Ok(ExtendedHeader { size, data })
}

fn read_ext_v4(data: &[u8]) -> Result<ExtendedHeader, ParseError> {
    // Certain taggers might have accidentally flipped the extended header byte,
    // meaning that frame data would start immediately. If that is the case, we
    // go through the size bytes and check if any of the bytes are uppercase
    // ASCII chars. This is because uppercase ASCII chars would not abide by the
    // unsynchronization scheme and are present in every frame id, official or
    // unofficial. If this check [and the size check later on] fails then we're
    // pretty much screwed, so this is the best we can do.

    let size = &data[0..4];

    for byte in size {
        if (b'A'..b'Z').contains(&byte) {
            return Err(ParseError::InvalidData);
        }
    }

    let size = syncdata::to_size(&data[0..4]);

    // ID3v2.4 extended headers aren't as clear-cut size-wise, so just check
    // if this abides by the spec [e.g bigger than 6 but still in-bounds]
    if size < 6 && (size + 4) > data.len() {
        return Err(ParseError::InvalidData);
    }

    let data = data[4..size].to_vec();

    Ok(ExtendedHeader { size, data })
}
