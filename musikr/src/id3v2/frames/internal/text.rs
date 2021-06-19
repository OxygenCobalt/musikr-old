use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::{ParseError, TagHeader};
use indexmap::IndexMap;
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 2 {
            // Must be at least 1 encoding byte and 1 byte of text data
            return Err(ParseError::NotEnoughData);
        }

        let encoding = Encoding::new(data[0])?;
        let text = parse_text(encoding, &data[1..]);

        Ok(TextFrame {
            header,
            encoding,
            text,
        })
    }

    pub(crate) fn is_text(frame_id: &str) -> bool {
        // Apple's WFED (Podcast URL), MVNM (Movement Name), MVIN (Movement Number),
        // and GRP1 (Grouping) frames are all actually text frames
        frame_id.starts_with('T') || matches!(frame_id, "WFED" | "MVNM" | "MVIN" | "GRP1")
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn text(&self) -> &Vec<String> {
        &self.text
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn text_mut(&mut self) -> &mut Vec<String> {
        &mut self.text
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

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn render(&self, header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = self.encoding.map_id3v2(header.major());
        result.push(encoding.render());

        result.extend(render_text(encoding, &self.text));

        result
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

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        let encoding = Encoding::parse(data)?;

        if data.len() < encoding.nul_size() + 2 {
            return Err(ParseError::NotEnoughData);
        }

        let desc = string::get_terminated(encoding, &data[1..]);
        let text = parse_text(encoding, &data[1 + desc.size..]);

        Ok(UserTextFrame {
            header,
            encoding,
            desc: desc.string,
            text,
        })
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

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn desc_mut(&mut self) -> &mut String {
        &mut self.desc
    }

    pub fn text_mut(&mut self) -> &mut Vec<String> {
        &mut self.text
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

    fn is_empty(&self) -> bool {
        self.desc.is_empty() && self.text.is_empty()
    }

    fn render(&self, header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = self.encoding.map_id3v2(header.major());
        result.push(encoding.render());

        // Append the description
        result.extend(string::render_terminated(encoding, &self.desc));

        // Then append the text normally.
        result.extend(render_text(encoding, &self.text));

        result
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
    people: IndexMap<String, String>,
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
            people: IndexMap::new(),
        }
    }

    pub(crate) fn parse(header: FrameHeader, data: &[u8]) -> Result<Self, ParseError> {
        let encoding = Encoding::parse(data)?;

        if data.len() < 2 {
            return Err(ParseError::NotEnoughData);
        }

        let mut text = parse_text(encoding, &data[1..]);

        if text.len() % 2 != 0 {
            // The spec says that TIPL must contain an even number of entries.
            // If this frame does have an incomplete pair, we just pop it off and move on.
            text.pop();
        }

        // Collect the parsed text into a single people map by role -> person.
        let mut people = IndexMap::new();
        let mut text = text.into_iter();

        while let Some(role) = text.next() {
            // We eliminated the possibility of an incomplete pair earlier, so we can
            // just unwrap here
            let role_people = text.next().unwrap();

            people.insert(role, role_people);
        }

        Ok(CreditsFrame {
            header,
            encoding,
            people,
        })
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn people(&self) -> &IndexMap<String, String> {
        &self.people
    }

    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    pub fn people_mut(&mut self) -> &mut IndexMap<String, String> {
        &mut self.people
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

    fn is_empty(&self) -> bool {
        self.people.is_empty()
    }

    fn render(&self, header: &TagHeader) -> Vec<u8> {
        let mut result = Vec::new();

        let encoding = self.encoding.map_id3v2(header.major());
        result.push(encoding.render());

        // Rendering a CreditsFrame is similar to a TextFrame, but has to be done
        // in pairs since there seems to be no way to zip keys and values into
        // an iterator without having to bring in a dependency.
        for (i, (role, people)) in self.people.iter().enumerate() {
            if i > 0 {
                result.resize(result.len() + encoding.nul_size(), 0);
            }

            result.extend(string::render_terminated(encoding, role));
            result.extend(string::render_string(encoding, people));
        }

        result
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

fn parse_text(encoding: Encoding, data: &[u8]) -> Vec<String> {
    // Text frames can contain multiple strings seperated by a NUL terminator, so we have to
    // manually iterate and find each terminated string. If there are none, then the Vec should
    // just contain one string without any issue.
    let mut text: Vec<String> = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        let fragment = string::get_terminated(encoding, &data[pos..]);

        pos += fragment.size;
        text.push(fragment.string);
    }

    text
}

fn render_text(encoding: Encoding, text: &[String]) -> Vec<u8> {
    let mut result = Vec::new();

    for (i, string) in text.iter().enumerate() {
        // Seperate each string by a NUL except for the last string.
        // For frames with a single string, there will be no NUL terminator.

        if i > 0 {
            result.resize(result.len() + encoding.nul_size(), 0)
        }

        result.extend(string::render_string(encoding, string));
    }

    result
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

        let frame = TextFrame::parse(FrameHeader::new("TIT2"), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Utf16);
        assert_eq!(frame.text()[0], "I Swallowed Hard, Like I Understood");
    }

    #[test]
    fn parse_txxx() {
        let data = b"\x00\
                     replaygain_track_gain\0\
                     -7.429688 dB";

        let frame = UserTextFrame::parse(FrameHeader::new("TXXX"), &data[..]).unwrap();

        assert_eq!(frame.encoding(), Encoding::Latin1);
        assert_eq!(frame.desc(), "replaygain_track_gain");
        assert_eq!(frame.text()[0], "-7.429688 dB");
    }

    #[test]
    fn parse_credits() {
        let data = b"\x00\
                     Violinist\0\
                     Vanessa Evans\0\
                     Bassist\0\
                     John Smith";

        let frame = CreditsFrame::parse(FrameHeader::new("TMCL"), &data[..]).unwrap();
        let people = frame.people();

        assert_eq!(frame.encoding(), Encoding::Latin1);
        assert_eq!(people["Violinist"], "Vanessa Evans");
        assert_eq!(people["Bassist"], "John Smith");
    }

    #[test]
    fn render_text_frame() {
        let out = b"\x01\
                     \xFF\xFE\x49\x00\x20\x00\x53\x00\x77\x00\x61\x00\x6c\x00\x6c\x00\
                     \x6f\x00\x77\x00\x65\x00\x64\x00\x20\x00\x48\x00\x61\x00\x72\x00\
                     \x64\x00\x2c\x00\x20\x00\x4c\x00\x69\x00\x6b\x00\x65\x00\x20\x00\
                     \x49\x00\x20\x00\x55\x00\x6e\x00\x64\x00\x65\x00\x72\x00\x73\x00\
                     \x74\x00\x6f\x00\x6f\x00\x64\x00";

        let mut frame = TextFrame::new("TIT2");
        *frame.encoding_mut() = Encoding::Utf16;
        frame
            .text_mut()
            .push(String::from("I Swallowed Hard, Like I Understood"));

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(3)), out)
    }

    #[test]
    fn render_multi_text() {
        let out = b"Post-Rock\0\
                    Ambient\0\
                    Electronica";

        let data = vec![
            "Post-Rock".to_string(),
            "Ambient".to_string(),
            "Electronica".to_string(),
        ];

        assert_eq!(render_text(Encoding::Latin1, &data), out);
    }

    #[test]
    fn render_txxx() {
        let out = b"\x00\
                    replaygain_track_gain\0\
                    -7.429688 dB";

        let mut frame = UserTextFrame::new();
        *frame.encoding_mut() = Encoding::Latin1;
        frame.desc_mut().push_str("replaygain_track_gain");
        frame.text_mut().push(String::from("-7.429688 dB"));

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), out);
    }

    #[test]
    fn render_credits() {
        let out = b"\x00\
                    Violinist\0\
                    Vanessa Evans\0\
                    Bassist\0\
                    John Smith";

        let mut frame = CreditsFrame::new_tmcl();
        *frame.encoding_mut() = Encoding::Latin1;

        let people = frame.people_mut();
        people.insert("Violinist".to_string(), "Vanessa Evans".to_string());
        people.insert("Bassist".to_string(), "John Smith".to_string());

        assert!(!frame.is_empty());
        assert_eq!(frame.render(&TagHeader::with_version(4)), out);
    }
}
