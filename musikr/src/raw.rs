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
    (byte >> pos) & 1 == 1
}

#[cfg(test)]
mod tests {
    use crate::raw;

    #[test]
    pub fn parse_u32() {
        let data = vec![0xAB, 0xCD, 0xDE, 0xF0];

        assert_eq!(raw::to_u32(&data), 0xABCDDEF0);
    }

    #[test]
    pub fn parse_u16() {
        let data = vec![0xAB, 0xCD];

        assert_eq!(raw::to_u16(&data), 0xABCD);
    }

    #[test]
    pub fn parse_bit() {
        let data = 0b10101101;

        assert_eq!(raw::bit_at(0, data), true);
        assert_eq!(raw::bit_at(1, data), false);
        assert_eq!(raw::bit_at(2, data), true);
        assert_eq!(raw::bit_at(3, data), true);
        assert_eq!(raw::bit_at(4, data), false);
        assert_eq!(raw::bit_at(5, data), true);
        assert_eq!(raw::bit_at(6, data), false);
        assert_eq!(raw::bit_at(7, data), true);
    }
}
