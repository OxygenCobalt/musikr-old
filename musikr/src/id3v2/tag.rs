use crate::id3v2::{syncdata, ParseError, ParseResult};
use crate::core::raw;

pub(crate) const ID_HEADER: &[u8] = b"ID3";

pub struct TagHeader {
    major: u8,
    minor: u8,
    tag_size: usize,
    flags: TagFlags
}

impl TagHeader {
    pub(crate) fn parse(data: [u8; 10]) -> ParseResult<Self> {
        // Verify that this header has a valid ID3 Identifier
        if &data[0..3] != ID_HEADER {
            return Err(ParseError::MalformedData);
        }

        let major = data[3];
        let minor = data[4];

        if !(2..=4).contains(&major) {
            // Versions must be (TODO: 2.2), 2.3, or 2.4.
            return Err(ParseError::Unsupported);
        }

        if minor != 0 {
            // The minor version will be zero on any 2.2, 2.3, or 2.4 file.
            // This may change in the future, but the last revision was in 2000, so I dont count on it.
            return Err(ParseError::MalformedData);
        }

        // Check for invalid flags
        let flags = data[5];

        if (major == 4 && flags & 0x0F != 0) || (major == 3 && flags & 0x1F != 0) {
            return Err(ParseError::MalformedData);
        }

        let flags = TagFlags {
            unsync: raw::bit_at(7, flags),
            extended: raw::bit_at(6, flags),
            experimental: raw::bit_at(5, flags),
            footer: raw::bit_at(4, flags),
        };

        let tag_size = syncdata::to_size(&data[6..10]);

        // ID3v2 tags must be never more than 256mb.
        if tag_size > 256_000_000 {
            return Err(ParseError::MalformedData);
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
            flags: TagFlags::default(),
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
    pub(crate) fn parse(data: &[u8], major: u8) -> ParseResult<Self> {
        match major {
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
        return Err(ParseError::MalformedData);
    }

    let data = data[4..size + 4].to_vec();

    Ok(ExtendedHeader {
        size: data.len() + 4,
        data,
    })
}

fn read_ext_v4(data: &[u8]) -> Result<ExtendedHeader, ParseError> {
    // Certain taggers might have accidentally flipped the extended header byte,
    // meaning that frame data would start immediately. If that is the case, we
    // go through the size bytes and check if any of the bytes are uppercase
    // ASCII chars. This is because uppercase ASCII chars would not abide by the
    // unsynchronization scheme and are present in every frame id, official or
    // unofficial. If this check [and the size check later on] fails then we're
    // pretty much screwed, so this is the best we can do.

    // TODO: This check does not actually work. Remove it.

    let size = &data[0..4];

    for byte in size {
        if (b'A'..b'Z').contains(&byte) {
            return Err(ParseError::MalformedData);
        }
    }

    let size = syncdata::to_size(&data[0..4]);

    // ID3v2.4 extended headers aren't as clear-cut size-wise, so just check
    // if this abides by the spec [e.g bigger than 6 but still in-bounds]
    if size < 6 && (size + 4) > data.len() {
        return Err(ParseError::MalformedData);
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
        let header = TagHeader::parse(*data).unwrap();
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
        let header = TagHeader::parse(*data).unwrap();
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
        let header = ExtendedHeader::parse(&data[..], 3).unwrap();

        assert_eq!(header.data(), &vec![0x16; 6]);
    }

    #[test]
    fn parse_v4_ext_header() {
        let data = b"\x00\x00\x00\x0A\x01\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(&data[..], 4).unwrap();

        assert_eq!(header.data(), &vec![0x01, 0x16, 0x16, 0x16, 0x16, 0x16]);
    }
}
