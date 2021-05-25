use crate::id3::frame::ID3Frame;
use crate::id3::frame::FrameHeader;
use crate::id3::frame::string;
use crate::id3::frame::string::ID3Encoding;

const TYPE_STRINGS: &'static [&'static str; 21] = &[
    "Other", "32x32 file icon", "Other file icon", "Front Cover",
    "Back Cover", "Leaflet Page", "Media", "Lead Performer",
    "Performer", "Conductor", "Band/Orchestra", "Composer",
    "Writer", "Recording Location", "During recording", "During performance",
    "Movie/Video screenshot", "A bright colored fish", "Illustration",
    "Band/Artist Logotype", "Publisher/Studio Logotype"
];

pub struct APICFrame<'a> {
    header: FrameHeader,
    pub encoding: ID3Encoding,
    pub mime: ApicMimeType,
    pub desc: String,
    pub pic_type: u8,
    pub pic_data: &'a [u8]
}

pub enum ApicMimeType {
    Png, Jpeg, Image
}

impl ApicMimeType {
    fn from(mime: String) -> ApicMimeType {
        return match mime.to_lowercase().as_str() {
            "image/png" => ApicMimeType::Png,
            "image/jpeg" => ApicMimeType::Jpeg,

            // There may be other possible mime types, but the spec only outlines png/jpeg

            _ => ApicMimeType::Image
        }
    }
}

impl <'a> ID3Frame for APICFrame<'a> {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }

    fn format(&self) -> String {
        return format![
            "{}: {}{}[{}]", 
            self.header.code,
            self.format_mime(),
            self.format_desc(),
            self.format_type()
        ];
    }
}

impl <'a> APICFrame<'a> {
    fn format_mime(&self) -> &str {
        return match self.mime {
            ApicMimeType::Png => "PNG",
            ApicMimeType::Jpeg => "JPEG",
            ApicMimeType::Image => "Image"
        }
    }

    fn format_desc(&self) -> String {
        return if self.desc == "" {
            String::from(" ")
        } else {
            format![" \"{}\" ", self.desc]
        }
    }

    fn format_type(&self) -> &str {
        return TYPE_STRINGS.get(self.pic_type as usize)
            .unwrap_or(&TYPE_STRINGS[0]); // Return "Other" if we have an invalid type byte
    }
}


impl <'a> APICFrame<'a> {
    pub fn from(header: FrameHeader, data: &[u8]) -> APICFrame {
        let mut pos = 0;

        let encoding = string::get_encoding(data[pos]);

        let mime = match string::get_nulstring(&ID3Encoding::UTF8, &data[1..]) {
            Some(mime) => {
                pos += mime.len() + 2;
                ApicMimeType::from(mime)
            },

            // If theres no mime type, Image is implied
            None => {
                pos += 2;
                ApicMimeType::Image
            }
        };

        let pic_type = data[pos];

        pos += 1;

        let desc = match string::get_nulstring(&encoding, &data[pos..]) {
            Some(desc) => {
                pos += desc.len() + 1;
                desc
            },

            None => {
                pos += 1;
                String::new()
            }
        };

        let pic_data = &data[pos..];

        return APICFrame {
            header, encoding, mime, desc, pic_type, pic_data
        };
    }
}
