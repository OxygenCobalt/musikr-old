use crate::mp3;
use crate::{errorln, print_entry};

use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, ErrorKind};

use clap::Values;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub struct DisplayTag {
    pub name: DisplayName,
    pub value: String,
}

impl DisplayTag {
    pub fn print(&self, indents: usize) {
        print_entry!("{}{}:", format!["{:>i$}", "", i = indents], self.name);

        let split: Vec<&str> = self
            .value
            .split('\n')
            .filter(|string| !string.is_empty())
            .collect();

        if split.len() == 1 {
            println!(" {}", split[0]);
        } else {
            let indent = format!["{:>i$}", "", i = indents + 2];

            println!();
            for line in split {
                println!("{}{}", indent, line);
            }
        }
    }
}

impl Ord for DisplayTag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for DisplayTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum DisplayName {
    Name(&'static str),
    Custom(&'static str, String),
    Unknown(String),
}

impl Ord for DisplayName {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Name(my_name), Self::Name(other_name)) => my_name.cmp(other_name),
            (Self::Custom(_, my_name), Self::Custom(_, other_name)) => my_name.cmp(other_name),
            (Self::Unknown(my_raw), Self::Unknown(other_raw)) => my_raw.cmp(other_raw),

            (Self::Name(_), Self::Custom(_, _)) => Ordering::Less,
            (Self::Custom(_, _), Self::Name(_)) => Ordering::Greater,

            (Self::Name(_), Self::Unknown(_)) => Ordering::Less,
            (Self::Unknown(_), Self::Name(_)) => Ordering::Greater,

            (Self::Custom(_, _), Self::Unknown(_)) => Ordering::Less,
            (Self::Unknown(_), Self::Custom(_, _)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for DisplayName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for DisplayName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Name(name) => write![f, "{}", name],
            Self::Custom(_, name) => write![f, "{}", name],
            Self::Unknown(raw) => write![f, "{}", raw],
        }
    }
}

/// Show plan:
/// -f Filters tags to specific names, like album or comment (desc), or IDs, like TPE1
///    If a tag is not found, then nothing happens.
/// -t Describes specific tag(s) to show. If it is not supplied, then all tags are
///    shown in order of their position in the media file.

#[derive(Debug)]
pub enum ShowError {
    IoError(io::Error),
    Unsupported,
    NoMetadata,
}

impl Display for ShowError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => write![f, "{}", err],
            Self::NoMetadata => write![f, "no metadata found"],
            Self::Unsupported => write![f, "unsupported file format"],
        }
    }
}

impl error::Error for ShowError {
    // Nothing to implement
}

impl From<io::Error> for ShowError {
    fn from(other: io::Error) -> Self {
        Self::IoError(other)
    }
}

pub type ShowResult = Result<(), ShowError>;
pub type TagFilter<'a> = Option<Values<'a>>;

pub fn show<'a>(paths: Values<'a>, filter: Option<Values<'a>>) -> ShowResult {
    for path in paths {
        // Borrow checker throws a hissy fit if I don't do an expensive clone of this
        // iterator every time.
        if let Err(err) = show_file(path, filter.clone()) {
            // It's okay if a file fails to parse here, just log the problem and move on.
            errorln!("{}: {}", path, err);
        }
    }

    Ok(())
}

fn show_file<'a>(path: &'a str, filter: TagFilter<'a>) -> ShowResult {
    // Validate that this path exists and isn't a directory here, mostly so that
    // when we determine the format a directory/non-existant file will be cryptically
    // marked as "unsupported".
    let path = new_path_safe(path)?;

    match path.extension() {
        Some(ext) if ext == "mp3" => mp3::show(path, filter),
        _ => Err(ShowError::Unsupported),
    }
}

fn new_path_safe(string: &str) -> Result<&Path, io::Error> {
    let path = Path::new(string);

    path.metadata()?;

    if path.is_dir() {
        return Err(io::Error::new(ErrorKind::Other, "Is a directory"));
    }

    Ok(path)
}
