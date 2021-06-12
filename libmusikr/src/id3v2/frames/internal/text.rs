use crate::id3v2::frames::string::{self, Encoding};
use crate::id3v2::frames::{Frame, FrameFlags, FrameHeader};
use crate::id3v2::ParseError;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

pub struct TextFrame {
    header: FrameHeader,
    encoding: Encoding,
    text: Text,
}

impl TextFrame {
    pub fn new(header: FrameHeader) -> Self {
        TextFrame {
            header,
            encoding: Encoding::default(),
            text: Text::One(String::new()),
        }
    }

    pub fn text(&self) -> &Text {
        &self.text
    }
}

impl Frame for TextFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn flags(&self) -> &FrameFlags {
        &self.header.flags
    }

    fn key(&self) -> String {
        self.id().clone()
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        if data.len() < 2 {
            return Err(ParseError::NotEnoughData);
        }

        self.encoding = Encoding::new(data[0])?;
        self.text = Text::new(self.encoding, &data[1..]);

        Ok(())
    }
}

impl Display for TextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

pub struct UserTextFrame {
    header: FrameHeader,
    encoding: Encoding,
    desc: String,
    text: Text,
}

impl UserTextFrame {
    pub fn new(header: FrameHeader) -> Self {
        UserTextFrame {
            header,
            encoding: Encoding::default(),
            desc: String::new(),
            text: Text::One(String::new()),
        }
    }

    pub fn desc(&self) -> &String {
        &self.desc
    }

    pub fn text(&self) -> &Text {
        &self.text
    }
}

impl Frame for UserTextFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn flags(&self) -> &FrameFlags {
        &self.header.flags
    }

    fn key(&self) -> String {
        format!["{}:{}", self.id(), self.desc]
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
        self.encoding = Encoding::parse(data)?;

        if data.len() < self.encoding.nul_size() + 2 {
            return Err(ParseError::NotEnoughData);
        }

        let desc = string::get_terminated_string(self.encoding, &data[1..]);
        self.desc = desc.string;

        let text_pos = 1 + desc.size;
        self.text = Text::new(self.encoding, &data[text_pos..]);

        Ok(())
    }
}

impl Display for UserTextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

pub struct CreditsFrame {
    header: FrameHeader,
    encoding: Encoding,
    people: HashMap<String, String>,
}

impl CreditsFrame {
    pub fn new(header: FrameHeader) -> Self {
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
        self.header.frame_id == "TMCL"
    }

    pub fn is_involved_people(&self) -> bool {
        self.header.frame_id == "TIPL" || self.header.frame_id == "IPLS"
    }
}

impl Frame for CreditsFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }

    fn flags(&self) -> &FrameFlags {
        &self.header.flags
    }

    fn key(&self) -> String {
        // This technically opens the door for IPLS and TIPL to co-exist
        // in a tag, but that probably shouldn't occur.
        self.id().clone()
    }

    fn parse(&mut self, data: &[u8]) -> Result<(), ParseError> {
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
        // TODO: The HashMap can vary in order, try to sort it
        for (role, people) in self.people.iter() {
            write![f, "\n{}: {}", role, people]?;
        }

        Ok(())
    }
}

pub enum Text {
    One(String),
    Many(Vec<String>),
}

impl Text {
    fn new(encoding: Encoding, data: &[u8]) -> Text {
        let text = string::get_string(encoding, data);

        // Split the text up by a NUL character, which is what seperates
        // strings in a multi-string frame
        let text_by_nuls: Vec<&str> = text.split('\u{0}').collect();

        if text_by_nuls.len() < 2 {
            // A length < 2 means that this is a single-string frame
            return Text::One(text);
        }

        // If we have many strings, convert them all from string slices
        // to owned Strings
        let text_full: Vec<String> = text_by_nuls
            .iter()
            .map(|slice| String::from(*slice))
            .collect();

        Text::Many(text_full)
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        return match self {
            Text::One(text) => {
                write![f, "{}", text]
            }

            Text::Many(text) => {
                // Write the first entry w/o a space
                write![f, "{}", text[0]]?;

                // Write the rest with spaces
                for string in &text[1..] {
                    write![f, " {}", string]?;
                }

                Ok(())
            }
        };
    }
}
