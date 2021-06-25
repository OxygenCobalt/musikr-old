use std::convert::TryInto;

#[inline(always)]
pub fn to_size(raw: &[u8]) -> usize {
    u32::from_be_bytes(raw.try_into().unwrap()) as usize
}

#[inline(always)]
pub fn bit_at(pos: u8, byte: u8) -> bool {
    (byte >> pos) & 1 == 1
}
