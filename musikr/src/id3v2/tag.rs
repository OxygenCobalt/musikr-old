//! Tag headers and meta information.
//!
//! This module contains the items for the ID3v2 header, version, and extended header.

use crate::core::io::BufStream;
use crate::id3v2::{syncdata, ParseError, ParseResult};
use std::convert::TryInto;

const ID: &[u8] = b"ID3";

#[derive(Clone, Debug)]
pub struct TagHeader {
    version: Version,
    tag_size: u32,
    flags: TagFlags,
}

impl TagHeader {
    pub(crate) fn parse(raw: [u8; 10]) -> ParseResult<Self> {
        // Verify that this header has a valid ID3 Identifier
        if &raw[0..3] != ID {
            return Err(ParseError::MalformedData);
        }

        // Get the version of this tag.
        // Technically, ID3v2.2 is never an actual case when it comes to the tag version, instead, it
        // gets upgraded to ID3v2.3 immediately. However, throwing the ID3v2.2 tag on a completely
        // seperate upgrade path is much less elegant than just having a useless [or even invalid]
        // enum variant. This isnt ideal, but its the best we can do.
        let version = match (raw[3], raw[4]) {
            (2, 0) => Version::V22,
            (3, 0) => Version::V23,
            (4, 0) => Version::V24,
            _ => return Err(ParseError::Unsupported),
        };

        let flags = raw[5];

        // Treat any unused flags being set as malformed data.
        if (version == Version::V22 && flags & 0x4F != 0)
            || (version == Version::V23 && flags & 0x1F != 0)
            || (version == Version::V24 && flags & 0x0f != 0)
        {
            return Err(ParseError::MalformedData);
        }

        let flags = TagFlags {
            unsync: flags & 0x80 != 0,
            extended: flags & 0x40 != 0,
            experimental: flags & 0x20 != 0,
            footer: flags & 0x10 != 0,
        };

        // Tag size is always 4 bytes, so we can unwrap here
        let tag_size = syncdata::to_u28(raw[6..10].try_into().unwrap());

        // ID3v2 tags must be at least 1 byte and never more than 256mb.
        if tag_size == 0 || tag_size > 256_000_000 {
            return Err(ParseError::MalformedData);
        }

        Ok(Self {
            version,
            tag_size,
            flags,
        })
    }

    pub(crate) fn with_version(version: Version) -> Self {
        Self {
            version,
            tag_size: 0,
            flags: TagFlags::default(),
        }
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn size(&self) -> u32 {
        self.tag_size
    }

    pub fn flags(&self) -> TagFlags {
        self.flags
    }

    pub(crate) fn flags_mut(&mut self) -> &mut TagFlags {
        &mut self.flags
    }
}

// The version of an ID3v2 tag.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Version {
    /// ID3v2.2,
    V22,
    /// ID3v2.3
    V23,
    /// ID3v2.4
    V24,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct TagFlags {
    pub unsync: bool,
    pub extended: bool,
    pub experimental: bool,
    pub footer: bool,
}

#[derive(Default, Debug, Clone)]
pub struct ExtendedHeader {
    pub padding_size: Option<usize>,
    pub crc32: Option<u32>,
    pub is_update: bool,
    pub restrictions: Option<Restrictions>,
}

impl ExtendedHeader {
    pub(crate) fn parse(stream: &mut BufStream, version: Version) -> ParseResult<Self> {
        match version {
            Version::V22 => Err(ParseError::Unsupported),
            Version::V23 => parse_ext_v3(stream),
            Version::V24 => parse_ext_v4(stream),
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
        restrictions: None,
    };

    if flags & 0x8000 != 0 {
        header.crc32 = Some(stream.read_u32()?)
    }

    Ok(header)
}

fn parse_ext_v4(stream: &mut BufStream) -> ParseResult<ExtendedHeader> {
    let size = syncdata::to_u28(stream.read_array()?);

    // A full extended header should only be 15 bytes.
    if size > 15 {
        return Err(ParseError::MalformedData);
    }

    // The flag count is always 1.
    if stream.read_u8()? != 1 {
        return Err(ParseError::MalformedData);
    }

    let mut header = ExtendedHeader {
        padding_size: None,
        crc32: None,
        is_update: false,
        restrictions: None,
    };

    let flags = stream.read_u8()?;

    // Tag is an update.
    if flags & 0x40 != 0 {
        // Flag must have no accompanying data.
        if stream.read_u8()? != 0 {
            return Err(ParseError::MalformedData);
        }

        header.is_update = true;
    }

    // CRC-32 data.
    if flags & 0x20 != 0 {
        // CRC-32 data must be a 32-bit syncsafe integer.
        if stream.read_u8()? != 5 {
            return Err(ParseError::MalformedData);
        }

        header.crc32 = Some(syncdata::to_u35(stream.read_array()?));
    }

    // Tag restrictions. Musikr doesnt really do anything with these since according to the spec
    // they are only flags for when the tag was *encoded*, now how it should *decode*.
    if flags & 0x10 != 0 {
        // Restrictions must be 1 byte in length.
        if stream.read_u8()? != 1 {
            return Err(ParseError::MalformedData);
        }

        let restrictions = stream.read_u8()?;

        let tag_size = match restrictions >> 6 {
            0 => TagSizeRestriction::Max128Frames1Mb,
            1 => TagSizeRestriction::Max64Frames128Kb,
            2 => TagSizeRestriction::Max32Frames40Kb,
            3 => TagSizeRestriction::Max32Frames4Kb,
            _ => unreachable!(),
        };

        let text_encoding = match (restrictions & 0x20) >> 5 {
            0 => TextEncodingRestriction::None,
            1 => TextEncodingRestriction::Latin1OrUtf8,
            _ => unreachable!(),
        };

        let text_size = match (restrictions & 0x18) >> 3 {
            0 => TextSizeRestriction::None,
            1 => TextSizeRestriction::LessThan1024Chars,
            2 => TextSizeRestriction::LessThan128Chars,
            3 => TextSizeRestriction::LessThan30Chars,
            _ => unreachable!(),
        };

        let image_encoding = match (restrictions & 0x4) >> 2 {
            0 => ImageEncodingRestriction::None,
            1 => ImageEncodingRestriction::OnlyPngOrJpeg,
            _ => unreachable!(),
        };

        let image_size = match (restrictions & 0x3) >> 1 {
            0 => ImageSizeRestriction::None,
            1 => ImageSizeRestriction::LessThan256x256,
            2 => ImageSizeRestriction::LessThan64x64,
            3 => ImageSizeRestriction::Exactly64x64,
            _ => unreachable!(),
        };

        header.restrictions = Some(Restrictions {
            tag_size,
            text_encoding,
            text_size,
            image_encoding,
            image_size,
        })
    }

    Ok(header)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    Max32Frames4Kb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncodingRestriction {
    None,
    Latin1OrUtf8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextSizeRestriction {
    None,
    LessThan1024Chars,
    LessThan128Chars,
    LessThan30Chars,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageEncodingRestriction {
    None,
    OnlyPngOrJpeg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageSizeRestriction {
    None,
    LessThan256x256,
    LessThan64x64,
    Exactly64x64,
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
        assert_eq!(header.version(), Version::V23);

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
        assert_eq!(header.version(), Version::V24);

        assert!(!flags.unsync);
        assert!(flags.extended);
        assert!(!flags.experimental);
        assert!(flags.footer);
    }

    #[test]
    fn parse_v3_ext_header() {
        let data = b"\x00\x00\x00\x06\x80\x00\xAB\xCD\xEF\x16\x16\x16\x16\x16";
        let header = ExtendedHeader::parse(&mut BufStream::new(data), Version::V23).unwrap();

        assert_eq!(header.padding_size, Some(0xABCDEF16));
        assert_eq!(header.crc32, Some(0x16161616));
        assert!(!header.is_update);
        assert_eq!(header.restrictions, None);
    }

    #[test]
    fn parse_v4_ext_header() {
        let data = b"\x00\x00\x00\x0D\x01\x70\x00\x05\x0A\x5E\x37\x5E\x16\x01\xB4";
        let header = ExtendedHeader::parse(&mut BufStream::new(data), Version::V24).unwrap();

        assert_eq!(header.padding_size, None);
        assert_eq!(header.crc32, Some(0x2BCDEF16));
        assert!(header.is_update);

        let restrictions = header.restrictions.unwrap();

        assert_eq!(restrictions.tag_size, TagSizeRestriction::Max32Frames40Kb);
        assert_eq!(
            restrictions.text_encoding,
            TextEncodingRestriction::Latin1OrUtf8
        );
        assert_eq!(
            restrictions.text_size,
            TextSizeRestriction::LessThan128Chars
        );
        assert_eq!(
            restrictions.image_encoding,
            ImageEncodingRestriction::OnlyPngOrJpeg
        );
        assert_eq!(restrictions.image_size, ImageSizeRestriction::None);
    }
}
