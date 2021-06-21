use crate::err::{ParseError, ParseResult};
use crate::id3v2::syncdata;
use crate::raw;

pub(crate) const ID_HEADER: &[u8] = b"ID3";

pub struct TagHeader {
    major: u8,
    minor: u8,
    tag_size: usize,
    flags: TagFlags,
}

impl TagHeader {
    pub(crate) fn parse(data: &[u8]) -> ParseResult<Self> {
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

impl TagFlags {
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
    pub(crate) fn parse(major: u8, data: &[u8]) -> ParseResult<Self> {
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

pub struct FrameHeader {
    frame_id: String,
    frame_size: usize,
    flags: FrameFlags,
}

impl FrameHeader {
    pub fn new(frame_id: &str) -> Self {
        Self::with_flags(frame_id, FrameFlags::default())
    }

    pub fn with_flags(frame_id: &str, flags: FrameFlags) -> Self {
        if frame_id.len() > 4 || !is_frame_id(frame_id.as_bytes()) {
            // It's generally better to panic here as passing a malformed ID is usually programmer error.
            panic!("A Frame ID must be exactly four valid uppercase ASCII characters or numbers.")
        }

        let frame_id = frame_id.to_string();

        FrameHeader {
            frame_id,
            frame_size: 0,
            flags,
        }
    }

    pub(crate) fn parse(major_version: u8, data: &[u8]) -> ParseResult<Self> {
        // Frame data must be at least 10 bytes to parse a header.
        if data.len() < 10 {
            return Err(ParseError::NotEnoughData);
        }

        // Frame header formats diverge quite signifigantly across ID3v2 versions,
        // so we need to handle them seperately
        match major_version {
            3 => parse_frame_header_v3(data),
            4 => parse_frame_header_v4(data),
            _ => Err(ParseError::Unsupported),
        }
    }

    pub fn id(&self) -> &String {
        &self.frame_id
    }

    pub fn size(&self) -> usize {
        self.frame_size
    }

    pub fn flags(&self) -> &FrameFlags {
        &self.flags
    }

    pub fn flags_mut(&mut self) -> &mut FrameFlags {
        &mut self.flags
    }

    pub(crate) fn size_mut(&mut self) -> &mut usize {
        &mut self.frame_size
    }
}

pub struct FrameFlags {
    pub tag_should_discard: bool,
    pub file_should_discard: bool,
    pub read_only: bool,
    pub has_group: bool,
    pub compressed: bool,
    pub encrypted: bool,
    pub unsync: bool,
    pub has_data_len: bool,
}

impl Default for FrameFlags {
    fn default() -> Self {
        FrameFlags {
            tag_should_discard: false,
            file_should_discard: false,
            read_only: false,
            has_group: false,
            compressed: false,
            encrypted: false,
            unsync: false,
            has_data_len: false,
        }
    }
}

fn parse_frame_header_v3(data: &[u8]) -> Result<FrameHeader, ParseError> {
    let frame_id = new_frame_id(&data[0..4])?;
    let frame_size = raw::to_size(&data[4..8]);

    let stat_flags = data[8];
    let format_flags = data[9];

    Ok(FrameHeader {
        frame_id,
        frame_size,
        flags: FrameFlags {
            tag_should_discard: raw::bit_at(7, stat_flags),
            file_should_discard: raw::bit_at(6, stat_flags),
            read_only: raw::bit_at(5, stat_flags),
            compressed: raw::bit_at(7, format_flags),
            encrypted: raw::bit_at(6, format_flags),
            has_group: raw::bit_at(5, format_flags),
            unsync: false,
            has_data_len: false,
        },
    })
}

fn parse_frame_header_v4(data: &[u8]) -> Result<FrameHeader, ParseError> {
    let frame_id = new_frame_id(&data[0..4])?;

    // ID3v2.4 sizes SHOULD Be syncsafe, but iTunes is a special little snowflake and wrote
    // old ID3v2.3 sizes instead for a time. Handle that.
    let mut frame_size = syncdata::to_size(&data[4..8]);

    if frame_size >= 0x80 {
        frame_size = handle_itunes_v4_size(frame_size, data);
    }

    let stat_flags = data[8];
    let format_flags = data[9];

    Ok(FrameHeader {
        frame_id,
        frame_size,
        flags: FrameFlags {
            tag_should_discard: raw::bit_at(6, stat_flags),
            file_should_discard: raw::bit_at(5, stat_flags),
            read_only: raw::bit_at(4, stat_flags),
            has_group: raw::bit_at(6, format_flags),
            compressed: raw::bit_at(3, format_flags),
            encrypted: raw::bit_at(2, format_flags),
            unsync: raw::bit_at(1, format_flags),
            has_data_len: raw::bit_at(0, format_flags),
        },
    })
}

fn handle_itunes_v4_size(sync_size: usize, data: &[u8]) -> usize {
    let next_id_start = sync_size + 10;
    let next_id_end = sync_size + 14;
    let next_id = next_id_start..next_id_end;

    // Ignore truncated data and padding
    if data.len() < next_id_end || data[next_id_start] == 0 {
        return sync_size;
    }

    if !is_frame_id(&data[next_id]) {
        // If the raw size leads us to the next frame where the "syncsafe"
        // size wouldn't, we will use that size instead.
        let raw_size = raw::to_size(&data[4..8]);

        if is_frame_id(&data[raw_size + 10..raw_size + 14]) {
            return raw_size;
        }
    }

    sync_size
}

fn new_frame_id(frame_id: &[u8]) -> Result<String, ParseError> {
    if !is_frame_id(frame_id) {
        return Err(ParseError::InvalidData);
    }

    String::from_utf8(frame_id.to_vec()).map_err(|_e| ParseError::InvalidData)
}

fn is_frame_id(frame_id: &[u8]) -> bool {
    for ch in frame_id {
        if !(b'A'..b'Z').contains(ch) && !(b'0'..b'9').contains(ch) {
            return false;
        }
    }

    true
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

    use crate::file::File;
    use std::env;

    #[test]
    fn parse_v3_frame_header() {
        let data = b"TXXX\x00\x0A\x71\x7B\xA0\x40";
        let header = FrameHeader::parse(3, &data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), "TXXX");
        assert_eq!(header.size(), 684411);

        assert!(flags.tag_should_discard);
        assert!(!flags.file_should_discard);
        assert!(flags.read_only);

        assert!(!flags.compressed);
        assert!(flags.encrypted);
        assert!(!flags.has_group);
    }

    #[test]
    fn parse_v4_frame_header() {
        let data = b"TXXX\x00\x34\x10\x2A\x50\x4B";
        let header = FrameHeader::parse(4, &data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), "TXXX");
        assert_eq!(header.size(), 854058);

        assert!(flags.tag_should_discard);
        assert!(!flags.file_should_discard);
        assert!(flags.read_only);

        assert!(flags.has_group);
        assert!(flags.compressed);
        assert!(!flags.encrypted);
        assert!(flags.unsync);
        assert!(flags.has_data_len);
    }

    #[test]
    fn handle_itunes_frame_sizes() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/itunes_sizes.mp3";
        let mut file = File::open(&path).unwrap();
        let tag = file.id3v2().unwrap();
        let frames = tag.frames();

        assert_eq!(frames["TIT2"].to_string(), "Sunshine Superman");
        assert_eq!(frames["TPE1"].to_string(), "Donovan");
        assert_eq!(frames["TALB"].to_string(), "Sunshine Superman");
        assert_eq!(frames["TRCK"].to_string(), "1");
    }
}
