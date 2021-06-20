#![forbid(unsafe_code)]

use std::env;
use std::process;

use musikr::file::File;

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

        let tag = match file.id3v2() {
            Ok(tag) => tag,
            Err(err) => {
                eprintln!(
                    "musikr: {}: Invalid or unsupported metadata [{}]",
                    path, err
                );
                continue;
            }
        };

        println!("Metadata for file: {}", path);

        for (key, frame) in tag.frames() {
            println!("\"{}\"={}", key, frame);
        }
    }
}
