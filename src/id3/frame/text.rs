use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{Id3Frame, Id3FrameHeader};

pub struct TextFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    text: String,
}

impl TextFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> TextFrame {
        let encoding = Encoding::from(data[0]);
        let text = string::get_string(&encoding, &data[1..]);

        return TextFrame {
            header,
            encoding,
            text,
        };
    }

    pub fn text(&self) -> &String {
        return &self.text;
    }
}

impl Id3Frame for TextFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for TextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

pub struct UserTextFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    desc: String,
    text: String,
}

impl UserTextFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> UserTextFrame {
        let encoding = Encoding::from(data[0]);
        let desc = string::get_nul_string(&encoding, &data[1..]).unwrap_or_default();
        let text_pos = desc.len() + encoding.get_nul_size();
        let text = string::get_string(&encoding, &data[text_pos..]);

        return UserTextFrame {
            header,
            encoding,
            desc,
            text,
        };
    }

    pub fn desc(&self) -> &String {
        return &self.desc;
    }

    pub fn text(&self) -> &String {
        return &self.text;
    }
}

impl Id3Frame for UserTextFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for UserTextFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{}", self.text]
    }
}

pub struct InvolvedPeopleFrame {
    header: Id3FrameHeader,
    encoding: Encoding,
    people: HashMap<String, String>,
}

impl InvolvedPeopleFrame {
    pub(super) fn from(header: Id3FrameHeader, data: &[u8]) -> InvolvedPeopleFrame {
        let encoding = Encoding::from(data[0]);
        let mut people: HashMap<String, String> = HashMap::new();
        let mut pos = 1;

        while pos < data.len() {
            // Involved people are stored roughly as:
            // ROLE (Terminated String)
            // PERSON, PERSON, PERSON (Terminated String)
            // Neither should be empty ideally, but we can handle it if it is.

            let role = string::get_nul_string(&encoding, &data[pos..]).unwrap_or_default();
            pos += role.len() + 1;

            // We don't bother parsing the people list here as that creates useless overhead.

            let role_people = string::get_nul_string(&encoding, &data[pos..]).unwrap_or_default();
            pos += role_people.len() + 1;

            if !role.is_empty() {
                people.insert(role, role_people);
            }
        }

        return InvolvedPeopleFrame {
            header,
            encoding,
            people,
        };
    }

    pub fn people(&self) -> &HashMap<String, String> {
        return &self.people;
    }
}

impl Id3Frame for InvolvedPeopleFrame {
    fn code(&self) -> &String {
        return &self.header.code;
    }

    fn size(&self) -> usize {
        return self.header.size;
    }
}

impl Display for InvolvedPeopleFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Involved people list will start with a newline and end with no newline, for formatting convienence.
        for (role, people) in self.people.iter() {
            write![f, "\n{}: {}", role, people]?;
        }

        return Ok(());
    }
}
