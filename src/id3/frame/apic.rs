use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};
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

pub struct AttatchedPictureFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    mime: ApicMimeType,
    desc: String,
    pic_type: u8,
    pic_data: Vec<u8>,
}

impl AttatchedPictureFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> AttatchedPictureFrame {
        let encoding = Encoding::from(data[0]);

        let mime = string::get_nul_string(&Encoding::Utf8, &data[1..]).unwrap_or_default();
        let mut pos = mime.len() + 2;
        let mime = ApicMimeType::from(mime);

        let pic_type = data[pos];
        pos += 1;

        let desc = string::get_nul_string(&encoding, &data[pos..]).unwrap_or_default();
        pos += desc.len() + encoding.get_nul_size();

        let pic_data = data[pos..].to_vec();

        return AttatchedPictureFrame {
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
}

impl Id3Frame for AttatchedPictureFrame {
    fn id(&self) -> &String {
        return &self.header.frame_id;
    }

    fn size(&self) -> usize {
        return self.header.frame_size;
    }
}

impl Display for AttatchedPictureFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (width, height) = get_size(&self.mime, &self.pic_data);

        if width != 0 && height != 0 {
            write![f, "{}x{} ", width, height]?;
        }

        let mime_str = match self.mime {
            ApicMimeType::Png => "PNG",
            ApicMimeType::Jpeg => "JPEG",
            ApicMimeType::Image => "Image",
        };

        write![f, "{} ", mime_str]?;

        if !self.desc.is_empty() {
            write![f, "\"{}\" ", self.desc]?;
        }

        write![f, "[{}]", self.type_str()]?;

        return Ok(());
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

fn get_size(mime: &ApicMimeType, data: &Vec<u8>) -> (usize, usize) {
    // Bringing in a whole image dependency just to get a width/height is dumb, so I parse it myself.
    // Absolutely nothing can go wrong with this. Trust me.

    return match mime {
        ApicMimeType::Png => parse_size_png(data),
        ApicMimeType::Jpeg => parse_size_jpg(data),

        // Can't parse a generic image
        ApicMimeType::Image => (0, 0),
    };
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
