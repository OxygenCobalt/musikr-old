#![forbid(unsafe_code)]

use std::env;
use std::process;

use musikr::file::File;
use musikr::id3::Id3Tag;

fn main() {
    let mut args = env::args();

    if args.len() < 2 {
        println!("usage: musikr [FILES...]");
        process::exit(1);
    }

    args.next();

    for path in args {
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("musikr: {}: {}", path, err);
                continue;
            }
        };

        let tag = match Id3Tag::new(&mut file) {
            Ok(tag) => tag,
            Err(_) => {
                eprintln!("musikr: {}: Invalid or unsupported metadata", path);
                continue;
            }
        };
        
        println!("Metadata for file: {}", path);

        for frame in tag.frames() {
            println!("{}: {}", frame.code(), frame);
        }
    }
}
