/// Tag string handling.

use crate::core::io::BufStream;
use log::warn;

/// The internal representation of text encodings in musikr.
///
/// Not all tag formats will use encodings in the same way. For example, ID3v2 will give you multiple options
/// for encoding frames, but Xiph tags are only limited to UTF-8. If you want the least hassle, use the default
/// encoding of [`Encoding::Utf8`](Encoding::Utf8) if you have the choice.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Encoding {
    /// ISO-8859-1, also known as Latin1. This is used in the older tag formats like ID3v1 and ID3v2.
    /// Using this encoding is discouraged, as all Unicode text in a string will be flattened into "?"
    /// characters when written. Use [`Utf16`](Encoding::Utf16) or [`Utf8`](Encoding::Utf8) instead if possible.
    Latin1,
    /// UTF-16 with a BOM. In practice, this will be UTF-16LE with a `0xFFFE` BOM. This generally provides
    /// the best compromise between rust strings and the more antiquated tag formats, and is what other
    /// encodings map to when they are not supported.
    Utf16,
    /// UTF-16BE with no BOM. This is only used in the ID3v2.4 format. In all other cases its mapped
    /// to [`Utf16`](Encoding::Utf16).
    Utf16Be,
    /// UTF-8. This will allow the direct mapping of rust strings to tag data, but is not supported
    /// on all tag formats. If it is not supported, then it will be mapped to [`Utf16`](Encoding::Utf16).
    Utf8,
    /// UTF-16LE with no BOM. This is analogous to [`Utf16`](Encoding::Utf16) and will be written as such.
    Utf16Le,
}

impl Encoding {
    pub(crate) fn nul_size(&self) -> usize {
        match self {
            Self::Utf8 | Self::Latin1 => 1,
            _ => 2,
        }
    }
}

impl Default for Encoding {
    fn default() -> Self {
        Self::Utf8
    }
}

/// Consumes the rest of this stream and decodes it into a string according
/// to the encoding,
pub(crate) fn read(encoding: Encoding, stream: &mut BufStream) -> String {
    decode(encoding, stream.take_rest())
}

/// Searches and consumes the stream up until a NUL terminator and decodes it into a
/// string according to the encoding. The string will not include the terminator.
pub(crate) fn read_terminated(encoding: Encoding, stream: &mut BufStream) -> String {
    // Search for the NUL terminator, which is 0x00 in Latin1/UTF-8 and 0x0000 in UTF-16,
    // The string data will not include the terminator, but the amount consumed in the
    // stream will.
    let string_data = match encoding.nul_size() {
        1 => stream.search(&[0]),
        2 => stream.search(&[0, 0]),
        _ => unreachable!(),
    };

    decode(encoding, string_data)
}

/// Renders a string according to the encoding
pub(crate) fn render(encoding: Encoding, string: &str) -> Vec<u8> {
    // Currently, our implementation just does the conversions and collects them into
    // a Vec. Should be efficient enough.
    match encoding {
        Encoding::Latin1 => encode_latin1(string),
        Encoding::Utf16 => encode_utf16(string),
        Encoding::Utf16Be => encode_utf16be(string),
        Encoding::Utf8 => string.as_bytes().to_vec(),
        Encoding::Utf16Le => encode_utf16le(string),
    }
}

/// Renders a string plus a NUL terminator according to the encoding
pub(crate) fn render_terminated(encoding: Encoding, string: &str) -> Vec<u8> {
    let mut result = render(encoding, string);

    // Append the NUL terminator to the end, one byte for Latin1/UTF-8 and two bytes for UTF-16
    result.resize(result.len() + encoding.nul_size(), 0);

    result
}

fn decode(encoding: Encoding, data: &[u8]) -> String {
    // Ensure that our data has no trailing NULs. This is done for two reasons:
    // 1. For terminated strings, BufStream::search will return the string data plus the terminator,
    // meaning that it has to be trimmed.
    // 2. Despite not having to, a TON of non-terminated string data will be nul-terminated, mostly to
    // make serializing into c-strings easy. But this is rust, and these NULs only pollute the string
    // and produce unexpected behavior, so we trim them.
    let data = match encoding.nul_size() {
        1 => data.strip_suffix(&[0]).unwrap_or(data),
        2 => data.strip_suffix(&[0, 0]).unwrap_or(data),
        _ => unreachable!(),
    };

    // All our decode operations will copy into owned string, as pretty much every piece of code
    // calling this function will need to own their data.
    match encoding {
        Encoding::Latin1 => decode_latin1(data),
        Encoding::Utf16 => decode_utf16(data),
        Encoding::Utf16Be => decode_utf16be(data),
        Encoding::Utf8 => String::from_utf8_lossy(data).to_string(),
        Encoding::Utf16Le => decode_utf16le(data),
    }
}

fn decode_latin1(data: &[u8]) -> String {
    // UTF-8 expresses high bits as two bytes instead of one, so we cannot convert directly.
    // Instead, we simply reinterpret the bytes as chars to make sure the code-points line up.
    data.iter().map(|&byte| char::from(byte)).collect()
}

fn decode_utf16(data: &[u8]) -> String {
    // There must be at least 2 bytes.
    if data.len() < 2 {
        return String::new();
    }

    // UTF16 requires us to figure out the endianness ourselves from the BOM
    match (data[0], data[1]) {
        (0xFF, 0xFE) => decode_utf16le(&data[2..]), // Little Endian
        (0xFE, 0xFF) => decode_utf16be(&data[2..]), // Big Endian
        _ => {
            // No BOM, assume UTF16-LE
            warn!("could not determine UTF-16 BOM, defaulting to UTF-16BE");
            decode_utf16be(data)
        }
    }
}

fn decode_utf16be(data: &[u8]) -> String {
    String::from_utf16_lossy(
        data.chunks_exact(2)
            .into_iter()
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<u16>>()
            .as_slice(),
    )
}

fn decode_utf16le(data: &[u8]) -> String {
    String::from_utf16_lossy(
        data.chunks_exact(2)
            .into_iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<u16>>()
            .as_slice(),
    )
}

fn encode_latin1(string: &str) -> Vec<u8> {
    // All Latin1 chars line up with UTF-8 code-points, but everything else has
    // to be expressed as a ?
    string
        .chars()
        .map(|ch| if u32::from(ch) > 0xFF { b'?' } else { ch as u8 })
        .collect()
}

fn encode_utf16(string: &str) -> Vec<u8> {
    // UTF-16 requires a BOM at the beginning.
    let mut result: Vec<u8> = vec![0xFF, 0xFE];

    // For simplicity, we just write UTF16LE bytes every time.
    result.extend(encode_utf16le(string));

    result
}

fn encode_utf16be(string: &str) -> Vec<u8> {
    string
        .encode_utf16()
        .flat_map(|cp| cp.to_be_bytes())
        .collect()
}

fn encode_utf16le(string: &str) -> Vec<u8> {
    string
        .encode_utf16()
        .flat_map(|cp| cp.to_le_bytes())
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
        assert_eq!(decode(Encoding::Latin1, DATA_LATIN1), STR_LATIN1);
    }

    #[test]
    fn parse_utf16() {
        assert_eq!(decode(Encoding::Utf16, DATA_UTF16), STR_UNICODE);
    }

    #[test]
    fn parse_utf16be() {
        assert_eq!(decode(Encoding::Utf16Be, DATA_UTF16BE), STR_UNICODE);
    }

    #[test]
    fn parse_utf8() {
        assert_eq!(decode(Encoding::Utf8, DATA_UTF8), STR_UNICODE)
    }

    #[test]
    fn parse_utf16le() {
        assert_eq!(decode(Encoding::Utf16Le, &DATA_UTF16[2..]), STR_UNICODE)
    }

    #[test]
    fn render_latin1() {
        assert_eq!(render(Encoding::Latin1, STR_LATIN1), DATA_LATIN1);
    }

    #[test]
    fn render_latin1_lossy() {
        assert_eq!(render(Encoding::Latin1, STR_UNICODE), DATA_LATIN1_LOSSY);
    }

    #[test]
    fn render_utf16() {
        assert_eq!(render(Encoding::Utf16, STR_UNICODE), DATA_UTF16);
    }

    #[test]
    fn render_utf16be() {
        assert_eq!(render(Encoding::Utf16Be, STR_UNICODE), DATA_UTF16BE);
    }

    #[test]
    fn render_utf8() {
        assert_eq!(render(Encoding::Utf8, STR_UNICODE), DATA_UTF8);
    }

    #[test]
    fn render_utf16le() {
        assert_eq!(render(Encoding::Utf16Le, STR_UNICODE), &DATA_UTF16[2..]);
    }

    #[test]
    fn parse_terminated() {
        let data = b"L\xEEke \xE2 while loo\0p w\xEFth n\xF8 escap\xEA";
        let mut stream = BufStream::new(data);

        let terminated = read_terminated(Encoding::Latin1, &mut stream);
        assert_eq!(terminated, "Lîke â while loo");

        let rest = read_terminated(Encoding::Latin1, &mut stream);
        assert_eq!(rest, "p wïth nø escapê");
    }

    #[test]
    fn parse_terminated_utf16() {
        let data = b"\xFF\xFE\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\
                     \xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\
                     \x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\0\0\
                     \xFF\xFE\x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\
                     \x6e\x00\xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\
                     \xea\x00\x20\x00\x51\x25";

        let mut stream = BufStream::new(data);

        let terminated = read_terminated(Encoding::Utf16, &mut stream);
        assert_eq!(terminated, "║ Lîke â 𝕨𝕙𝕚le l𝒐𝒐");

        let rest = read_terminated(Encoding::Utf16, &mut stream);
        assert_eq!(rest, "p wïth nø escapê ║");
    }

    #[test]
    fn render_nul() {
        let out = b"\x4c\xee\x6b\x65\x20\xe2\x20\x77\x68\x69\x6c\x65\x20\x6c\x6f\x6f\
                    \x70\x20\x77\xef\x74\x68\x20\x6e\xf8\x20\x65\x73\x63\x61\x70\xea\0";

        assert_eq!(render_terminated(Encoding::Latin1, STR_LATIN1), out);
    }

    #[test]
    fn render_nul_utf16() {
        let out = b"\xFF\xFE\x51\x25\x20\x00\x4c\x00\xee\x00\x6b\x00\x65\x00\x20\x00\
                     \xe2\x00\x20\x00\x35\xd8\x68\xdd\x35\xd8\x59\xdd\x35\xd8\x5a\xdd\
                     \x6c\x00\x65\x00\x20\x00\x6c\x00\x35\xd8\x90\xdc\x35\xd8\x90\xdc\
                     \x70\x00\x20\x00\x77\x00\xef\x00\x74\x00\x68\x00\x20\x00\x6e\x00\
                     \xf8\x00\x20\x00\x65\x00\x73\x00\x63\x00\x61\x00\x70\x00\xea\x00\
                     \x20\x00\x51\x25\0\0";

        assert_eq!(render_terminated(Encoding::Utf16, STR_UNICODE), out);
    }
}
