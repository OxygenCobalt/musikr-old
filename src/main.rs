#![forbid(unsafe_code)]

// TODO: Move most of the tag parsing code into a libmusikr of some kind

use std::env;
use std::process;

mod id3;

use id3::ID3Tag;

fn main() {
    let mut args = env::args();

    if args.len() < 2 {
        println!("usage: musikr [FILES...]");
        process::exit(1);
    }

    args.next();

    for path in args {
        let mut file = match musikr::open(&path) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("musikr: {}: {}", path, err);
                continue;
            }
        };

        let tag = match ID3Tag::new(&mut file.handle) {
            Ok(tag) => tag,
            Err(_) => {
                eprintln!("musikr: {}: Invalid or unsupported metadata", path);
                continue;
            }
        };

        println!("Metadata for file: {}", path);

        for frame in tag.frames {
            println!("{}", frame.format());
        }
    }
}
