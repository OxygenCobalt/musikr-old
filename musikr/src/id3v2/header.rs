use crate::id3v2::{syncdata, ParseError};
use crate::raw;

pub(crate) const ID_HEADER: &[u8] = b"ID3";

pub struct TagHeader {
    major: u8,
    minor: u8,
    tag_size: usize,
    flags: TagFlags,
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
        if tag_size == 0 || tag_size > 256_000_000 {
            return Err(ParseError::InvalidData);
        }

        Ok(TagHeader {
            major,
            minor,
            tag_size,
            flags,
        })
    }
    
    #[cfg(test)]
    pub(crate) fn with_version(major: u8) -> Self {
        TagHeader {
            major,
            minor: 0,
            tag_size: 0,
            flags: TagFlags::new(),
        }
    }

    pub fn major(&self) -> u8 {
        self.major
    }

    pub fn minor(&self) -> u8 {
        self.minor
    }

    pub fn size(&self) -> usize {
        self.tag_size
    }

    pub(crate) fn size_mut(&mut self) -> &mut usize {
        &mut self.tag_size
    }

    pub fn flags(&self) -> &TagFlags {
        &self.flags
    }

    pub(crate) fn flags_mut(&mut self) -> &mut TagFlags {
        &mut self.flags
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
        Self::default()
    }

    fn parse(flags: u8) -> Self {
        TagFlags {
            unsync: raw::bit_at(7, flags),
            extended: raw::bit_at(6, flags),
            experimental: raw::bit_at(5, flags),
            footer: raw::bit_at(4, flags),
        }
    }
}

impl Default for TagFlags {
    fn default() -> Self {
        TagFlags {
            unsync: false,
            extended: false,
            experimental: false,
            footer: false,
        }
    }
}

pub struct ExtendedHeader {
    size: usize,
    data: Vec<u8>,
}

impl ExtendedHeader {
    pub(crate) fn parse(major_version: u8, data: &[u8]) -> Result<Self, ParseError> {
        match major_version {
            3 => read_ext_v3(data),
            4 => read_ext_v4(data),
            _ => Err(ParseError::Unsupported),
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v3_tag_header() {
        let data = b"\x49\x44\x33\x03\x00\xA0\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.size(), 140464);
        assert_eq!(header.major(), 3);
        assert_eq!(header.minor(), 0);

        assert!(flags.unsync);
        assert!(!flags.extended);
        assert!(flags.experimental);
    }

    #[test]
    fn parse_v4_tag_header() {
        let data = b"\x49\x44\x33\x04\x00\x50\x00\x08\x49\x30";
        let header = TagHeader::parse(&data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.size(), 140464);
        assert_eq!(header.major(), 4);
        assert_eq!(header.minor(), 0);

        assert!(!flags.unsync);
        assert!(flags.extended);
        assert!(!flags.experimental);
        assert!(flags.footer);
    }

    #[test]
    fn parse_v3_ext_header() {
        let data = b"\x00\x00\x00\x06\x16\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(3, &data[..]).unwrap();

        assert_eq!(header.size(), 6);
        assert_eq!(header.data(), &vec![0x16; 6]);
    }

    #[test]
    fn parse_v4_ext_header() {
        let data = b"\x00\x00\x00\x0A\x01\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(4, &data[..]).unwrap();

        assert_eq!(header.size(), 10);
        assert_eq!(header.data(), &vec![0x01, 0x16, 0x16, 0x16, 0x16, 0x16]);
    }
}
