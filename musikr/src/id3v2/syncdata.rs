use crate::raw;

pub fn to_size(raw: &[u8]) -> usize {
    let mut sum: usize = 0;

    // Ensure that we're not going to overflow a 32-bit usize
    let len = usize::min(raw.len(), 4);
    let last = len - 1;

    for i in 0..len {
        if raw[i] >= 0x80 {
            // Not actually sync-safe, assume it may be a normal size
            return raw::to_size(raw);
        }

        sum |= (raw[i] as usize) << ((last - i) * 7);
    }

    sum
}

pub fn decode(src: &[u8]) -> Vec<u8> {
    // This is an implementation of Taglib's fast syncdata decoding algorithm.
    // https://github.com/taglib/taglib/blob/master/taglib/mpeg/id3v2/id3v2synchdata.cpp#L75
    // There may be some magic series of iterator methods we could use to do the same thing
    // here, but whatever

    let mut dest = Vec::with_capacity(src.len());
    let mut pos = 0;
    let mut total = 0;

    while pos < src.len() - 1 {
        dest.push(src[pos]);

        pos += 1;
        total += 1;

        // Roughly, the two sync guards in ID3v2 are:
        // 0xFF 0xXX -> 0xFF 0x00 0xXX where 0xXX >= 0xE0
        // 0xFF 0x00 -> 0xFF 0x00 0x00
        // Since both guards share the initial 0xFF 0x00 bytes, we can simply detect for that
        // and then skip the added 0x00.
        if src[pos - 1] == 0xFF && src[pos] == 0x00 {
            pos += 1;
        }
    }

    if pos < src.len() {
        total += 1;
        dest.push(src[pos]);
    }

    // Remove excess zeroes from the Vec that didn't end up being filled.
    dest.truncate(total);

    dest
}

#[cfg(test)]
mod tests {
    use crate::file::File;
    use std::env;

    #[test]
    fn decode_unsync_data() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/res/test/unsync.mp3";
        let mut file = File::open(&path).unwrap();
        let tag = file.id3v2().unwrap();
        let frames = tag.frames();

        assert_eq!(frames["TIT2"].to_string(), "My babe just cares for me");
        assert_eq!(frames["TPE1"].to_string(), "Nina Simone");
        assert_eq!(frames["TALB"].to_string(), "100% Jazz");
        assert_eq!(frames["TRCK"].to_string(), "03");
        assert_eq!(frames["TLEN"].to_string(), "216000");
    }
}
