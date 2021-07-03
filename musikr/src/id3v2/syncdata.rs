use crate::core::io::BufStream;

/// Takes an ID3v2 syncsafe size from `raw` and converts it to a `usize`.
pub fn to_size(raw: [u8; 4]) -> usize {
    let mut sum: usize = 0;

    // Ensure that we're not going to overflow a 32-bit usize
    for (i, &byte) in raw.iter().enumerate() {
        if byte >= 0x80 {
            // Not actually sync-safe, assume it may be a normal size
            return u32::from_be_bytes(raw) as usize;
        }

        sum |= (byte as usize) << ((3 - i) * 7);
    }

    sum
}

/// Lossily converts a 5-byte array into a u32.
pub fn to_u32(mut raw: [u8; 5]) -> u32 {
    let mut sum: u32 = 0;

    // Remove the last 5 bits of the first byte so that we don't overflow the u32.
    // The spec says that these bits shouldnt be used, so this is okay.
    raw[0] &= 0x7;

    for (i, &byte) in raw.iter().enumerate() {
        sum |= (byte as u32) << ((4 - i) * 7);
    }

    sum
}

/// Consumes a stream `src` and returns a `Vec<u8>` decoded from the ID3v2 unsynchronization scheme.
/// This is an implementation of Taglib's fast syncdata decoding algorithm. Credit goes to them.
/// https://github.com/taglib/taglib/blob/master/taglib/mpeg/id3v2/id3v2synchdata.cpp#L75
pub fn decode(src: &mut BufStream) -> Vec<u8> {
    // The end size of any decoded data will always be less than or equal to the length of
    // src, so making the initial capacity src.len() allows us to only alloc once
    let mut dest = Vec::with_capacity(src.len());
    let mut last = 0;

    while src.remaining() > 1 {
        let cur = src.read_u8().unwrap();
        dest.push(cur);

        // Roughly, the two sync guards in ID3v2 are:
        // 0xFF 0xXX -> 0xFF 0x00 0xXX where 0xXX & 0xE0 != 0
        // 0xFF 0x00 -> 0xFF 0x00 0x00
        // Since both guards share the initial 0xFF 0x00 bytes, we can simply detect for that
        // and then skip the added 0x00.
        if last == 0xFF && cur == 0x00 {
            src.skip(1).unwrap()
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
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/unsync.mp3";
        let tag = Tag::open(path).unwrap();

        assert_eq!(tag.frames["TIT2"].to_string(), "My babe just cares for me");
        assert_eq!(tag.frames["TPE1"].to_string(), "Nina Simone");
        assert_eq!(tag.frames["TALB"].to_string(), "100% Jazz");
        assert_eq!(tag.frames["TRCK"].to_string(), "03");
        assert_eq!(tag.frames["TLEN"].to_string(), "216000");
    }
}
