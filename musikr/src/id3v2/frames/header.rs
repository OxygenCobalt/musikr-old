use crate::id3v2::{syncdata, ParseError};
use crate::raw;

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
        // It's generally better to panic here as passing a malformed ID is usually programmer error.
        if frame_id.len() > 4 || !is_frame_id(frame_id.as_bytes()) {
            panic!("A Frame ID must be exactly four valid uppercase ASCII characters or numbers.")
        }

        let frame_id = frame_id.to_string();

        FrameHeader {
            frame_id,
            frame_size: 0,
            flags,
        }
    }

    pub(crate) fn parse(major_version: u8, data: &[u8]) -> Result<Self, ParseError> {
        // Frame data must be at least 10 bytes to parse a header.
        if data.len() < 10 {
            return Err(ParseError::NotEnoughData);
        }

        // Frame header formats diverge quite signifigantly across ID3v2 versions,
        // so we need to handle them seperately
        match major_version {
            3 => new_header_v3(data),
            4 => new_header_v4(data),
            _ => Err(ParseError::Unsupported),
        }
    }

    pub fn id(&self) -> &String {
        &self.frame_id
    }

    pub fn size(&self) -> usize {
        self.frame_size
    }

    pub(crate) fn size_mut(&mut self) -> &mut usize {
        &mut self.frame_size
    }

    pub fn flags(&self) -> &FrameFlags {
        &self.flags
    }

    pub fn flags_mut(&mut self) -> &mut FrameFlags {
        &mut self.flags
    }

    pub(crate) fn set_size(&mut self, size: usize) {
        self.frame_size = size;
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

fn new_header_v3(data: &[u8]) -> Result<FrameHeader, ParseError> {
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

fn new_header_v4(data: &[u8]) -> Result<FrameHeader, ParseError> {
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
    use crate::id3v2::frames::FrameHeader;

    #[test]
    fn parse_v3_frame_header() {
        let data = b"TXXX\x00\x0A\x71\x7B\xA0\x40";
        let header = FrameHeader::parse(3, &data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), "TXXX");
        assert_eq!(header.size(), 684411);

        assert_eq!(flags.tag_should_discard, true);
        assert_eq!(flags.file_should_discard, false);
        assert_eq!(flags.read_only, true);

        assert_eq!(flags.compressed, false);
        assert_eq!(flags.encrypted, true);
        assert_eq!(flags.has_group, false);
    }

    #[test]
    fn parse_v4_frame_header() {
        let data = b"TXXX\x00\x34\x10\x2A\x50\x4B";
        let header = FrameHeader::parse(4, &data[..]).unwrap();
        let flags = header.flags();

        assert_eq!(header.id(), "TXXX");
        assert_eq!(header.size(), 854058);

        assert_eq!(flags.tag_should_discard, true);
        assert_eq!(flags.file_should_discard, false);
        assert_eq!(flags.read_only, true);

        assert_eq!(flags.has_group, true);
        assert_eq!(flags.compressed, true);
        assert_eq!(flags.encrypted, false);
        assert_eq!(flags.unsync, true);
        assert_eq!(flags.has_data_len, true);
    }
}

