//! Tag headers and meta information.
//!
//! This module contains the items for the ID3v2 header, version, and extended header.

use crate::core::io::BufStream;
use crate::id3v2::{syncdata, ParseError, ParseResult};
use log::error;
use std::fmt::{self, Display, Formatter};
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
            error!("no id3v2 identifier found");
            return Err(ParseError::NotFound);
        }

        // Get the version of this tag.
        let version = match (raw[3], raw[4]) {
            (2, 0) => Version::V22,
            (3, 0) => Version::V23,
            (4, 0) => Version::V24,
            (m, _) => {
                error!("ID3v2.{} is not supported", m);
                return Err(ParseError::Unsupported);
            }
        };

        let flags = raw[5];

        // Treat any unused flags being set as malformed data.
        if (version == Version::V22 && flags & 0x4F != 0)
            || (version == Version::V23 && flags & 0x1F != 0)
            || (version == Version::V24 && flags & 0x0f != 0)
        {
            error!("unused flags are set on the tag header");
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
            error!("tag size can only be 1b..256mb");
            return Err(ParseError::MalformedData);
        }

        Ok(Self {
            version,
            tag_size,
            flags,
        })
    }

    pub(crate) fn render(&mut self) -> [u8; 10] {
        assert_ne!(self.version, Version::V22);

        let mut header = [b'I', b'D', b'3', 0, 0, 0, 0, 0, 0, 0];

        // Write out the major version. The header at this point should have 
        // been upgraded, so ID3v2.2 shouldn't be a possibility.
        match self.version {
            Version::V24 => header[3] = 4,
            Version::V23 => header[3] = 3,
            Version::V22 => unreachable!()
        };

        // Add tag flags
        header[5] |= u8::from(self.flags.unsync) * 0x80;
        header[5] |= u8::from(self.flags.extended) * 0x40;
        header[5] |= u8::from(self.flags.experimental) * 0x20; 
        header[5] |= u8::from(self.flags.footer) * 0x10; 

        // ID3v2 tag sizes are always syncsafe
        header[6..10].copy_from_slice(&syncdata::from_u28(self.tag_size));

        header
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

    pub(crate) fn flags(&self) -> TagFlags {
        self.flags
    }

    pub(crate) fn version_mut(&mut self) -> &mut Version {
        &mut self.version
    }

    pub(crate) fn size_mut(&mut self) -> &mut u32 {
        &mut self.tag_size
    }

    pub(crate) fn flags_mut(&mut self) -> &mut TagFlags {
        &mut self.flags
    }
}

/// The overall flags for a tag. This is meant for internal use.
#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct TagFlags {
    pub unsync: bool,
    pub extended: bool,
    pub experimental: bool,
    pub footer: bool,
}

/// The version of an ID3v2 tag.
///
/// This enum represents the current version of a tag.
/// This cannot be used for writing tags. Instead, use [`SaveVersion`](SaveVersion)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Version {
    /// ID3v2.2.
    V22,
    /// ID3v2.3.
    V23,
    /// ID3v2.4.
    V24,
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::V22 => write![f, "ID3v2.2"],
            Self::V23 => write![f, "ID3v2.3"],
            Self::V24 => write![f, "ID3v2.4"]
        }
    }
}

impl From<SaveVersion> for Version {
    fn from(other: SaveVersion) -> Self {
        match other {
            SaveVersion::V23 => Version::V23,
            SaveVersion::V24 => Version::V24,
        }
    }
}

/// The version to save an ID3v2 tag with.
///
/// This enum differs from [`Version`](Version) in that it represents the ID3v2 versions
/// that musikr can create and write, that being ID3v2.3 and ID3v2.4. It is primarily used
/// during creation, upgrading, or saving operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SaveVersion {
    /// ID3v2.3.
    V23,
    /// ID3v2.4.
    V24,
}

#[derive(Default, Debug, Clone)]
pub struct ExtendedHeader {
    pub padding_size: Option<u32>,
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

    pub(crate) fn render(&self, version: Version) -> Vec<u8> {
        assert_ne!(version, Version::V22);

        match version {
            Version::V24 => render_ext_v4(self),
            Version::V23 => render_ext_v3(self),
            Version::V22 => unreachable!()
        }
    }

    pub(crate) fn update(&mut self, to: SaveVersion) {
        match to {
            SaveVersion::V23 => {
                self.padding_size = Some(0);
                self.is_update = false;
                self.restrictions = None;
            },

            SaveVersion::V24 => {
                self.padding_size = None;
            }
        }
    } 
}

fn parse_ext_v3(stream: &mut BufStream) -> ParseResult<ExtendedHeader> {
    let size = stream.read_u32()?;

    // The extended header should be 6 or 10 bytes
    if size != 6 && size != 10 {
        error!("ID3v2.3 extended headers are 6 or 10 bytes, found {}", size);
        return Err(ParseError::MalformedData);
    }

    let flags = stream.read_u16()?;

    let mut header = ExtendedHeader {
        padding_size: Some(stream.read_u32()?),
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

    // An extended header can be at most between 6 and 15 bytes
    if !(6..=15).contains(&size) {
        error!("ID3v2.4 extended headers can only be 6 to 15 bytes long");
        return Err(ParseError::MalformedData);
    }

    // The flag count is always 1.
    if stream.read_u8()? != 1 {
        error!("ID3v2.4 extended headers must have a flag count of 1");
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
            error!("invalid is_update length");
            return Err(ParseError::MalformedData);
        }

        header.is_update = true;
    }

    // CRC-32 data.
    if flags & 0x20 != 0 {
        // CRC-32 data must be a 35-bit syncsafe integer.
        if stream.read_u8()? != 5 {
            error!("invalid CRC-32 length");
            return Err(ParseError::MalformedData);
        }

        header.crc32 = Some(syncdata::to_u35(stream.read_array()?));
    }

    // Tag restrictions. Musikr doesn't really do anything with these since according to the spec
    // they are only flags for when the tag was *encoded*, now how it should *decode*.
    if flags & 0x10 != 0 {
        // Restrictions must be 1 byte in length.
        if stream.read_u8()? != 1 {
            error!("invalid restrictions length");
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

fn render_ext_v3(header: &ExtendedHeader) -> Vec<u8> {
    // We do a bit of an efficiency hack here. Since the extended header is only 6 or 10 bytes,
    // we can pre-set the size and flags and simply modify it later.
    let mut data = vec![0, 0, 0, 6, 0, 0];

    // Since there is no padding size field in ID3v2.4's extended header, the padding size is an
    // Option, so we default to zero if its not given
    data.extend(header.padding_size.unwrap_or_default().to_be_bytes());

    // The CRC-32 data is optional. Update the size and flags if it's present
    if let Some(crc) = header.crc32 {
        data[3] = 10;
        data[4] = 0x80;
        data.extend(crc.to_be_bytes());
    }

    data
}

fn render_ext_v4(header: &ExtendedHeader) -> Vec<u8> {
    // Rendering the v4 extended header is a bit more complicated.
    let mut data = vec![0, 0, 0, 6, 1, 0];

    // The is update flag is always empty
    if header.is_update {
        data[3] += 1;
        data[5] |= 0x40;
        data.push(0);
    }

    // CRC-32, also optional
    if let Some(crc) = header.crc32 {
        data[3] += 6;
        data[5] |= 0x20;
        data.push(5);
        data.extend(syncdata::from_u35(crc));
    }

    // Restrictions
    if let Some(restrictions) = header.restrictions {
        data[3] += 2;
        data[5] |= 0x10;
        data.push(1);

        let mut bits = 0;
        bits |= (restrictions.tag_size as u8) << 6;
        bits |= (restrictions.text_encoding as u8) << 5;
        bits |= (restrictions.text_size as u8) << 3;
        bits |= (restrictions.image_encoding as u8) << 2;
        bits |= (restrictions.image_size as u8) << 1;

        data.push(bits)
    }

    data
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
    Max128Frames1Mb = 0,
    Max64Frames128Kb = 1,
    Max32Frames40Kb = 2,
    Max32Frames4Kb = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncodingRestriction {
    None = 0,
    Latin1OrUtf8 = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextSizeRestriction {
    None = 0,
    LessThan1024Chars = 1,
    LessThan128Chars = 2,
    LessThan30Chars = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageEncodingRestriction {
    None = 0,
    OnlyPngOrJpeg = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageSizeRestriction {
    None = 0,
    LessThan256x256 = 1,
    LessThan64x64 = 2,
    Exactly64x64 = 3,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::io::BufStream;

    const EXT_DATA_V3: &[u8] = b"\x00\x00\x00\x0A\x80\x00\xAB\xCD\xEF\x16\x16\x16\x16\x16";
    const EXT_DATA_V4: &[u8] = b"\x00\x00\x00\x0F\x01\x70\x00\x05\x07\x5E\x37\x5E\x16\x01\xB4";

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
        let header = ExtendedHeader::parse(&mut BufStream::new(EXT_DATA_V3), Version::V23).unwrap();

        assert_eq!(header.padding_size, Some(0xABCDEF16));
        assert_eq!(header.crc32, Some(0x16161616));
        assert!(!header.is_update);
        assert_eq!(header.restrictions, None);
    }

    #[test]
    fn parse_v4_ext_header() {
        let header = ExtendedHeader::parse(&mut BufStream::new(EXT_DATA_V4), Version::V24).unwrap();

        assert_eq!(header.padding_size, None);
        assert_eq!(header.crc32, Some(0x7BCDEF16));
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

    #[test]
    fn render_v3_ext_header() {
        let header = ExtendedHeader {
            padding_size: Some(0xABCDEF16),
            crc32: Some(0x16161616),
            ..Default::default()
        };

        assert_eq!(header.render(Version::V23), EXT_DATA_V3);
    }

    #[test]
    fn render_v4_ext_header() {
        let header = ExtendedHeader {
            crc32: Some(0x7BCDEF16),
            is_update: true,
            restrictions: Some(Restrictions {
                tag_size: TagSizeRestriction::Max32Frames40Kb,
                text_encoding: TextEncodingRestriction::Latin1OrUtf8,
                text_size: TextSizeRestriction::LessThan128Chars,
                image_encoding: ImageEncodingRestriction::OnlyPngOrJpeg,
                image_size: ImageSizeRestriction::None,
            }),
            ..Default::default()
        };

        assert_eq!(header.render(Version::V24), EXT_DATA_V4);
    }
}
