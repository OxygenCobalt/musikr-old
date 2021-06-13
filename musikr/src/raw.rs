pub fn to_size(raw: &[u8]) -> usize {
    to_u32(raw) as usize
}

pub fn to_u32(raw: &[u8]) -> u32 {
    if raw.len() < 4 {
        return to_u32_var(raw);
    }

    // Bitshift is unrolled here for efficency
    (raw[0] as u32) << 24 | (raw[1] as u32) << 16 | (raw[2] as u32) << 8 | (raw[3] as u32)
}

fn to_u32_var(raw: &[u8]) -> u32 {
    let mut sum = 0;

    for i in 0..raw.len() {
        sum |= (raw[i] as u32) << ((raw.len() - i) * 8)
    }

    sum
}

pub fn to_u16(raw: &[u8]) -> u16 {
    if raw.len() < 2 {
        return match raw.get(0) {
            Some(n) => *n as u16,
            None => 0,
        };
    }

    (raw[0] as u16) << 8 | raw[1] as u16
}

pub fn bit_at(pos: u8, byte: u8) -> bool {
    (byte << pos) & 0x80 == 0x80
}
