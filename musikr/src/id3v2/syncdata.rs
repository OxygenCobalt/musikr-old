use crate::core::io::BufStream;

/// Converts a 28-bit syncsafe integer to a `u32`.
pub fn to_u28(raw: [u8; 4]) -> u32 {
    let mut sum = 0;

    for (i, &byte) in raw.iter().enumerate() {
        if byte >= 0x80 {
            // Not actually sync-safe, assume it may be a normal size
            return u32::from_be_bytes(raw);
        }

        sum |= u32::from(byte) << ((3 - i) * 7);
    }

    sum
}

/// Lossily converts a 35-byte syncsafe integer into a u32.
pub fn to_u35(mut raw: [u8; 5]) -> u32 {
    let mut sum: u32 = 0;

    // Remove the last 5 bits of the first byte so that we don't overflow the u32.
    // The spec says that these bits shouldn't be used, so this is okay.
    raw[0] &= 0x7;

    for (i, &byte) in raw.iter().enumerate() {
        sum |= u32::from(byte) << ((4 - i) * 7);
    }

    sum
}

/// Converts a u32 into a 28-bit syncsafe integer.
pub fn from_u28(num: u32) -> [u8; 4] {
    let mut result = [0; 4];

    for (i, byte) in result.iter_mut().enumerate() {
        *byte = ((num >> ((3 - i) * 7)) & 0x7f) as u8;
    }

    result
}

/// Converts a u32 into a 35-bit syncsafe integer.
pub fn from_u35(num: u32) -> [u8; 5] {
    let mut result = [0; 5];

    for (i, byte) in result.iter_mut().enumerate() {
        *byte = ((num >> ((4 - i) * 7)) & 0x7f) as u8;
    }

    result
}

/// Consumes a stream `src` and returns a `Vec<u8>` decoded from the ID3v2 synchronization scheme.
/// This is an implementation of Taglib's fast syncdata decoding algorithm. Credit goes to them.
/// https://github.com/taglib/taglib/blob/master/taglib/mpeg/id3v2/id3v2synchdata.cpp#L75
pub fn decode(src: &mut BufStream) -> Vec<u8> {
    // The end size of any decoded data will always be less than or equal to the length of
    // src, so making the initial capacity src.len() allows us to only alloc once
    let mut dest = Vec::with_capacity(src.len());
    let mut last = 0;

    while src.remaining() > 1 {
        let cur = src.read_u8().unwrap();

        // Roughly, the two sync guards in ID3v2 are:
        // 0xFF 0xXX -> 0xFF 0x00 0xXX where 0xXX & 0xE0 != 0
        // 0xFF 0x00 -> 0xFF 0x00 0x00
        // Since both guards share the initial 0xFF 0x00 bytes, we can simply detect for that
        // and then skip the added 0x00.
        if !(last == 0xFF && cur == 0x00) {
            dest.push(cur);
        }

        last = cur;
    }

    // Since we have to look ahead, we'll sometimes need to add a lone u8 that wasnt able
    // to be added initially.
    if src.remaining() == 1 {
        dest.push(src.read_u8().unwrap());
    }

    dest.shrink_to_fit();

    dest
}

#[cfg(test)]
mod tests {
    use crate::id3v2::Tag;
    use std::env;

    #[test]
    fn decode_unsync_data() {
        // Instead of directly using syncdata::decode, its nicer to have an authentic file
        // to detect any subtle changes that might occur from bad syncdata parsing.

        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/unsync.mp3";
        let tag = Tag::open(path).unwrap();

        assert_eq!(tag.frames["TIT2"].to_string(), "My babe just cares for me");
        assert_eq!(tag.frames["TPE1"].to_string(), "Nina Simone");
        assert_eq!(tag.frames["TALB"].to_string(), "100% Jazz");
        assert_eq!(tag.frames["TRCK"].to_string(), "03");
        assert_eq!(tag.frames["TLEN"].to_string(), "216000");
    }
}
