use crate::id3v2::frames::ParseError;

const ENCODING_LATIN1: u8 = 0x00;
const ENCODING_UTF16_BOM: u8 = 0x01;
const ENCODING_UTF16_BE: u8 = 0x02;
const ENCODING_UTF8: u8 = 0x03;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Encoding {
    Latin1,
    Utf16,
    Utf16Be,
    Utf8,
    Utf16Le,
}

impl Encoding {
    pub(crate) fn new(flag: u8) -> Result<Self, ParseError> {
        match flag {
            // Latin1 [Basically ASCII but now europe exists]
            ENCODING_LATIN1 => Ok(Encoding::Latin1),

            // UTF16 with BOM [Can be both LE or BE]
            ENCODING_UTF16_BOM => Ok(Encoding::Utf16),

            // UTF16 without BOM [Always BE]
            ENCODING_UTF16_BE => Ok(Encoding::Utf16Be),

            // Utf8. Theoretically Utf8 and Latin1 could be mapped to the same enum,
            // but this preserves consistency when encoding.
            ENCODING_UTF8 => Ok(Encoding::Utf8),

            // Malformed.
            _ => Err(ParseError::InvalidEncoding),
        }
    }

    pub(crate) fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let flag = match data.get(0) {
            Some(flag) => *flag,
            None => return Err(ParseError::NotEnoughData),
        };

        Self::new(flag)
    }

    pub(crate) fn nul_size(&self) -> usize {
        match self {
            Encoding::Utf8 | Encoding::Latin1 => 1,
            _ => 2,
        }
    }
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding::Utf8
    }
}

pub(crate) fn get_string(encoding: Encoding, data: &[u8]) -> String {
    return match encoding {
        Encoding::Latin1 => str_from_latin1(data),

        // UTF16BOM requires us to figure out the endianness ourselves from the BOM
        Encoding::Utf16 => match (data[0], data[1]) {
            (0xFF, 0xFE) => str_from_utf16le(&data[2..]), // Little Endian
            (0xFE, 0xFF) => str_from_utf16be(&data[2..]), // Big Endian
            _ => str_from_utf16be(data),                  // No BOM, assume UTF16-BE
        },

        Encoding::Utf16Be => str_from_utf16be(data),

        // LE isn't part of the spec, but it's needed when a BOM needs to be re-used
        Encoding::Utf16Le => str_from_utf16le(data),

        Encoding::Utf8 => String::from_utf8_lossy(data).to_string(),
    };
}

pub(crate) struct TerminatedString {
    pub string: String,
    pub size: usize,
}

pub(crate) fn get_terminated_string(encoding: Encoding, data: &[u8]) -> TerminatedString {
    // Search for the NUL terminator, which is 0x00 in Latin1/UTF-8 and 0x0000 in UTF-16
    // The string data will not include the terminator, but the size will.
    let (string_data, size) = match encoding.nul_size() {
        1 => slice_nul_single(data),
        _ => slice_nul_double(data),
    };

    let string = get_string(encoding, string_data);

    TerminatedString { string, size }
}

fn slice_nul_single(data: &[u8]) -> (&[u8], usize) {
    let mut size = 0;

    loop {
        if size >= data.len() {
            return (&data[0..size], size);
        }

        if data[size] == 0 {
            return (&data[0..size], size + 1);
        }

        size += 1;
    }
}

fn slice_nul_double(data: &[u8]) -> (&[u8], usize) {
    let mut size = 0;

    loop {
        if size + 1 > data.len() {
            return (&data[0..size], size);
        }

        if data[size] == 0 && data[size + 1] == 0 {
            return (&data[0..size], size + 2);
        }

        size += 2;
    }
}

fn str_from_latin1(data: &[u8]) -> String {
    // UTF-8 expresses high bits as two bytes instead of one, so we cannot convert directly.
    // Instead, we simply reinterpret the bytes as chars to make sure the codepoints line up.
    data.iter().map(|&byte| byte as char).collect()
}

fn str_from_utf16le(data: &[u8]) -> String {
    let result: Vec<u16> = data
        .chunks_exact(2)
        .into_iter()
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();

    return String::from_utf16_lossy(&result.as_slice());
}

fn str_from_utf16be(data: &[u8]) -> String {
    let result: Vec<u16> = data
        .chunks_exact(2)
        .into_iter()
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();

    return String::from_utf16_lossy(&result.as_slice());
}

#[cfg(test)]
mod tests {
    use crate::id3v2::frames::string::{self, Encoding};

    #[test]
    fn parse_latin1() {
        let data = b"\x4c\xee\x6b\x65\x20\xe2\x20\x77\x68\x69\x6c\x65\x20\x6c\x6f\x6f\x70\x20\x77\xef\x74\x68\x20\x6e\xf8\x20\x65\x73\x63\x61\x70\xea";

        assert_eq!(
            string::get_string(Encoding::Latin1, data),
            "Lîke â while loop wïth nø escapê"
        )
    }

    #[test]
    fn parse_utf16() {
        let data = b"\xFF\xFE\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\x20\x00\x51\x25";

        assert_eq!(
            string::get_string(Encoding::Utf16, data),
            "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║"
        )
    }

    #[test]
    fn parse_utf16be() {
        let data = b"\x25\x51\x00\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\xe2\x00\x20\xd8\x35\xdd\x68\xd8\x35\xdd\x59\xd8\x35\xdd\x5a\x00\x6c\x00\x65\x00\x20\x00\x6c\xd8\x35\xdc\x90\xd8\x35\xdc\x90\x00\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\x20\x25\x51";

        assert_eq!(
            string::get_string(Encoding::Utf16Be, data),
            "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║"
        )
    }

    #[test]
    fn parse_utf8() {
        let data = b"\xe2\x95\x91\x20\x4c\xc3\xae\x6b\x65\x20\xc3\xa2\x20\xf0\x9d\x95\xa8\xf0\x9d\x95\x99\xf0\x9d\x95\x9a\x6c\x65\x20\x6c\xf0\x9d\x92\x90\xf0\x9d\x92\x90\x70\x20\x77\xc3\xaf\x74\x68\x20\x6e\xc3\xb8\x20\x65\x73\x63\x61\x70\xc3\xaa\x20\xe2\x95\x91";

        assert_eq!(
            string::get_string(Encoding::Utf8, data),
            "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║"
        )
    }

    #[test]
    fn parse_utf16le() {
        let data = b"\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\x20\x00\x51\x25";

        assert_eq!(
            string::get_string(Encoding::Utf16Le, data),
            "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║"
        )
    }
}
