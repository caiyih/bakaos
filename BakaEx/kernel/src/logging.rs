use log::{LevelFilter, Log};
use platform_specific::legacy_println;

static LOGGER: GlobalLogger = GlobalLogger;

pub fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("TRACE") => LevelFilter::Trace,
        Some("DEBUG") => LevelFilter::Debug,
        Some("INFO") => LevelFilter::Info,
        Some("WARN") => LevelFilter::Warn,
        Some("ERROR") => LevelFilter::Error,
        _ => LevelFilter::Off,
    });
}

struct GlobalLogger;

impl Log for GlobalLogger {
    fn enabled(&self, _m: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        legacy_println!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}
