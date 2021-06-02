use crate::id3::util;

pub struct TagHeader {
    pub major: u8,
    pub minor: u8,
    pub flags: u8,
    pub tag_size: usize,
}

impl TagHeader {
    pub fn from(data: &[u8]) -> Option<TagHeader> {
        // Verify that this header has a valid ID3 Identifier
        if !data[0..3].eq(b"ID3") {
            return None;
        }

        let major = data[3];
        let minor = data[4];
        let flags = data[5];

        if major == 0xFF || minor == 0xFF {
            // Versions must be less than 0xFF
            return None;
        }

        let tag_size = util::syncsafe_decode(&data[6..10]);

        // A size of zero is invalid, as id3 tags must have at least one frame.
        if tag_size == 0 {
            return None;
        }

        Some(TagHeader {
            major,
            minor,
            flags,
            tag_size,
        })
    }

    pub fn unsynchonized(&self) -> bool {
        (self.flags & 1) == 1
    }

    pub fn has_ext_header(&self) -> bool {
        ((self.flags >> 1) & 1) == 1
    }

    pub fn experimental(&self) -> bool {
        ((self.flags >> 2) & 1) == 1
    }

    pub fn has_footer(&self) -> bool {
        ((self.flags >> 3) & 1) == 1
    }
}

pub struct ExtendedHeader {
    pub size: usize,
    pub data: Vec<u8>,
}

impl ExtendedHeader {
    pub fn from(data: &[u8]) -> Option<ExtendedHeader> {
        // We don't exactly care about parsing the extended header, but we do
        // keep it around when it's time to write new tag information
        let size = util::syncsafe_decode(&data[0..4]);

        // Validate that this header is valid.
        if size == 0 && (size + 4) > data.len() {
            return None;
        }

        let data = data[4..size].to_vec();

        return Some(ExtendedHeader { size, data });
    }
}
