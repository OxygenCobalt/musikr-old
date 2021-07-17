#![forbid(unsafe_code)]

use std::env;
use std::io::ErrorKind;
use std::process;

use musikr::id3v2::tag::SaveVersion;
use musikr::id3v2::ParseError;
use musikr::id3v2::Tag;

fn main() {
    let mut args = env::args();

    if args.len() < 2 {
        println!("usage: musikr [FILES...]");
        process::exit(1);
    }

    args.next();

    for path in args {
        let mut tag = match Tag::open(&path) {
            Ok(file) => file,
            Err(err) => {
                if let ParseError::IoError(io_err) = err {
                    if io_err.kind() != ErrorKind::UnexpectedEof {
                        eprintln!("{}: {}", path, io_err);
                    }
                } else {
                    eprintln!("{}: Invalid or unsupported metadata", path);
                }

                continue;
            }
        };

        println!("Metadata for file: {}", path);

        for (key, frame) in &tag.frames {
            println!("\"{}\"={}", key, frame);
        }

        tag.update(SaveVersion::V23);
    }
}
