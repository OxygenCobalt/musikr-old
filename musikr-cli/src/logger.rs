use log::{Level, LevelFilter, Log, Metadata, Record};

static LOGGER: PedanticLogger = PedanticLogger;

/// A custom logger that just works.
///
/// This implementation has basic coloring, supports all the log levels that musikr
/// uses, and doesn't require any time dependencies that add nothing but confusion.
/// Turns out avoiding dependencies is actually quite nice.
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
