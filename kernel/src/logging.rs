use drivers::current_timespec;
use log::{self, Level, LevelFilter, Log, Metadata, Record};

use platform_specific::legacy_println;

struct GlobalLogger;

impl Log for GlobalLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let color = match record.level() {
            Level::Error => 31, // Red
            Level::Warn => 93,  // BrightYellow
            Level::Info => 34,  // Blue
            Level::Debug => 32, // Green
            Level::Trace => 90, // BrightBlack
        };

        let time = current_timespec();
        let total_seconds = time.tv_sec;
        let hours = (total_seconds / 3600) % 24;
        let minutes = (total_seconds / 60) % 60;
        let seconds = total_seconds % 60;
        let milliseconds = time.tv_nsec / 1_000_000;

        legacy_println!(
            "\u{1B}[95m[{:02}:{:02}:{:02}.{:03}]\u{1B}[0m \u{1B}[{}m{}\u{1B}[37m | {}\u{1B}[0m",
            hours,
            minutes,
            seconds,
            milliseconds,
            color,
            normalized_loglevel(record.level()),
            record.args(),
        );
    }

    fn flush(&self) {
        // nop
    }
}

#[inline]
fn normalized_loglevel(level: Level) -> &'static str {
    match level {
        Level::Error => "ERRO",
        Level::Warn => "WARN",
        Level::Info => "INFO",
        Level::Debug => "DEBG",
        Level::Trace => "TRAC",
    }
}

static LOGGER_INSTANCE: GlobalLogger = GlobalLogger;

pub fn init() {
    legacy_println!("Initializing logging system...");

    log::set_logger(&LOGGER_INSTANCE).unwrap();

    let level = match option_env!("LOG") {
        Some("OFF") => LevelFilter::Off,
        Some("ERROR") => LevelFilter::Error,
        Some("WARN") => LevelFilter::Warn,
        Some("INFO") => LevelFilter::Info,
        Some("DEBUG") => LevelFilter::Debug,
        Some("TRACE") => LevelFilter::Trace,
        _ => {
            legacy_println!("BAKA! You forgot to input the log level. Defaulting to WARN.");
            LevelFilter::Info
        }
    };

    log::set_max_level(level);
}
