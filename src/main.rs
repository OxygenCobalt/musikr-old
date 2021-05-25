#![forbid(unsafe_code)]

mod id3;

use std::env;
use std::process;

// TODO: Format code
// TODO: use unified `crate` syntax with parent crates everywhere
// TODO: Add "debugstr" macro that will fill in a readable error string but omit it from a release build

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

        let tag = match id3::new(&mut file) {
            Ok(tag) => tag,
            Err(_) => {
                eprintln!("musikr: {}: Invalid or unsupported metadata", path);
                continue;
            }
        };

        println!("Major Version: {}", tag.major);
        println!("Minor Version: {}", tag.minor);
        println!("Flags: {:x?}", tag.flags);
        println!("Size: {}", tag.size);

        let frames = id3::read_frames(&tag);

        for frame in frames {
            println!("{}", frame.format());
        }
    }
}
