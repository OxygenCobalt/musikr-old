pub fn to_size(raw: &[u8]) -> usize {
    if raw.len() < 4 {
        // Below minimum size, will use loop instead
        return to_size_var(raw);
    }

    // Bit-Shifting is unrolled here for efficency
    (raw[0] as usize) << 24 | (raw[1] as usize) << 16 | (raw[2] as usize) << 8 | (raw[3] as usize)
}

fn to_size_var(raw: &[u8]) -> usize {
    let mut sum = 0;

    for i in 0..raw.len() {
        sum |= (raw[i] as usize) << ((raw.len() - i) * 8)
    }

    sum
}

pub fn bit_at(pos: u8, byte: u8) -> bool {
    (byte >> pos) & 1 == 1
}
