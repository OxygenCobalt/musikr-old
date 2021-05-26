pub enum ID3Encoding {
    UTF8,
    UTF16BOM,
    UTF16BE,
}

const ENCODING_ASCII: u8 = 0x00;
const ENCODING_UTF16_BOM: u8 = 0x01;
const ENCODING_UTF16_BE: u8 = 0x02;
const ENCODING_UTF8: u8 = 0x03;

pub fn get_encoding(flag: u8) -> ID3Encoding {
    return match flag {
        // ASCII and UTF8 can be mapped to the same type
        ENCODING_ASCII | ENCODING_UTF8 => ID3Encoding::UTF8,

        // UTF16 with BOM [Can be both LE or BE]
        ENCODING_UTF16_BOM => ID3Encoding::UTF16BOM,

        // UTF16 without BOM [Always BE]
        ENCODING_UTF16_BE => ID3Encoding::UTF16BE,

        // Malformed, just say its UTF-8 and hope for the best
        _ => ID3Encoding::UTF8,
    };
}

pub fn get_string(encoding: &ID3Encoding, data: &[u8]) -> String {
    return match encoding {
        ID3Encoding::UTF8 => String::from_utf8_lossy(data).to_string(),

        ID3Encoding::UTF16BE => str_from_utf16be(data),

        // UTF16BOM requires us to figure out the endianness ourselves from the BOM
        ID3Encoding::UTF16BOM => match (data[0], data[1]) {
            (0xFF, 0xFE) => str_from_utf16le(&data[2..]), // Little Endian
            (0xFE, 0xFF) => str_from_utf16be(&data[2..]), // Big Endian
            _ => str_from_utf16ne(data),                  // No BOM, use native UTF-16
        },
    };
}

pub fn get_nulstring(encoding: &ID3Encoding, data: &[u8]) -> Option<String> {
    // Find the NUL terminator for this data stream

    let mut size: usize = 0;

    if let ID3Encoding::UTF8 = encoding {
        // Normal UTF-8 can be done one at a time
        while data[size] != 0 {
            size += 1;
        }
    } else {
        // Otherwise its UTF-16 and we need to parse by two bytes instead
        for chunk in data.chunks_exact(2) {
            if chunk[0] == 0x00 && chunk[1] == 0x00 {
                break;
            }

            size += 2;
        }
    }

    // If the data starts with a NUL terminator, then there is no
    // primitive string for this data
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
