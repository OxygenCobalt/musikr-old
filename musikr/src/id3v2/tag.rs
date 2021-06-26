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
            return Err(ParseError::Unsupported)
        }

        if minor != 0 {
            // In ID3v2.2, v2.3, and v2.4, the minor byte is always zero.
            // This may change in the future, but the last revision was in 2000, so I doubt it.
            return Err(ParseError::MalformedData)
        }

        let flags = raw[5];

        // Check for invalid flags
        if (major == 4 && flags & 0x0F != 0) || (major == 3 && flags & 0x1F != 0) {
            return Err(ParseError::MalformedData);
        }

        let flags = TagFlags {
            unsync: flags & 0x80 != 0,
            extended: flags & 0x40 != 0,
            experimental: flags & 0x20 != 0,
            footer: flags & 0x10 != 0,
        };

        // Tag size is always 4 bytes, so we can unwrap here
        let tag_size = syncdata::to_size(&raw[6..10].try_into().unwrap());

        // ID3v2 tags must be at least 1 byte and never more than 256mb.
        if tag_size == 0 || tag_size > 256_000_000 {
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
    pub padding_size: Option<usize>,
    pub crc32: Option<u32>,
    pub is_update: bool,
    pub restrictions: Option<Restrictions>
}

impl ExtendedHeader {
    pub(crate) fn parse(stream: &mut BufStream, major: u8) -> ParseResult<Self> {
        match major {
            3 => parse_ext_v3(stream),
            4 => parse_ext_v4(stream),
            _ => Err(ParseError::Unsupported)
        }
    }
}

fn parse_ext_v3(stream: &mut BufStream) -> ParseResult<ExtendedHeader> {
    let size = stream.read_u32()? as usize;

    // The extended header should be 6 or 10 bytes
    if size != 6 && size != 10 {
        return Err(ParseError::MalformedData);
    }

    let flags = stream.read_u16()?;

    let mut header = ExtendedHeader {
        padding_size: Some(stream.read_u32()? as usize),
        crc32: None,
        is_update: false,
        restrictions: None
    };

    if flags & 0x8000 == 0x8000 {
        header.crc32 = Some(stream.read_u32()?)
    }

    Ok(header)
}

fn parse_ext_v4(stream: &mut BufStream) -> ParseResult<ExtendedHeader> {
    // Neither the size and flag count are that useful when parsing the v4 extended header, so 
    // we largely ignore them.
    stream.skip(4)?;

    if stream.read_u8()? != 1 {
        return Err(ParseError::MalformedData)
    }

    let mut header = ExtendedHeader {
        padding_size: None,
        crc32: None,
        is_update: false,
        restrictions: None
    };

    let flags = stream.read_u8()?;

    // Tag is an update.
    if flags & 0x40 != 0 {
        // Flag must have no accompanying data.
        if stream.read_u8()? != 0 {
            return Err(ParseError::MalformedData)
        }

        header.is_update = true;
    }
    
    // CRC-32 data.
    if flags & 0x20 != 0 {
        // Restrictions must be a 32-bit syncsafe integer.
        if stream.read_u8()? != 5 {
            return Err(ParseError::MalformedData)
        }

        header.crc32 = Some(syncdata::read_u32(stream)?);
    }

    // Tag restrictions. Musikr doesnt really do anything with these since according to the spec
    // they are only flags for when the tag was *encoded*, now how it should *decode*.
    if flags & 0x10 != 0 {
        // Restrictions must be 1 byte in length.
        if stream.read_u8()? != 1 {
            return Err(ParseError::MalformedData)
        }

        let restrictions = stream.read_u8()?;

        let tag_size = match restrictions >> 6 {
            0 => TagSizeRestriction::Max128Frames1Mb,
            1 => TagSizeRestriction::Max64Frames128Kb,
            2 => TagSizeRestriction::Max32Frames40Kb,
            3 => TagSizeRestriction::Max32Frames4Kb,
            _ => unreachable!()
        };

        let text_encoding = match (restrictions & 0x20) >> 5 {
            0 => TextEncodingRestriction::None,
            1 => TextEncodingRestriction::Latin1OrUtf8,
            _ => unreachable!()
        };

        let text_size = match (restrictions & 0x18) >> 3 {
            0 => TextSizeRestriction::None,
            1 => TextSizeRestriction::LessThan1024Chars,
            2 => TextSizeRestriction::LessThan128Chars,
            3 => TextSizeRestriction::LessThan30Chars,
            _ => unreachable!()     
        };

        let image_encoding = match (restrictions & 0x4) >> 2 {
            0 => ImageEncodingRestriction::None,
            1 => ImageEncodingRestriction::OnlyPngOrJpeg,
            _ => unreachable!()
        };

        let image_size = match (restrictions & 0x3) >> 1 {
            0 => ImageSizeRestriction::None,
            1 => ImageSizeRestriction::LessThan256x256,
            2 => ImageSizeRestriction::LessThan64x64,
            3 => ImageSizeRestriction::Exactly64x64,
            _ => unreachable!()
        };

        header.restrictions = Some(Restrictions {
            tag_size,
            text_encoding,
            text_size,
            image_encoding,
            image_size
        })
    }

    Ok(header)
}

#[derive(Debug, Eq, PartialEq)]
pub struct Restrictions {
    pub tag_size: TagSizeRestriction,
    pub text_encoding: TextEncodingRestriction,
    pub text_size: TextSizeRestriction,
    pub image_encoding: ImageEncodingRestriction,
    pub image_size: ImageSizeRestriction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TagSizeRestriction {
    Max128Frames1Mb,
    Max64Frames128Kb,
    Max32Frames40Kb,
    Max32Frames4Kb
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncodingRestriction {
    None,
    Latin1OrUtf8
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextSizeRestriction {
    None,
    LessThan1024Chars,
    LessThan128Chars,
    LessThan30Chars
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageEncodingRestriction {
    None,
    OnlyPngOrJpeg
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageSizeRestriction {
    None,
    LessThan256x256,
    LessThan64x64,
    Exactly64x64
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
        let data = b"\x00\x00\x00\x06\x80\x00\xAB\xCD\xEF\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(&mut BufStream::new(data), 3).unwrap();

        assert_eq!(header.padding_size, Some(0xABCDEF16));
        assert_eq!(header.crc32, Some(0x16161616));
        assert!(!header.is_update);
        assert_eq!(header.restrictions, None);
    }

    #[test]
    fn parse_v4_ext_header() {
        let data = b"\x00\x00\x00\x0D\x01\x70\x00\x05\x0A\x5E\x37\x5E\x16\x01\xB4";
        let header = ExtendedHeader::parse(&mut BufStream::new(data), 4).unwrap();

        assert_eq!(header.padding_size, None);
        assert_eq!(header.crc32, Some(0x2BCDEF16));
        assert!(header.is_update);

        let restrictions = header.restrictions.unwrap();

        assert_eq!(restrictions.tag_size, TagSizeRestriction::Max32Frames40Kb);
        assert_eq!(restrictions.text_encoding, TextEncodingRestriction::Latin1OrUtf8);
        assert_eq!(restrictions.text_size, TextSizeRestriction::LessThan128Chars);
        assert_eq!(restrictions.image_encoding, ImageEncodingRestriction::OnlyPngOrJpeg);
        assert_eq!(restrictions.image_size, ImageSizeRestriction::None);
    }
}
