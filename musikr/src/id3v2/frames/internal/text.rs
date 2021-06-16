use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

pub struct TextFrame {
    header: FrameHeader,
    encoding: Encoding,
    text: Vec<String>,
}

impl TextFrame {
    pub fn new(frame_id: &str) -> Self {
        Self::with_flags(frame_id, FrameFlags::default())
    }

    pub fn with_flags(frame_id: &str, flags: FrameFlags) -> Self {
        if !Self::is_text(frame_id) {
            panic!("Text Frame IDs must begin with a T or be WFED/MVNM/MVIN/GRP1.");
        }

        if frame_id == "TXXX" {
            panic!("TextFrame cannot encode TXXX frames. Try UserTextFrame instead.")
        }

        Self::with_header(FrameHeader::with_flags(frame_id, flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        TextFrame {
            header,
            encoding: Encoding::default(),
            text: Vec::new(),
        }
    }

    pub(crate) fn is_text(frame_id: &str) -> bool {
        // Apple's WFED (Podcast URL), MVNM (Movement Name), MVIN (Movement Number),
        // and GRP1 (Grouping) frames are all actually text frames
        frame_id.starts_with('T') || matches!(frame_id, "WFED" | "MVNM" | "MVIN" | "GRP1")
    }

    pub fn text(&self) -> &Vec<String> {
        &self.text
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }
}

impl Frame for TextFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        self.id().clone()
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        if data.len() < 2 {
            return Err(ParseError::NotEnoughData);
        }

        self.encoding = Encoding::new(data[0])?;

        // Text frames can contain multiple strings seperated by a NUL terminator, so
        // we have to manually iterate and find each terminated string.
        // If there are none, then the Vec should just contain one string without
        // any issue.
        let mut pos = 1;

        while pos < data.len() {
            let fragment = string::get_terminated_string(self.encoding(), &data[pos..]);

            pos += fragment.size;
            self.text.push(fragment.string);
        }

        Ok(())
    }
}

impl Display for TextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_text(&self.text, f)
    }
}

pub struct UserTextFrame {
    header: FrameHeader,
    encoding: Encoding,
    desc: String,
    text: Vec<String>,
}

impl UserTextFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flags(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("TXXX", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        UserTextFrame {
            header,
            encoding: Encoding::default(),
            desc: String::new(),
            text: Vec::new(),
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn text(&self) -> &Vec<String> {
        &self.text
    }
}

impl Frame for UserTextFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.desc]
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 2 {
            return Err(ParseError::NotEnoughData);
        }

        let desc = string::get_terminated_string(self.encoding, &data[1..]);
        self.desc = desc.string;

        // Text strings, it's unclear whether TXXX can contain multiple strings, but we support it
        // anyway just in case.
        let mut pos = 1 + desc.size;

        while pos < data.len() {
            let fragment = string::get_terminated_string(self.encoding(), &data[pos..]);

            pos += fragment.size;
            self.text.push(fragment.string);
        }
        Ok(())
    }
}

impl Display for UserTextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt_text(&self.text, f)
    }
}

impl Default for UserTextFrame {
    fn default() -> Self {
        Self::with_flags(FrameFlags::default())
    }
}

pub struct CreditsFrame {
    header: FrameHeader,
    encoding: Encoding,
    people: HashMap<String, String>,
}

impl CreditsFrame {
    pub fn new_tipl() -> Self {
        Self::with_flags_tipl(FrameFlags::default())
    }

    pub fn new_tmcl() -> Self {
        Self::with_flags_tmcl(FrameFlags::default())
    }

    pub fn with_flags_tipl(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("TIPL", flags))
    }

    pub fn with_flags_tmcl(flags: FrameFlags) -> Self {
        Self::with_header(FrameHeader::with_flags("TMCL", flags))
    }

    pub(crate) fn with_header(header: FrameHeader) -> Self {
        CreditsFrame {
            header,
            encoding: Encoding::default(),
            people: HashMap::new(),
        }
    }

    pub fn people(&self) -> &HashMap<String, String> {
        &self.people
    }

    pub fn is_musician_credits(&self) -> bool {
        self.id() == "TIPL"
    }

    pub fn is_involved_people(&self) -> bool {
        self.id() == "IPLS" || self.id() == "TMCL"
    }
}

impl Frame for CreditsFrame {
    fn id(&self) -> &String {
        self.header.id()
    }

    fn size(&self) -> usize {
        self.header.size()
    }

    fn flags(&self) -> &FrameFlags {
        self.header.flags()
    }

    fn key(&self) -> String {
        // This technically opens the door for IPLS and TIPL to co-exist
        // in a tag, but that probably shouldn't occur.
        self.id().clone()
    }

    fn parse(&mut self, _header: &TagHeader, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < 2 {
            return Err(ParseError::NotEnoughData);
        }

        let mut pos = 1;

        while pos < data.len() {
            // Credits frames are stored roughly as:
            // ROLE/INSTRUMENT (Terminated String)
            // PERSON, PERSON, PERSON (Terminated String)
            // Neither should be empty ideally, but we can handle it if it is.

            let role = string::get_terminated_string(self.encoding, &data[pos..]);
            pos += role.size;

            // We don't bother parsing the people list here as that creates useless overhead.

            let people = string::get_terminated_string(self.encoding, &data[pos..]);
            pos += people.size;

            if !role.string.is_empty() {
                self.people.insert(role.string, people.string);
            }
        }

        Ok(())
    }
}

impl Display for CreditsFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Involved people list will start with a newline and end with no newline, for formatting convienence.
        for (role, people) in self.people.iter() {
            write![f, "\n{}: {}", role, people]?;
        }

        Ok(())
    }
}

fn fmt_text(text: &[String], f: &mut Formatter) -> fmt::Result {
    // Write the first entry w/o a space
    write![f, "{}", text[0]]?;

    if text.len() > 1 {
        // Write the rest with spaces
        for string in &text[1..] {
            write![f, " {}", string]?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_frame() {
        let data = b"\x01\
                     \xFF\xFE\x49\x00\x20\x00\x53\x00\x77\x00\x61\x00\x6c\x00\x6c\x00\
                     \x6f\x00\x77\x00\x65\x00\x64\x00\x20\x00\x48\x00\x61\x00\x72\x00\
                     \x64\x00\x2c\x00\x20\x00\x4c\x00\x69\x00\x6b\x00\x65\x00\x20\x00\
                     \x49\x00\x20\x00\x55\x00\x6e\x00\x64\x00\x65\x00\x72\x00\x73\x00\
                     \x74\x00\x6f\x00\x6f\x00\x64\x00";

        let mut frame = TextFrame::new("TIT2");
        frame.parse(&TagHeader::new(4), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16);
        assert_eq!(frame.text()[0], "I Swallowed Hard, Like I Understood");
    }

    #[test]
    fn parse_multi_text_frame() {
        let data = b"\x03\
                     Electronica\0\
                     Ambient";

        let mut frame = TextFrame::new("TCON");
        frame.parse(&TagHeader::new(4), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf8);
        assert_eq!(frame.text()[0], "Electronica");
        assert_eq!(frame.text()[1], "Ambient");
    }

    #[test]
    fn parse_txxx() {
        let data = b"\x00\
                     replaygain_track_gain\0\
                     -7.429688 dB";

        let mut frame = UserTextFrame::new();
        frame.parse(&TagHeader::new(4), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Latin1);
        assert_eq!(frame.desc(), "replaygain_track_gain");
        assert_eq!(frame.text()[0], "-7.429688 dB");
    }

    #[test]
    fn parse_multi_txxx() {
        let data = b"\x00\
                     Description\0\
                     Text1\0\
                     Text2";

        let mut frame = UserTextFrame::new();
        frame.parse(&TagHeader::new(4), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Latin1);
        assert_eq!(frame.desc(), "Description");
        assert_eq!(frame.text()[0], "Text1");
        assert_eq!(frame.text()[1], "Text2");
    }

    #[test]
    fn parse_credits() {
        let data = b"\x00\
                     Violinist\0\
                     Vanessa Evans\0\
                     Bassist\0\
                     John Smith";

        let mut frame = CreditsFrame::new_tmcl();
        frame.parse(&TagHeader::new(4), &data[..]).unwrap();

        let people = frame.people();

        assert_eq!(people["Violinist"], "Vanessa Evans");
        assert_eq!(people["Bassist"], "John Smith");
    }
}