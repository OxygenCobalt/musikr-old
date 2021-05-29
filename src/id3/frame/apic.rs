use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string;
use crate::id3::frame::string::Encoding;
use crate::id3::frame::Id3Frame;
use crate::id3::frame::Id3FrameHeader;
use crate::id3::util;

const TYPE_STRINGS: &'static [&'static str; 21] = &[
    "Other",
    "32x32 file icon",
    "Other file icon",
    "Front Cover",
    "Back Cover",
    "Leaflet Page",
    "Media",
    "Lead Performer",
    "Performer",
    "Conductor",
    "Band/Orchestra",
    "Composer",
    "Writer",
    "Recording Location",
    "During recording",
    "During performance",
    "Movie/Video screenshot",
    "A bright colored fish",
    "Illustration",
    "Band/Artist Logotype",
    "Publisher/Studio Logotype",
];

pub struct ApicFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    mime: ApicMimeType,
    desc: String,
    pic_type: u8,
    pic_data: Vec<u8>,
}

impl ApicFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> ApicFrame {
        let mut pos = 0;

        let encoding = Encoding::from(data[pos]);

        let mime = match string::get_nul_string(&Encoding::Utf8, &data[1..]) {
            Some(mime) => {
                pos += mime.len() + 2;
                ApicMimeType::from(mime)
            }

            // If theres no mime type, Image is implied
            None => {
                pos += 2;
                ApicMimeType::Image
            }
        };

        let pic_type = data[pos];

        pos += 1;

        let desc = string::get_nul_string(&encoding, &data[pos..])
            .unwrap_or_default();

        pos += desc.len() + encoding.get_nul_size();

        // Cloning directly makes editing and lifecycle management easier
        let pic_raw = &data[pos..];
        let mut pic_data = vec![0; pic_raw.len()];
        pic_data.clone_from_slice(pic_raw);

        return ApicFrame {
            header,
            encoding,
            mime,
            desc,
            pic_type,
            pic_data,
        };
    }

    pub fn mime(&self) -> &ApicMimeType {
        return &self.mime;
    }

    pub fn desc(&self) -> &String {
        return &self.desc;
    }

    pub fn data(&self) -> &Vec<u8> {
        return &self.pic_data;
    }

    pub fn type_str(&self) -> &str {
        return TYPE_STRINGS
            .get(self.pic_type as usize)
            .unwrap_or(&TYPE_STRINGS[0]); // Return "Other" if we have an invalid type byte
    }

    fn fmt_mime(&self) -> &str {
        return match self.mime {
            ApicMimeType::Png => "PNG",
            ApicMimeType::Jpeg => "JPEG",
            ApicMimeType::Image => "Image",
        };
    }

    fn fmt_desc(&self) -> String {
        return if self.desc == "" {
            String::from(" ")
        } else {
            format![" \"{}\" ", self.desc]
        };
    }
}

impl Id3Frame for ApicFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for ApicFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![
            f,
            "{}{}{}[{}]",
            fmt_size(&self.mime, &self.pic_data),
            self.fmt_mime(),
            self.fmt_desc(),
            self.type_str()
        ]
    }
}

pub enum ApicMimeType {
    Png,
    Jpeg,
    Image,
}

impl ApicMimeType {
    fn from(mime: String) -> ApicMimeType {
        return match mime.to_lowercase().as_str() {
            "image/png" => ApicMimeType::Png,
            "image/jpeg" => ApicMimeType::Jpeg,

            // There may be other possible mime types, but the spec only outlines png/jpeg
            _ => ApicMimeType::Image,
        };
    }
}

fn fmt_size(mime: &ApicMimeType, data: &Vec<u8>) -> String {
    // Bringing in a whole image dependency just to get a width/height is dumb, so I parse it myself.
    // Absolutely nothing can go wrong with this. Trust me.

    let (width, height) = match mime {
        ApicMimeType::Png => parse_size_png(data),
        ApicMimeType::Jpeg => parse_size_jpg(data),

        // Can't parse a generic image
        ApicMimeType::Image => (0, 0),
    };

    if width == 0 && height == 0  {
        // Could not parse size
        return String::new();
    }

    return format!["{}x{} ", width, height];
}

fn parse_size_png(data: &Vec<u8>) -> (usize, usize) {
    // PNG sizes should be in the IDHR frame, which is always the first frame
    // after the PNG header. This means that the width and height should be at
    // fixed locations.

    return (
        util::size_decode(&data[16..20]),
        util::size_decode(&data[20..24]),
    );
}

fn parse_size_jpg(data: &Vec<u8>) -> (usize, usize) {
    // JPEG sizes are in the baseline DCT, which can be anywhere in the file,
    // therefore we have to manually search the file for the beginning of the
    // DCT and then get the size from there.

    for i in 0..data.len() {
        // Can't check by chunks of 2 since the codes could be misaligned
        let first = data[i];
        let second: u8 = *data.get(i + 1).unwrap_or(&0);

        if first == 0xFF && second == 0xC0 {
            let dct = &data[(i + 4)..(i + 10)];

            let height = u16::from_be_bytes([dct[1], dct[2]]);
            let width = u16::from_be_bytes([dct[3], dct[4]]);

            return (width.into(), height.into());
        }
    }

    return (0, 0);
}
