use clap::Values;
use std::path::Path;

use musikr::id3v2::{self, frames::Frame, tag::SaveVersion};

use crate::args::{OpError, ReadTag};
use std::io::{self, ErrorKind};

pub fn show<'a>(paths: Values<'a>, tags: Option<Values<'a>>) -> Result<(), OpError> {
    // Parse out any tags if we have them. If this fails due to invalid tags, then we return an error.
    let tags = match tags {
        Some(tag_iter) => {
            let mut tags = Vec::new();

            for tag in tag_iter {
                tags.push(ReadTag::from_arg(tag)?);
            }

            Some(tags)
        }

        None => None,
    };

    for path in paths {
        // It's okay if a file fails to parse here, just log the problem and move on.
        if let Err(err) = show_file(path, &tags) {
            eprintln!("musikr: {}: {}", path, err);
        }
    }

    Ok(())
}

fn show_file(path: &str, tags: &Option<Vec<ReadTag>>) -> Result<(), OpError> {
    // Validate that this path exists and isn't a directory here, mostly so that
    // when we determine the format a directory/non-existant file will be cryptically
    // marked as "unsupported".
    let path = new_path_safe(path)?;

    match path.extension() {
        Some(ext) if ext == "mp3" => show_id3v2(path, tags),
        _ => Err(OpError::UnsupportedMetadata),
    }
}

fn new_path_safe(string: &str) -> Result<&Path, io::Error> {
    let path = Path::new(string);

    if !path.exists() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "no such file or directory",
        ));
    }

    if path.is_dir() {
        return Err(io::Error::new(ErrorKind::Other, "is a directory"));
    }

    Ok(path)
}

fn show_id3v2(path: &Path, tags: &Option<Vec<ReadTag>>) -> Result<(), OpError> {
    let mut tag = id3v2::Tag::open(path).map_err(|err| match err {
        id3v2::ParseError::IoError(err) if err.kind() != ErrorKind::UnexpectedEof => {
            OpError::IoError(err)
        }

        _ => OpError::MalformedMetadata,
    })?;

    // For a sane representation, always upgrade this tag to ID3v2.4.
    tag.update(SaveVersion::V24);

    let frames: Vec<&dyn Frame> = match tags {
        Some(tags) => tag
            .frames
            .values()
            .filter(|frame| {
                for tag in tags {
                    if frame.id() == tag.as_id3v2() {
                        return true;
                    }
                }

                return false;
            })
            .collect(),

        None => tag.frames.values().collect(),
    };

    for frame in &frames {
        match ReadTag::from_id3v2(frame.id()) {
            Some(tag) => println!("{}: {}", tag, frame),
            None => println!("\"{}\": {}", frame.id(), frame)
        };
    }

    Ok(())
}
