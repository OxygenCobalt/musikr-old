pub fn slice_to_size(raw: &[u8]) -> usize {
    if raw.len() < 4 {
        return 0;
    }

    return (raw[0] as usize) << 24
        | (raw[1] as usize) << 16
        | (raw[2] as usize) << 8
        | (raw[3] as usize);
}