#![forbid(unsafe_code)]

mod mp3;
mod show;
mod stdout;

#[macro_use]
extern crate clap;

use clap::AppSettings;
use std::process;
use stdout::PedanticLogger;

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
        (@arg pedantic: -p --pedantic "Print technical information")
        (@subcommand show =>
            (about: "Read audio metadata")
            (@arg path: +required +hidden +takes_value +multiple "A file or directory to write to")
            (@arg filter: -f --filter +takes_value +multiple "Filter to specific tags")
            (settings: &[AppSettings::DisableVersion])
        )
    )
    .get_matches();

    if matches.is_present("pedantic") {
        PedanticLogger::setup();
    }

    let result = match matches.subcommand() {
        ("show", Some(show)) => {
            show::show(show.values_of("path").unwrap(), show.values_of("filter"))
        }

        _ => unreachable!(),
    };

    if let Err(err) = result {
        errorln!("musikr: {}", err);
        process::exit(1);
    }
}
