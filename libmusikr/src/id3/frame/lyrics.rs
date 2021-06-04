use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::time::{Timestamp, TimestampFormat};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};
use crate::raw;
use std::fmt::{self, Display, Formatter};

pub struct UnsyncLyricsFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    lang: String,
    desc: String,
    lyrics: String,
}

impl UnsyncLyricsFrame {
    pub(super) fn new(header: Id3FrameHeader, data: &[u8]) -> UnsyncLyricsFrame {
        let encoding = Encoding::from_raw(data[0]);

        let lang = string::get_string(Encoding::Utf8, &data[1..3]);
        let (desc, desc_size) = string::get_terminated_string(encoding, &data[4..]);

        let text_pos = 4 + desc_size;
        let lyrics = string::get_string(encoding, &data[text_pos..]);

        UnsyncLyricsFrame {
            header,
            encoding,
            lang,
            desc,
            lyrics,
        }
    }
}

impl Id3Frame for UnsyncLyricsFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }
}

impl Display for UnsyncLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.desc.is_empty() {
            write![f, "\n{}:", self.desc]?;
        }

        write![f, "\n{}", self.lyrics]
    }
}

pub struct SyncedLyricsFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    lang: String,
    time_format: TimestampFormat,
    content_type: SyncedContentType,
    desc: String,
    lyrics: Vec<SyncedText>,
}

impl SyncedLyricsFrame {
    pub(super) fn new(header: Id3FrameHeader, data: &[u8]) -> SyncedLyricsFrame {
        let encoding = Encoding::from_raw(data[0]);
        let lang = String::from_utf8_lossy(&data[1..3]).to_string();
        let time_format = TimestampFormat::new(data[4]);
        let content_type = SyncedContentType::new(data[5]);
        let (desc, desc_size) = string::get_terminated_string(encoding, &data[6..]);

        // For UTF-16 Synced Lyrics frames, a tagger might only write the BOM to the description
        // and nowhere else. If thats the case, we will subsitute the generic Utf16 encoding for
        // the implicit encoding if there is no bom in each lyric.

        let implicit_encoding = match encoding {
            Encoding::Utf16Bom => {
                let bom = raw::to_u16(&data[6..8]);

                match bom {
                    0xFFFE => Encoding::Utf16Le,
                    0xFEFF => Encoding::Utf16Be,
                    _ => encoding,
                }
            }

            _ => encoding,
        };

        let mut pos = desc_size + 6;
        let mut lyrics: Vec<SyncedText> = Vec::new();

        while pos < header.frame_size {
            let bom = raw::to_u16(&data[pos..pos + 2]);

            // If the lyric does not have a BOM, use the implicit encoding we got earlier.
            let enc = if bom != 0xFEFF && bom != 0xFFFE {
                implicit_encoding
            } else {
                encoding
            };

            let (text, text_size) = string::get_terminated_string(enc, &data[pos..]);
            pos += text_size;
            let timestamp = time_format.make_timestamp(raw::to_u32(&data[pos..pos + 4]));
            pos += 4;

            lyrics.push(SyncedText { text, timestamp })
        }

        SyncedLyricsFrame {
            header,
            encoding,
            lang,
            time_format,
            content_type,
            desc,
            lyrics,
        }
    }

    pub fn lang(&self) -> &String {
        &self.lang
    }

    pub fn time_format(&self) -> TimestampFormat {
        self.time_format
    }

    pub fn content_type(&self) -> SyncedContentType {
        self.content_type
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn lyrics(&self) -> &Vec<SyncedText> {
        &self.lyrics
    }
}

impl Id3Frame for SyncedLyricsFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }
}

impl Display for SyncedLyricsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Append a brief header if we have a description, otherwise we omit the content type
        // altogether since it only really works in conjunction with a description
        if !self.desc.is_empty() {
            write![f, "\n\"{}\" [{}]:", self.desc, self.content_type]?;
        }

        for lyric in &self.lyrics {
            write![f, "\n{}", lyric]?;
        }

        Ok(())
    }
}

byte_enum! {
    pub enum SyncedContentType {
        Other = 0x00,
        Lyrics = 0x01,
        TextTranscription = 0x02,
        Movement = 0x03,
        Events = 0x04,
        Chord = 0x05,
        Trivia = 0x06,
        WebpageUrls = 0x07,
        ImageUrls = 0x08,
    }
}

const TYPE_STRS: &[&str; 8] = &[
    "Other",
    "Lyrics",
    "Text Transcription",
    "Movement",
    "Events",
    "Trivia",
    "Webpage URLs",
    "Image URLs",
];

impl SyncedContentType {
    pub fn readable_name(&self) -> &str {
        TYPE_STRS[*self as usize]
    }
}

impl Display for SyncedContentType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.readable_name()]
    }
}

impl Default for SyncedContentType {
    fn default() -> Self {
        SyncedContentType::Other
    }
}

pub struct SyncedText {
    pub text: String,
    pub timestamp: Timestamp,
}

impl Display for SyncedText {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Since we are formatting this already, strip any trailing newlines from the lyrics
        // using a somewhat clunky if block.
        let text = if self.text.starts_with('\n') {
            self.text
                .strip_prefix("\r\n")
                .or_else(|| self.text.strip_prefix("\n"))
                .unwrap_or(&self.text)
        } else if self.text.ends_with('\n') {
            self.text
                .strip_suffix("\r\n")
                .or_else(|| self.text.strip_suffix("\n"))
                .unwrap_or(&self.text)
        } else {
            &self.text
        };

        // Don't include the timestamp by default, as formatting time is beyond the scope of libmusikr
        write![f, "{}", text]
    }
}
