use std::convert::TryInto;

pub fn to_size(raw: &[u8]) -> usize {
    to_u32(raw) as usize
}

pub fn to_u64(raw: &[u8]) -> u64 {
    u64::from_be_bytes(slice_to_arr(raw))
}

pub fn to_u32(raw: &[u8]) -> u32 {
    u32::from_be_bytes(slice_to_arr(raw))
}

pub fn to_u16(raw: &[u8]) -> u16 {
    u16::from_be_bytes(slice_to_arr(raw))
}

pub fn slice_to_arr<const N: usize>(raw: &[u8]) -> [u8; N] {
    match raw.try_into() {
        Ok(arr) => arr,
        Err(_) => {
            // For invalid slices, just create an array of N and fill it with the slice,
            // leaving zeroes for bytes that cant be filled.
            let mut arr = [0; N];

            for i in 0..usize::min(N, raw.len()) {
                arr[N - i - 1] = raw[raw.len() - i - 1];
            }

            arr
        }
    }
}

pub fn bit_at(pos: u8, byte: u8) -> bool {
    (byte >> pos) & 1 == 1
}

#[cfg(test)]
mod tests {
    use crate::raw;

    #[test]
    pub fn parse_u64() {
        let data = b"\x12\x34\x56\x78\x90\xAB\xCD\xEF";
        assert_eq!(raw::to_u64(&data[..]), 0x1234567890ABCDEF);
    }

    #[test]
    pub fn parse_u32() {
        let data = b"\xAB\xCD\xEF\x16";
        assert_eq!(raw::to_u32(&data[..]), 0xABCDEF16);
    }

    #[test]
    pub fn parse_u16() {
        let data = b"\xAB\xCD";
        assert_eq!(raw::to_u16(&data[..]), 0xABCD);
    }

    #[test]
    pub fn parse_weird_ints() {
        let too_much = b"\xAB\xCD\xEF\x16\x16";
        let too_little = b"\xAB\xCD\xEF";

        assert_eq!(raw::to_u32(&too_much[..]), 0xCDEF1616);
        assert_eq!(raw::to_u32(&too_little[..]), 0xABCDEF);
    }

    #[test]
    pub fn parse_bit() {
        let data = 0b10101101;

        assert!(raw::bit_at(0, data));
        assert!(!raw::bit_at(1, data));
        assert!(raw::bit_at(2, data));
        assert!(raw::bit_at(3, data));
        assert!(!raw::bit_at(4, data));
        assert!(raw::bit_at(5, data));
        assert!(!raw::bit_at(6, data));
        assert!(raw::bit_at(7, data));
    }
}
