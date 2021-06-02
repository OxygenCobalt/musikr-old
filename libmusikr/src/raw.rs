pub fn slice_to_size(raw: &[u8]) -> usize {
    if raw.len() < 4 {
        return 0;
    }

    (raw[0] as usize) << 24 | (raw[1] as usize) << 16 | (raw[2] as usize) << 8 | (raw[3] as usize)
}

pub fn bit_at(pos: u8, byte: u8) -> bool {
    (byte >> pos) & 1 == 1
}
