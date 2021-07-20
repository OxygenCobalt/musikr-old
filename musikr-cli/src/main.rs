#![forbid(unsafe_code)]

use std::env;
use std::io::ErrorKind;
use std::process;

use musikr::id3v2::ParseError;
use musikr::id3v2::Tag;

use log::{Level, LevelFilter, Log, Metadata, Record};

struct PedanticLogger;

impl Log for PedanticLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        let md = record.metadata();

        if self.enabled(md) {
            let module = record.module_path().unwrap_or_default();

            match md.level() {
                Level::Info => println!("\x1b[0;37m{}: {}\x1b[0m", module, record.args()),
                Level::Warn => eprintln!("\x1b[1;33m{}: {}\x1b[0m", module, record.args()),
                Level::Error => eprintln!(" \x1b[0;31m{}: {}\x1b[0m", module, record.args()),
                _ => println!("\x1b[1;30m{}: {}\x1b[0m", module, record.args()),
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: PedanticLogger = PedanticLogger;

fn main() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Info);

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
                    ParseError::IoError(io_err) if io_err.kind() != ErrorKind::UnexpectedEof => {
                        eprintln!("{}: {}", path, io_err);
                    }

                    _ => eprintln!("{}: invalid or unsupported metadata", path),
                }

                continue;
            }
        };

        println!("metadata for file: {}", path);

        for (key, frame) in &tag.frames {
            println!("\"{}\"={}", key, frame);
        }
    }
}
