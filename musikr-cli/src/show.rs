use std::path::Path; 

use musikr::id3v2::{self, tag::SaveVersion, frames::Frame};

use crate::args::{ReadTag, OpError};
use std::io::{self, ErrorKind};

static ID3V2_IDS: &[&[u8; 4]] = &[b"TALB", b"TPE2", b"TCON", b"TIT2", b"TRCK", b"TDRC"];

pub fn show(paths: &[&str], tags: &Option<Vec<ReadTag>>) {
	for path in paths {
		if let Err(err) = show_file(path, tags) {
			match err {
				OpError::IoError(err) => eprintln!("{}: {}", path, err),
				OpError::Unsupported => eprintln!("{}: unsupported format", path),
				OpError::Invalid => eprintln!("{}: malformed metadata", path),				
			}
		}
	}
}

fn show_file(path: &str, tags: &Option<Vec<ReadTag>>) -> Result<(), OpError> {
	let path = new_path_checked(path)?;

	match path.extension() {
		Some(ext) if ext == "mp3" => show_id3v2(path, tags),
		_ => Err(OpError::Unsupported)
	}	
}

fn show_id3v2(path: &Path, tags: &Option<Vec<ReadTag>>) -> Result<(), OpError> {
	let mut tag = id3v2::Tag::open(path).map_err(|err|
		match err {
			id3v2::ParseError::IoError(err) if err.kind() != ErrorKind::UnexpectedEof => 
				OpError::IoError(err),

			_ => OpError::Invalid
		}
	)?;

	tag.update(SaveVersion::V24);

    let frames: Vec<&dyn Frame> = match tags {
    	Some(tags) => tag.frames.values().filter(|frame| {
    		for tag in tags {
    			if frame.id() == ID3V2_IDS[*tag as usize] {
    				return true
    			}
    		}

    		return false
    	}).collect(),

    	None => tag.frames.values().collect()
    };

    for frame in &frames {
        println!("\"{}\"={}", frame.id(), frame);
    }

	Ok(())
}

fn new_path_checked(string: &str) -> Result<&Path, OpError> {
	let path = Path::new(string);

	if !path.exists() {
		return Err(OpError::IoError(io::Error::new(ErrorKind::NotFound, "no such file or directory")))
	}

	if path.is_dir() {
		return Err(OpError::IoError(io::Error::new(ErrorKind::Other, "is a directory")))
	}

	Ok(path)
}