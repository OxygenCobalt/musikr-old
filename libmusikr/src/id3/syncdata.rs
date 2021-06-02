use crate::raw;

pub fn to_size(raw: &[u8]) -> usize {
    let mut sum: usize = 0;

    // Ensure that we're not going to overflow a 32-bit usize
    let len = if raw.len() > 4 {
        4
    } else {
        raw.len()
    };

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
