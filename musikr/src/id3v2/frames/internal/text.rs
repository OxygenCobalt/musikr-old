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
        self.text = parse_text(self.encoding, &data[1..]);

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

        let text_pos = 1 + desc.size;
        self.text = parse_text(self.encoding, &data[text_pos..]);

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

fn parse_text(encoding: Encoding, data: &[u8]) -> Vec<String> {
    let mut text = string::get_string(encoding, data);

    // Split the text up by a NUL character, which is what seperates
    // strings in a multi-string frame
    let text_by_nuls: Vec<&str> = text.split_terminator('\0')
        .filter(|&string| !string.is_empty())
        .collect();

    // A length of 1 means that this is a single-string frame
    if text_by_nuls.len() == 1 {
        // We manually truncate any nuls off of the string so that we
        // don't have to copy it.
        let len = text_by_nuls[0].len();
        text.truncate(len);

        return vec![text];
    }

    // If we have many strings, convert them all from string slices
    // to owned Strings
    text_by_nuls
        .iter()
        .map(|slice| String::from(*slice))
        .collect()
}

fn fmt_text(text: &[String], f: &mut Formatter) -> fmt::Result {
    println!("{:x?}", text[0]);

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
