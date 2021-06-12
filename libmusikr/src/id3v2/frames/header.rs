use crate::raw;
use crate::id3v2::syncdata;
use crate::id3v2::TagHeader;

pub struct FrameHeader {
    pub frame_id: String,
    pub frame_size: usize,
    pub flags: FrameFlags
}

impl FrameHeader {
    pub fn new(frame_id: String) -> Self {
        Self::with_flags(frame_id, FrameFlags::default())
    }

    pub fn with_flags(frame_id: String, flags: FrameFlags) -> Self {
        FrameHeader {
            frame_id,
            frame_size: 0,
            flags
        }
    }

    pub(crate) fn parse(header: &TagHeader, data: &[u8]) -> Option<Self> {
        // Frame header formats diverge quite signifigantly across ID3v2 versions,
        // so we need to handle them seperately

        match header.major {
            3 => new_header_v3(data),
            4 => new_header_v4(data),
            _ => None, // TODO: Parse ID3v2.2 headers
        }
    }

    pub fn flags(&self) -> &FrameFlags {
        &self.flags
    }
    
    pub fn flags_mut(&mut self) -> &mut FrameFlags {
        &mut self.flags
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

fn new_header_v3(data: &[u8]) -> Option<FrameHeader> {
    let frame_id = new_frame_id(&data[0..4])?;
    let frame_size = raw::to_size(&data[4..8]);

    let stat_flags = data[8];
    let format_flags = data[9];

    Some(FrameHeader {
        frame_id,
        frame_size,
        flags: FrameFlags {
            tag_should_discard: raw::bit_at(0, stat_flags),
            file_should_discard: raw::bit_at(1, stat_flags),
            read_only: raw::bit_at(2, stat_flags),
            compressed: raw::bit_at(0, format_flags),
            encrypted: raw::bit_at(1, format_flags),
            has_group: raw::bit_at(2, format_flags),
            unsync: false,
            has_data_len: false,
        }
    })
}

fn new_header_v4(data: &[u8]) -> Option<FrameHeader> {
    let frame_id = new_frame_id(&data[0..4])?;

    // ID3v2.4 sizes SHOULD Be syncsafe, but iTunes is a special little snowflake and wrote
    // old ID3v2.3 sizes instead for a time. Handle that.
    let mut frame_size = syncdata::to_size(&data[4..8]);

    if frame_size >= 0x80
        && !is_frame_id(&data[frame_size + 10..frame_size + 14])
        && data[frame_size + 10] != 0
    {
        let raw_size = raw::to_size(&data[4..8]);

        if is_frame_id(&data[raw_size + 10..raw_size + 14]) {
            frame_size = raw_size;
        }
    }

    let stat_flags = data[8];
    let format_flags = data[9];

    Some(FrameHeader {
        frame_id,
        frame_size,
        flags: FrameFlags {
            tag_should_discard: raw::bit_at(1, stat_flags),
            file_should_discard: raw::bit_at(2, stat_flags),
            read_only: raw::bit_at(3, stat_flags),
            has_group: raw::bit_at(1, format_flags),
            compressed: raw::bit_at(4, format_flags),
            encrypted: raw::bit_at(5, format_flags),
            unsync: raw::bit_at(6, format_flags),
            has_data_len: raw::bit_at(7, format_flags),
        }
    })
}

fn new_frame_id(frame_id: &[u8]) -> Option<String> {
    if !is_frame_id(frame_id) {
        return None;
    }

    String::from_utf8(frame_id.to_vec()).ok()
}

fn is_frame_id(frame_id: &[u8]) -> bool {
    for ch in frame_id {
        if !(b'A'..b'Z').contains(ch) && !(b'0'..b'9').contains(ch) {
            return false;
        }
    }

    true
}
