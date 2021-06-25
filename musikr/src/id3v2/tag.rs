use crate::core::io::BufStream;
use crate::id3v2::{syncdata, ParseError, ParseResult};
use std::convert::TryInto;

pub(crate) const ID_HEADER: &[u8] = b"ID3";

pub struct TagHeader {
    major: u8,
    minor: u8,
    tag_size: usize,
    flags: TagFlags,
}

impl TagHeader {
    pub(crate) fn parse(raw: [u8; 10]) -> ParseResult<Self> {
        // Verify that this header has a valid ID3 Identifier
        if &raw[0..3] != ID_HEADER {
            return Err(ParseError::MalformedData);
        }

        let major = raw[3];
        let minor = raw[4];

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
        let flags = raw[5];

        if (major == 4 && flags & 0x0F != 0) || (major == 3 && flags & 0x1F != 0) {
            return Err(ParseError::MalformedData);
        }

        let flags = TagFlags {
            unsync: flags & 0x80 == 0x80,
            extended: flags & 0x40 == 0x40,
            experimental: flags & 0x20 == 0x20,
            footer: flags & 0x10 == 0x10,
        };

        // Tag size is always 4 bytes, so we can unwrap here
        let tag_size = syncdata::to_size(&raw[6..10].try_into().unwrap());

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
    pub(crate) fn parse(stream: &mut BufStream, major: u8) -> ParseResult<Self> {
        match major {
            3 => read_ext_v3(stream),
            4 => read_ext_v4(stream),
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

fn read_ext_v3(stream: &mut BufStream) -> Result<ExtendedHeader, ParseError> {
    let size = stream.read_u32()? as usize;

    // The extended header should be 6 or 10 bytes
    if size != 6 && size != 10 {
        return Err(ParseError::MalformedData);
    }

    let mut data = vec![0; size];
    stream.read_exact(&mut data)?;

    Ok(ExtendedHeader {
        size: data.len() + 4,
        data,
    })
}

fn read_ext_v4(stream: &mut BufStream) -> Result<ExtendedHeader, ParseError> {
    let size = syncdata::read_size(stream)?;

    // ID3v2.4 extended headers are at mininum 6 bytes and at maximum 13 bytes.
    if !(6..=13).contains(&size) {
        return Err(ParseError::MalformedData);
    }

    let mut data = vec![0; size];
    stream.read_exact(&mut data)?;

    Ok(ExtendedHeader { size, data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::io::BufStream;

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
        let header = ExtendedHeader::parse(&mut BufStream::new(data), 3).unwrap();

        assert_eq!(header.data(), &vec![0x16; 6]);
    }

    #[test]
    fn parse_v4_ext_header() {
        let data = b"\x00\x00\x00\x06\x01\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(&mut BufStream::new(data), 4).unwrap();

        assert_eq!(header.data(), &vec![0x01, 0x16, 0x16, 0x16, 0x16, 0x16]);
    }
}
