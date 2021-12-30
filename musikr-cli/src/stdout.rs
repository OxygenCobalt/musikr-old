use log::{Level, LevelFilter, Log, Metadata, Record};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

static LOGGER: PedanticLogger = PedanticLogger;

pub struct PedanticLogger;

impl PedanticLogger {
    pub fn setup() {
        log::set_logger(&LOGGER).unwrap();
        log::set_max_level(LevelFilter::Info);
    }
}

impl Log for PedanticLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        let md = record.metadata();

        if self.enabled(md) {
            let module = record.module_path().unwrap_or_default();

            let mut out = match md.level() {
                Level::Info => {
                    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

                    stdout.set_color(ColorSpec::new().set_dimmed(true)).unwrap();

                    stdout
                }

                Level::Warn => {
                    let mut stderr = StandardStream::stderr(ColorChoice::Auto);

                    stderr
                        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_intense(true))
                        .unwrap();

                    stderr
                }

                Level::Error => {
                    let mut stderr = StandardStream::stderr(ColorChoice::Auto);

                    stderr
                        .set_color(ColorSpec::new().set_fg(Some(Color::Red)))
                        .unwrap();

                    stderr
                }

                _ => StandardStream::stdout(ColorChoice::Auto),
            };

            writeln![out, "{}: {}", module, record.args()].unwrap();
            out.reset().unwrap()
        }
    }

    fn flush(&self) {}
}

#[macro_export]
macro_rules! print_header {
    ($($arg:tt)+) => {
        use std::io::Write;
        use termcolor::{StandardStream, ColorChoice, Color, ColorSpec, WriteColor};

        let mut stdout = StandardStream::stdout(ColorChoice::Auto);

        stdout.set_color(
            ColorSpec::new()
                .set_bold(true)
                .set_intense(true)
                .set_fg(Some(Color::Blue))
        ).unwrap();

        writeln![&mut stdout, $($arg)+].unwrap();

        stdout.reset().unwrap();
    };
}

#[macro_export]
macro_rules! print_entry {
    ($($arg:tt)+) => {
        use std::io::Write;
        use termcolor::{StandardStream, ColorChoice, Color, ColorSpec, WriteColor};

        let mut stdout = StandardStream::stdout(ColorChoice::Auto);

        stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Green))
        ).unwrap();

        write![&mut stdout, $($arg)+].unwrap();

        stdout.reset().unwrap();
    };
}

#[macro_export]
macro_rules! errorln {
    ($($arg:tt)+) => {
        use std::io::Write;
        use termcolor::{StandardStream, ColorChoice, Color, ColorSpec, WriteColor};

        let mut stderr = StandardStream::stderr(ColorChoice::Auto);

        stderr.set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Red))
                .set_bold(true)
        ).unwrap();

        write![&mut stderr, "error"].unwrap();
        stderr.reset().unwrap();
        write![&mut stderr, ": "].unwrap();
        writeln![&mut stderr, $($arg)+].unwrap();
    };
}
