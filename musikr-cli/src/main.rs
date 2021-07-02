#![forbid(unsafe_code)]

use std::env;
use std::process;
use std::io::ErrorKind;

use musikr::id3v2::Tag;
use musikr::id3v2::ParseError;

fn main() {
    let mut args = env::args();

    if args.len() < 2 {
        println!("usage: musikr [FILES...]");
        process::exit(1);
    }

    args.next();

    for path in args {
        let tag = match Tag::open(&path) {
            Ok(file) => file,
            Err(err) => {
                match err {
                    ParseError::IoError(err) => {
                        if err.kind() != ErrorKind::UnexpectedEof {
                            eprintln!("{}: {}", path, err)
                        }
                    }

                    _ => eprintln!("{}: {}", path, err)
                }

                continue;
            }
        };
        
        println!("Metadata for file: {}", path);

        for (key, frame) in tag.frames() {
            println!("\"{}\"={}", key, frame);
        }
    }
}
