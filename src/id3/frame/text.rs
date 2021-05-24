use super::FrameHeader;
use super::ID3Frame;

pub enum TextFrameEncoding {
    UTF8, UTF16LE, UTF16BE
}

const ENCODING_ASCII: u8 = 0x00;
const ENCODING_UTF16_BOM: u8 = 0x01;
const ENCODING_UTF16_BE: u8 = 0x02;
const ENCODING_UTF8: u8 = 0x03;

pub struct TextFrame {
    header: FrameHeader,
    pub encoding: TextFrameEncoding,
    pub text: String
}

impl ID3Frame for TextFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }

    fn format(&self) -> String {
        return format!["{}: {}", self.header.code, self.text];
    }
}

impl TextFrame {
    pub fn from(header: FrameHeader, data: &[u8]) -> TextFrame {
        let (encoding, start) = determine_encoding(data);

        let data = &data[start..data.len()];

        let text = match encoding {
            TextFrameEncoding::UTF8 => String::from_utf8_lossy(data).to_string(),

            // We have to manually transform our bytes into UTF16 strings due to the ID3 spec
            // allowing both kinds of endianness in a tag
            TextFrameEncoding::UTF16LE => str_from_utf16le(data),
            TextFrameEncoding::UTF16BE => str_from_utf16be(data)
        };

        return TextFrame {
            header, encoding, text
        };
    }
}

fn determine_encoding(data: &[u8]) -> (TextFrameEncoding, usize) {
    return match data[0] {
        // UTF-8 and ASCII encodings can be represented with UTF-8
        ENCODING_ASCII | ENCODING_UTF8 => (TextFrameEncoding::UTF8, 1),

        // UTF16BE will not have a BOM
        ENCODING_UTF16_BE => (TextFrameEncoding::UTF16BE, 1),

        // UTF16 will have a bom, so we need to handle that
        ENCODING_UTF16_BOM => {
            let encoding = handle_bom((data[1], data[2]))
                .unwrap_or(TextFrameEncoding::UTF16LE); // Default to UTF-16LE if malformed

            (encoding, 3)
        }

        // If we have a malformed byte, we will need to become a bit more involved.
        _ => handle_malformed_encoding(data)
    }
}

fn handle_malformed_encoding(data: &[u8]) -> (TextFrameEncoding, usize) {

    // Case 1: No encoding byte, but valid UTF16 BOM
    if let Some(encoding) = handle_bom((data[0], data[1])) {
        return (encoding, 2);
    }

    // Case 2: Malformed encoding byte, but valid UTF16 BOM
    if let Some(encoding) = handle_bom((data[1], data[2])) {
        return (encoding, 3);
    }

    // No idea, just return UTF8 and move on
    return (TextFrameEncoding::UTF8, 0);
}

fn handle_bom(bom: (u8, u8)) -> Option<TextFrameEncoding> {
    return match bom {
        (0xFF, 0xFE) => Some(TextFrameEncoding::UTF16LE),
        (0xFE, 0xFF) => Some(TextFrameEncoding::UTF16BE),
        _ => None
    }
}

fn str_from_utf16le(data: &[u8]) -> String {
    let result: Vec<u16> = data.chunks_exact(2)
            .into_iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();

    return String::from_utf16_lossy(&result.as_slice());
}

fn str_from_utf16be(data: &[u8]) -> String {
    let result: Vec<u16> = data.chunks_exact(2)
            .into_iter()
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();

    return String::from_utf16_lossy(&result.as_slice());    
}
