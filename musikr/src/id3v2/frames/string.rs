use crate::id3v2::frames::ParseError;

const ENCODING_LATIN1: u8 = 0x00;
const ENCODING_UTF16: u8 = 0x01;
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
            ENCODING_UTF16 => Ok(Encoding::Utf16),

            // UTF16 without BOM [Always BE]
            ENCODING_UTF16_BE => Ok(Encoding::Utf16Be),

            // Utf8, the only good one
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

    pub(crate) fn map_id3v2(&self, major: u8) -> Encoding {
        match self {
            // Utf16Be and Utf8 are only supported in ID3v2.4, map to UTF-16 on
            // older versions.
            Encoding::Utf16Be | Encoding::Utf8 if major <= 3 => Encoding::Utf16,

            // UTF-16LE is not part of the spec and will be mapped to UTF-16
            // no matter what.
            Encoding::Utf16Le => Encoding::Utf16,

            _ => *self,
        }
    }

    pub(crate) fn render(&self) -> u8 {
        match self {
            Encoding::Latin1 => ENCODING_LATIN1,
            Encoding::Utf16 => ENCODING_UTF16,
            Encoding::Utf16Be => ENCODING_UTF16_BE,
            Encoding::Utf8 => ENCODING_UTF8,
            Encoding::Utf16Le => ENCODING_UTF16,
        }
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

        Encoding::Utf8 => String::from_utf8_lossy(data).to_string(),

        // LE isn't part of the spec, but it's needed when a BOM needs to be re-used
        Encoding::Utf16Le => str_from_utf16le(data),
    };
}

pub(crate) struct TerminatedString {
    pub string: String,
    pub size: usize,
}

pub(crate) fn get_terminated(encoding: Encoding, data: &[u8]) -> TerminatedString {
    // Search for the NUL terminator, which is 0x00 in Latin1/UTF-8 and 0x0000 in UTF-16
    // The string data will not include the terminator, but the size will.
    let (string_data, size) = match encoding.nul_size() {
        1 => slice_nul_single(data),
        _ => slice_nul_double(data),
    };

    let string = get_string(encoding, string_data);

    TerminatedString { string, size }
}

pub(crate) fn render_string(encoding: Encoding, string: &str) -> Vec<u8> {
    // Aside from UTF-8, all string formats have to be rendered in special ways.
    // All these conversions will result in a copy, but this is intended.
    match encoding {
        Encoding::Latin1 => str_render_latin1(string),
        Encoding::Utf16 => str_render_utf16(string),
        Encoding::Utf16Be => str_render_utf16be(string),
        Encoding::Utf8 => string.as_bytes().to_vec(),
        Encoding::Utf16Le => str_render_utf16le(string),
    }
}

pub(crate) fn render_terminated(encoding: Encoding, string: &str) -> Vec<u8> {
    let mut result = render_string(encoding, string);

    // Append the NUL terminator to the end, one byte for Latin1/UTF-8 and two bytes for UTF-16
    result.resize(result.len() + encoding.nul_size(), 0);

    result
}

fn slice_nul_single(data: &[u8]) -> (&[u8], usize) {
    let mut size = 0;

    loop {
        if size >= data.len() {
            // No NUL terminator, return the full slice and it's length.
            return (data, size);
        }

        if data[size] == 0 {
            // NUL terminator, return the sliced portion and the size plus the NUL
            return (&data[0..size], size + 1);
        }

        size += 1;
    }
}

fn slice_nul_double(data: &[u8]) -> (&[u8], usize) {
    let mut size = 0;

    loop {
        if size + 1 > data.len() {
            // No NUL terminator, return the slice up to the last full
            // chunk and its length
            return (&data[0..size], size);
        }

        if data[size] == 0 && data[size + 1] == 0 {
            // NUL terminator, return the sliced portion and the
            // size plus the two NUL bytes
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

fn str_from_utf16be(data: &[u8]) -> String {
    String::from_utf16_lossy(
        data.chunks_exact(2)
            .into_iter()
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<u16>>()
            .as_slice(),
    )
}

fn str_from_utf16le(data: &[u8]) -> String {
    String::from_utf16_lossy(
        data.chunks_exact(2)
            .into_iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<u16>>()
            .as_slice(),
    )
}

fn str_render_latin1(string: &str) -> Vec<u8> {
    // All Latin1 chars line up with UTF-8 codepoints, but
    // everything else has to be expressed as a ?
    string
        .chars()
        .map(|ch| if ch as u64 > 0xFF { b'?' } else { ch as u8 })
        .collect()
}

fn str_render_utf16(string: &str) -> Vec<u8> {
    // When encoding UTF16, we have a BOM at the beginning.
    let mut result: Vec<u8> = vec![0xFF, 0xFE];

    result.extend(string.encode_utf16().map(|cp| cp.to_le_bytes()).flatten());

    result
}

fn str_render_utf16be(string: &str) -> Vec<u8> {
    string
        .encode_utf16()
        .map(|cp| cp.to_be_bytes())
        .flatten()
        .collect()
}

fn str_render_utf16le(string: &str) -> Vec<u8> {
    string
        .encode_utf16()
        .map(|cp| cp.to_le_bytes())
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STR_LATIN1: &str = "Lîke â while loop wïth nø escapê";
    const STR_UNICODE: &str = "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐p wïth nø escapê ║";

    const DATA_LATIN1: &[u8] = b"L\xEEke \xE2 while loop w\xEFth n\xF8 escap\xEA";
    const DATA_LATIN1_LOSSY: &[u8] = b"? L\xEEke \xE2 ???le l??p w\xEFth n\xF8 escap\xEA ?";

    const DATA_UTF16: &[u8] = b"\xFF\xFE\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\
                                \xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\
                                \x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\
                                \x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\
                                \xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\
                                \x20\x00\x51\x25";

    const DATA_UTF16BE: &[u8] = b"\x25\x51\x00\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\xe2\
                                  \x00\x20\xd8\x35\xdd\x68\xd8\x35\xdd\x59\xd8\x35\xdd\x5a\x00\x6c\
                                  \x00\x65\x00\x20\x00\x6c\xd8\x35\xdc\x90\xd8\x35\xdc\x90\x00\x70\
                                  \x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\xf8\
                                  \x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\x20\
                                  \x25\x51";

    const DATA_UTF8: &[u8] = b"\xe2\x95\x91\x20\x4c\xc3\xae\x6b\x65\x20\xc3\xa2\x20\xf0\x9d\x95\
                               \xa8\xf0\x9d\x95\x99\xf0\x9d\x95\x9a\x6c\x65\x20\x6c\xf0\x9d\x92\
                               \x90\xf0\x9d\x92\x90\x70\x20\x77\xc3\xaf\x74\x68\x20\x6e\xc3\xb8\
                               \x20\x65\x73\x63\x61\x70\xc3\xaa\x20\xe2\x95\x91";

    #[test]
    fn parse_latin1() {
        assert_eq!(get_string(Encoding::Latin1, DATA_LATIN1), STR_LATIN1)
    }

    #[test]
    fn parse_utf16() {
        assert_eq!(get_string(Encoding::Utf16, DATA_UTF16), STR_UNICODE)
    }

    #[test]
    fn parse_utf16be() {
        assert_eq!(get_string(Encoding::Utf16Be, DATA_UTF16BE), STR_UNICODE)
    }

    #[test]
    fn parse_utf8() {
        assert_eq!(get_string(Encoding::Utf8, DATA_UTF8), STR_UNICODE)
    }

    #[test]
    fn parse_utf16le() {
        assert_eq!(get_string(Encoding::Utf16Le, &DATA_UTF16[2..]), STR_UNICODE)
    }

    #[test]
    fn render_latin1() {
        assert_eq!(render_string(Encoding::Latin1, STR_LATIN1), DATA_LATIN1);
    }

    #[test]
    fn render_latin1_lossy() {
        assert_eq!(
            render_string(Encoding::Latin1, STR_UNICODE),
            DATA_LATIN1_LOSSY
        );
    }

    #[test]
    fn render_utf16() {
        assert_eq!(render_string(Encoding::Utf16, STR_UNICODE), DATA_UTF16);
    }

    #[test]
    fn render_utf16be() {
        assert_eq!(render_string(Encoding::Utf16Be, STR_UNICODE), DATA_UTF16BE);
    }

    #[test]
    fn render_utf8() {
        assert_eq!(render_string(Encoding::Utf8, STR_UNICODE), DATA_UTF8);
    }

    #[test]
    fn render_utf16le() {
        assert_eq!(
            render_string(Encoding::Utf16Le, STR_UNICODE),
            &DATA_UTF16[2..]
        );
    }

    #[test]
    fn parse_nul_single() {
        let data = b"L\xEEke \xE2 while loo\0p w\xEFth n\xF8 escap\xEA";

        let terminated = get_terminated(Encoding::Latin1, data);

        assert_eq!(terminated.size, 17);
        assert_eq!(terminated.string, "Lîke â while loo");

        let rest = get_terminated(Encoding::Latin1, &data[terminated.size..]);

        assert_eq!(rest.size, 16);
        assert_eq!(rest.string, "p wïth nø escapê");
    }

    #[test]
    fn parse_nul_double() {
        let data = b"\xFF\xFE\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\
                     \xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\
                     \x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\0\0\
                     \xFF\xFE\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\
                     \x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\
                     \xea\x00\x20\x00\x51\x25";

        let terminated = get_terminated(Encoding::Utf16, data);

        assert_eq!(terminated.size, 50);
        assert_eq!(terminated.string, "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐");

        let rest = get_terminated(Encoding::Utf16, &data[terminated.size..]);

        assert_eq!(rest.size, 38);
        assert_eq!(rest.string, "p wïth nø escapê ║");
    }

    #[test]
    fn render_nul_single() {
        let out = b"\x4c\xee\x6b\x65\x20\xe2\x20\x77\x68\x69\x6c\x65\x20\x6c\x6f\x6f\
                    \x70\x20\x77\xef\x74\x68\x20\x6e\xf8\x20\x65\x73\x63\x61\x70\xea\0";

        assert_eq!(render_terminated(Encoding::Latin1, STR_LATIN1), out);
    }

    #[test]
    fn render_nul_double() {
        let out = b"\xFF\xFE\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\
                     \xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\
                     \x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\
                     \x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\
                     \xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\
                     \x20\x00\x51\x25\0\0";

        assert_eq!(render_terminated(Encoding::Utf16, STR_UNICODE), out);
    }

    #[test]
    fn render_id3v2_encoding() {
        assert_eq!(Encoding::Latin1.map_id3v2(4).render(), 0x00);
        assert_eq!(Encoding::Utf16.map_id3v2(4).render(), 0x01);
        assert_eq!(Encoding::Utf16Be.map_id3v2(4).render(), 0x02);
        assert_eq!(Encoding::Utf8.map_id3v2(4).render(), 0x03);

        // Test that encoding flattening works
        assert_eq!(Encoding::Utf16Be.map_id3v2(3).render(), 0x01);
        assert_eq!(Encoding::Utf8.map_id3v2(3).render(), 0x01);
        assert_eq!(Encoding::Utf16Le.map_id3v2(3).render(), 0x01);
    }
}
