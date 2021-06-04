use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};
use std::fmt::{self, Display, Formatter};

pub struct AttatchedPictureFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    mime: MimeType,
    desc: String,
    pic_type: Type,
    pic_data: Vec<u8>,
}

impl AttatchedPictureFrame {
    pub(super) fn new(header: Id3FrameHeader, data: &[u8]) -> AttatchedPictureFrame {
        let encoding = Encoding::from_raw(data[0]);

        let (mime, mime_size) = string::get_terminated_string(Encoding::Utf8, &data[1..]);
        let mut pos = 1 + mime_size;
        let mime = MimeType::from(mime);

        let pic_type = Type::new(data[pos]);
        pos += 1;

        let (desc, desc_size) = string::get_terminated_string(encoding, &data[pos..]);
        pos += desc_size;

        let pic_data = data[pos..].to_vec();

        AttatchedPictureFrame {
            header,
            encoding,
            mime,
            desc,
            pic_type,
            pic_data,
        }
    }

    pub fn mime(&self) -> &MimeType {
        &self.mime
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.pic_data
    }

    pub fn pic_type(&self) -> &Type {
        &self.pic_type
    }

    pub fn type_str(&self) -> &str {
        &self.pic_type.readable_name()
    }
}

impl Id3Frame for AttatchedPictureFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }
}

impl Display for AttatchedPictureFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{} ", self.mime]?;

        if !self.desc.is_empty() {
            write![f, "\"{}\" ", self.desc]?;
        }

        write![f, "[{}]", self.pic_type]
    }
}

byte_enum! {
    pub enum Type {
        Other = 0x00,
        FileIcon = 0x01,
        OtherFileIcon = 0x02,
        FrontCover = 0x03,
        BackCover = 0x04,
        LeafletPage = 0x05,
        Media = 0x06,
        LeadArtist = 0x07,
        Artist = 0x08,
        Conductor = 0x09,
        Band = 0x0A,
        Composer = 0x0B,
        Writer = 0x0C,
        RecordingLocation = 0x0D,
        DuringRecording = 0x0E,
        DuringPerformance = 0x0F,
        MovieScreenCapture = 0x10,
        ColoredFish = 0x11,
        Illustration = 0x12,
        BandLogo = 0x13,
        PublisherLogo = 0x14,
    }
}

const TYPE_STRS: &[&str; 21] = &[
    "Other",
    "32x32 file icon",
    "Other file icon",
    "Front Cover",
    "Back Cover",
    "Leaflet Page",
    "Media",
    "Lead Artist",
    "Artist",
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

impl Type {
    pub fn readable_name(&self) -> &str {
        TYPE_STRS[*self as usize]
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::Other
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.readable_name()]
    }
}

pub enum MimeType {
    Png,
    Jpeg,
    Image,
    Other(String),
}

impl MimeType {
    fn from(mime: String) -> MimeType {
        return match mime.to_lowercase().as_str() {
            "image/png" => MimeType::Png,
            "image/jpeg" => MimeType::Jpeg,
            "" => MimeType::Image, // Image is implied when there is no MIME type
            _ => MimeType::Other(mime), // Unknown mime type not outlined by the spec
        };
    }
}

impl Display for MimeType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mime_str = match self {
            // Formatted values for convienence
            MimeType::Png => "PNG",
            MimeType::Jpeg => "JPEG",
            MimeType::Image => "Image",

            // Default to the raw mime type if it's unknown
            MimeType::Other(mime) => mime.as_str(),
        };

        write![f, "{}", mime_str]
    }
}
