use crate::core::io::BufStream;
use crate::id3v2::{ParseError, ParseResult};
use crate::string::Encoding;

const FLAG_LATIN1: u8 = 0x00;
const FLAG_UTF16: u8 = 0x01;
const FLAG_UTF16BE: u8 = 0x02;
const FLAG_UTF8: u8 = 0x03;

pub fn parse(stream: &mut BufStream) -> ParseResult<Encoding> {
    match stream.read_u8()? {
        // Latin1 [Basically ASCII but now europe exists]
        FLAG_LATIN1 => Ok(Encoding::Latin1),

        // UTF16 with BOM [Can be both LE or BE]
        FLAG_UTF16 => Ok(Encoding::Utf16),

        // UTF16 without BOM [Always BE]
        FLAG_UTF16BE => Ok(Encoding::Utf16Be),

        // Utf8, the only good one that I don't have to make shims for
        FLAG_UTF8 => Ok(Encoding::Utf8),

        // Malformed.
        _ => Err(ParseError::MalformedData),
    }
}

pub fn check(enc: Encoding, major: u8) -> Encoding {
    match enc {
        // Utf16Be and Utf8 are only supported in ID3v2.4, map to UTF-16 on
        // older versions.
        Encoding::Utf16Be | Encoding::Utf8 if major < 4 => Encoding::Utf16,

        // UTF-16LE is not part of the spec and will be mapped to UTF-16
        // no matter what.
        Encoding::Utf16Le => Encoding::Utf16,

        _ => enc,
    }
}

pub fn render(enc: Encoding) -> u8 {
    match enc {
        Encoding::Latin1 => FLAG_LATIN1,
        Encoding::Utf16 => FLAG_UTF16,
        Encoding::Utf16Be => FLAG_UTF16BE,
        Encoding::Utf8 => FLAG_UTF8,
        Encoding::Utf16Le => FLAG_UTF16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_id3v2_encoding() {
        assert_eq!(render(Encoding::Latin1), 0x00);
        assert_eq!(render(Encoding::Utf16), 0x01);
        assert_eq!(render(Encoding::Utf16Be), 0x02);
        assert_eq!(render(Encoding::Utf8), 0x03);
        assert_eq!(render(Encoding::Utf16Le), 0x01);
    }

    #[test]
    fn check_id3v2_encoding() {
        assert_eq!(check(Encoding::Utf16Le, 4), Encoding::Utf16);
        assert_eq!(check(Encoding::Utf16Be, 3), Encoding::Utf16);
        assert_eq!(check(Encoding::Utf8, 3), Encoding::Utf16);
    }
}
