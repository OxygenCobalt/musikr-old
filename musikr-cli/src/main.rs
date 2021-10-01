#![forbid(unsafe_code)]

mod args;
mod logger;
mod show;

use crate::args::ReadTag;
use crate::logger::PedanticLogger;
use clap::{App, Arg, SubCommand};

fn main() {
    PedanticLogger::setup();

    let matches = App::new("musikr")
        .subcommand(
            SubCommand::with_name("show")
                .help("Show the tags of a file")
                .arg(
                    Arg::with_name("files")
                        .takes_value(true)
                        .min_values(1)
                        .required(true),
                )
                .arg(
                    Arg::with_name("tags")
                        .short("t")
                        .help("Filter files by tag")
                        .takes_value(true)
                        .min_values(1)
                        .possible_values(args::TAG_NAMES),
                ),
        )
        .get_matches();

    // TODO: Make your own arg parser thats bland and functional and without the "oh uwu you forgot an argument" nonsense.
    //  Clap means nothing when you still have to output other error messages that will never line up.
    // TODO: Upgrade tags to ID3v2.4 when outputting [nowhere else]

    match matches.subcommand() {
        ("show", Some(show)) => {
            let files: Vec<&str> = show.values_of("files").unwrap().collect();

            let tags: Option<Vec<ReadTag>> = match show.values_of("tags") {
                Some(tag_args) => {
                    let mut tags = Vec::new();

                    for tag in tag_args {
                        tags.push(ReadTag::from_arg(tag).unwrap())
                    }

                    Some(tags)
                },

                _ => None
            };

            show::show(&files, &tags)
        },

        _ => {}
    };
}
