use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{FrameHeader, Id3Frame};
use std::fmt::{self, Display, Formatter};

pub struct AttatchedPictureFrame {
    header: FrameHeader,
    encoding: Encoding,
    mime: String,
    desc: String,
    pic_type: Type,
    pic_data: Vec<u8>,
}

impl AttatchedPictureFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        let encoding = Encoding::new(*data.get(0)?);

        if data.len() < encoding.nul_size() + 4 {
            return None; // Not enough data
        }

        let (mut mime, mime_size) = string::get_terminated_string(Encoding::Utf8, &data[1..]);

        // image/ is implied when there is no mime type.
        if mime.is_empty() {
            mime = "image/".to_string()
        }

        let mut pos = 1 + mime_size;

        let pic_type = Type::new(data[pos]);
        pos += 1;

        let (desc, desc_size) = string::get_terminated_string(encoding, &data[pos..]);
        pos += desc_size;

        let pic_data = data[pos..].to_vec();

        Some(AttatchedPictureFrame {
            header,
            encoding,
            mime,
            desc,
            pic_type,
            pic_data,
        })
    }

    pub fn from(frame: &dyn Id3Frame) -> Option<&Self> {
        frame.downcast_ref()
    }

    pub fn mime(&self) -> &String {
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
}

impl Id3Frame for AttatchedPictureFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn key(&self) -> String {
        // *Technically* the spec says that there can only be one FileIcon and OtherFileIcon 
        // APIC frame per tag, but pretty much no tagger enforces this.
        format!["{}:{}", self.id(), self.desc]
    }
}

impl Display for AttatchedPictureFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{} ", self.mime]?;

        if !self.desc.is_empty() {
            write![f, "\"{}\" ", self.desc]?;
        }

        write![f, "[{:?}]", self.pic_type]
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

impl Default for Type {
    fn default() -> Self {
        Type::Other
    }
}
