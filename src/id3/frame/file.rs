use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

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
    mime: PictureMimeType,
    desc: String,
    pic_type: u8,
    pic_data: Vec<u8>,
}

impl AttatchedPictureFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> AttatchedPictureFrame {
        let encoding = Encoding::from(data[0]);

        let (mime, mime_size) = string::get_terminated_string(&Encoding::Utf8, &data[1..]);
        let mut pos = 1 + mime_size;
        let mime = PictureMimeType::from(mime);

        let pic_type = data[pos];
        pos += 1;

        let (desc, desc_size) = string::get_terminated_string(&encoding, &data[pos..]);
        pos += desc_size;

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

    pub fn mime(&self) -> &PictureMimeType {
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
        write![f, "{} ", self.mime]?;

        if !self.desc.is_empty() {
            write![f, "\"{}\" ", self.desc]?;
        }

        write![f, "[{}]", self.type_str()]?;

        return Ok(());
    }
}

pub enum PictureMimeType {
    Png,
    Jpeg,
    Image,
    Other(String),
}

impl PictureMimeType {
    fn from(mime: String) -> PictureMimeType {
        return match mime.to_lowercase().as_str() {
            "image/png" => PictureMimeType::Png,
            "image/jpeg" => PictureMimeType::Jpeg,
            "" => PictureMimeType::Image, // Image is implied when there is no MIME type
            _ => PictureMimeType::Other(mime), // Unknown mime type not outlined by the spec
        };
    }
}

impl Display for PictureMimeType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mime_str = match self {
            // Hardcoded values for convienence
            PictureMimeType::Png => "PNG",
            PictureMimeType::Jpeg => "JPEG",
            PictureMimeType::Image => "Image",

            // Default to the raw mime type if it's unknown
            PictureMimeType::Other(mime) => mime.as_str(),
        };

        write![f, "{}", mime_str]
    }
}

pub struct GeneralObjectFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    mime: String,
    filename: String,
    desc: String,
    data: Vec<u8>,
}

impl GeneralObjectFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> GeneralObjectFrame {
        let encoding = Encoding::from(data[0]);

        let (mime, mime_size) = string::get_terminated_string(&encoding, &data[1..]);
        let mut pos = mime_size + 1;

        let (filename, fn_size) = string::get_terminated_string(&encoding, &data[pos..]);
        pos += fn_size;

        let (desc, desc_size) = string::get_terminated_string(&encoding, &data[pos..]);
        pos += desc_size;

        let data = data[pos..].to_vec();

        return GeneralObjectFrame {
            header,
            encoding,
            mime,
            filename,
            desc,
            data,
        };
    }

    fn mime(&self) -> &String {
        return &self.mime;
    }

    fn filename(&self) -> &String {
        return &self.filename;
    }

    fn desc(&self) -> &String {
        return &self.desc;
    }

    fn data(&self) -> &Vec<u8> {
        return &self.data;
    }
}

impl Id3Frame for GeneralObjectFrame {
    fn id(&self) -> &String {
        return &self.header.frame_id;
    }

    fn size(&self) -> usize {
        return self.header.frame_size;
    }
}

impl Display for GeneralObjectFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.mime.is_empty() {
            write![f, "{} ", self.mime]?;
        }

        if !self.filename.is_empty() {
            write![f, "\"{}\"", self.filename]?;
        }

        if !self.desc.is_empty() {
            write![f, " [{}]", self.desc]?;
        }

        return Ok(());
    }
}
