#![forbid(unsafe_code)]

mod args;
mod logger;
mod show;

#[macro_use]
extern crate clap;

use logger::PedanticLogger;
use clap::AppSettings;
use std::process;

fn main() {
    // I do not like clap. It breaks all CLI conventions with excessive newline messages and 
    // infantilizing "oh uwu u fowgot an awgument" garbage. Overriding these messages is deeply
    // impractical and undocumented on purpose so that you're railroaded into their bloated
    // lowest-common-denominator vision of what "command-line **APPS**" should be. I only use it
    // because I would rather get musikr working that focusing on pedantic garbage like this.
    let matches = clap_app!(app =>
        (name: "musikr")
        (version: crate_version!())
        (about: "Musikr is a utility for reading and writing audio metadata.")
        (setting: AppSettings::SubcommandRequiredElseHelp)
        (@arg pedantic: -p --pedantic "Print all technical information")
        (@subcommand show =>
            (about: "Read audio metadata")
            (@arg path: +required +hidden +takes_value +multiple "A file or directory to write to")
            (@arg tags: -t --tags +takes_value +multiple "Tags that should be shown")
            (settings: &[AppSettings::DisableVersion])
        )
    ).get_matches();

    if matches.is_present("pedantic") {
        PedanticLogger::setup();
    }

    let result = match matches.subcommand() {
        ("show", Some(show)) => {
            show::show(show.values_of("path").unwrap(), show.values_of("tags"))
        }

        _ => unreachable!()
    };

    if let Err(err) = result {
        eprintln!("musikr: {}", err);
        process::exit(1);
    }
}
