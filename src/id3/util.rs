pub fn syncsafe_decode(raw: &[u8]) -> Option<usize> {
    if raw.len() < 4  || is_not_syncsafe(raw) {
        return None;
    }

    return Some((raw[0] as usize) << 21 | 
           (raw[1] as usize) << 14 |
           (raw[2] as usize) << 7 |
           (raw[3] as usize));
}

pub fn size_decode(raw: &[u8]) -> usize {
    return (raw[0] as usize) << 24 | 
           (raw[1] as usize) << 16 |
           (raw[2] as usize) << 8 |
           (raw[3] as usize);
}

pub fn has_ext_header(flags: u8) -> bool {
    return ((flags >> 1) & 1) == 1;
}

fn is_not_syncsafe(raw: &[u8]) -> bool {
    return raw[0] >= 0x80 || raw[1] >= 0x80 || raw[2] >= 0x80 || raw[3] >= 0x80;
}
