const ENCODING_ASCII: u8 = 0x00;
const ENCODING_UTF16_BOM: u8 = 0x01;
const ENCODING_UTF16_BE: u8 = 0x02;
const ENCODING_UTF8: u8 = 0x03;

pub(super) enum Encoding {
    Utf8,
    Utf16Bom,
    Utf16Be,
}

impl Encoding {
    pub fn from(flag: u8) -> Encoding {
        return match flag {
            // ASCII and UTF8 can be mapped to the same type
            ENCODING_ASCII | ENCODING_UTF8 => Encoding::Utf8,

            // UTF16 with BOM [Can be both LE or BE]
            ENCODING_UTF16_BOM => Encoding::Utf16Bom,

            // UTF16 without BOM [Always BE]
            ENCODING_UTF16_BE => Encoding::Utf16Be,

            // Malformed, just say its UTF-8 and hope for the best
            _ => Encoding::Utf8,
        };
    }

    pub fn get_nul_size(&self) -> usize {
        return match self {
            Encoding::Utf8 => 1, // UTF-8 has a one byte NUL terminator
            _ => 2,              // UTF-16 has a two-byte NUL terminator
        };
    }
}

pub(super) fn get_string(encoding: &Encoding, data: &[u8]) -> String {
    return match encoding {
        Encoding::Utf8 => String::from_utf8_lossy(data).to_string(),

        Encoding::Utf16Be => str_from_utf16be(data),

        // UTF16BOM requires us to figure out the endianness ourselves from the BOM
        Encoding::Utf16Bom => match (data[0], data[1]) {
            (0xFF, 0xFE) => str_from_utf16le(&data[2..]), // Little Endian
            (0xFE, 0xFF) => str_from_utf16be(&data[2..]), // Big Endian
            _ => str_from_utf16ne(data),                  // No BOM, use native UTF-16
        },
    };
}

pub(super) fn get_nul_string(encoding: &Encoding, data: &[u8]) -> Option<String> {
    // Find the NUL terminator for this data stream
    let mut size: usize = 0;

    if let Encoding::Utf8 = encoding {
        // Normal UTF-8 can be done one at a time
        while size < data.len() && data[size] != 0 {
            size += 1;
        }
    } else {
        // We need to parse by two bytes with UTF-16
        for chunk in data.chunks_exact(2) {
            if chunk[0] == 0 && chunk[1] == 0 {
                break;
            }

            size += 2;
        }
    }

    // Check for an empty string
    if size == 0 {
        return None;
    }

    return Some(get_string(encoding, &data[0..size]));
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

fn str_from_utf16ne(data: &[u8]) -> String {
    let result: Vec<u16> = data
        .chunks_exact(2)
        .into_iter()
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect();

    return String::from_utf16_lossy(&result.as_slice());
}
