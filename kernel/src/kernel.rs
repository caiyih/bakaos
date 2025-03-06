use drivers::current_timespec;
use log::{debug, info};
use timing::TimeSpec;

use crate::statistics::KernelStatistics;

static mut KERNEL: Option<KernelMetadata> = None;

#[allow(unused)]
pub fn kernel_metadata() -> &'static KernelMetadata {
    #[allow(static_mut_refs)]
    unsafe {
        KERNEL.as_ref().unwrap()
    }
}

#[allow(static_mut_refs)]
pub fn init() {
    unsafe {
        if KERNEL.is_none() {
            KERNEL = Some(KernelMetadata::new());

            print_banner();
            debug!("Initializing kernel");

            debug!("Kernel initialized successfully");

            let machine = drivers::machine();
            debug!("  Machine    : {}", machine.name());
            debug!("  Frequency  : {} Hz", machine.query_performance_frequency());
            debug!("  Memory End : {:#010x}", machine.memory_end());

            for (idx, (start, len)) in machine.mmio().iter().enumerate() {
                debug!(
                    "  MMIO[{}]    : {:#010x} - {:#010x}",
                    idx,
                    start,
                    start + len
                );
            }

            debug!("  Uptime     : {:.3} ms", machine.machine_uptime());

            display_current_time(0);
        }
    }
}

fn print_banner() {
    info!("\u{1B}[34m⠀⠀⠀⣠⠤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣀⠀⠀");
    info!("\u{1B}[34m⠀⠀⡜⠁⠀⠈⢢⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣴⠋⠷⠶⠱⡄");
    info!("\u{1B}[34m⠀⢸⣸⣿⠀⠀⠀⠙⢦⡀⠀⠀⠀⠀⠀⠀⠀⢀⡴⠫⢀⣖⡃⢀⣸⢹");
    info!("\u{1B}[34m⠀⡇⣿⣿⣶⣤⡀⠀⠀⠙⢆⠀⠀⠀⠀⠀⣠⡪⢀⣤⣾⣿⣿⣿⣿⣸");
    info!("\u{1B}[34m⠀⡇⠛⠛⠛⢿⣿⣷⣦⣀⠀⣳⣄⠀⢠⣾⠇⣠⣾⣿⣿⣿⣿⣿⣿⣽");
    info!("\u{1B}[34m⠀⠯⣠⣠⣤⣤⣤⣭⣭⡽⠿⠾⠞⠛⠷⠧⣾⣿⣿⣯⣿⡛⣽⣿⡿⡼");
    info!("\u{1B}[34m⠀⡇⣿⣿⣿⣿⠟⠋⠁⠀⠀⠀⠀⠀⠀⠀⠀⠈⠙⠻⣿⣿⣮⡛⢿⠃");
    info!("\u{1B}[34m⠀⣧⣛⣭⡾⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⢿⣿⣷⣎⡇");
    info!("\u{1B}[34m⠀⡸⣿⡟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⢿⣷⣟⡇");
    info!("\u{1B}[34m⣜⣿⣿⡧⠀⠀⠀⠀⠀⡀⠀⠀⠀⠀⠀⠀⣄⠀⠀⠀⠀⠀⣸⣿⡜⡄");
    info!("\u{1B}[34m⠉⠉⢹⡇⠀⠀⠀⢀⣞⠡⠀⠀⠀⠀⠀⠀⡝⣦⠀⠀⠀⠀⢿⣿⣿⣹");
    info!("\u{1B}[34m⠀⠀⢸⠁⠀⠀⢠⣏⣨⣉⡃⠀⠀⠀⢀⣜⡉⢉⣇⠀⠀⠀⢹⡄⠀⠀");
    info!("\u{1B}[34m⠀⠀⡾⠄⠀⠀⢸⣾⢏⡍⡏⠑⠆⠀⢿⣻⣿⣿⣿⠀⠀⢰⠈⡇⠀⠀");
    info!("\u{1B}[34m⠀⢰⢇⢀⣆⠀⢸⠙⠾⠽⠃⠀⠀⠀⠘⠿⡿⠟⢹⠀⢀⡎⠀⡇⠀⠀");
    info!("\u{1B}[34m⠀⠘⢺⣻⡺⣦⣫⡀⠀⠀⠀⣄⣀⣀⠀⠀⠀⠀⢜⣠⣾⡙⣆⡇⠀⠀");
    info!("\u{1B}[34m⠀⠀⠀⠙⢿⡿⡝⠿⢧⡢⣠⣤⣍⣀⣤⡄⢀⣞⣿⡿⣻⣿⠞⠀⠀⠀");
    info!("\u{1B}[34m⠀⠀⠀⢠⠏⠄⠐⠀⣼⣿⣿⣿⣿⣿⣿⣿⣿⡇⠀⠳⢤⣉⢳⠀⠀⠀");
    info!("\u{1B}[34m⢀⡠⠖⠉⠀⠀⣠⠇⣿⡿⣿⡿⢹⣿⣿⣿⣿⣧⣠⡀⠀⠈⠉⢢⡀⠀");
    info!("\u{1B}[34m⢿⠀⠀⣠⠴⣋⡤⠚⠛⠛⠛⠛⠛⠛⠛⠛⠙⠛⠛⢿⣦⣄⠀⢈⡇⠀");
    info!("\u{1B}[34m⠈⢓⣤⣵⣾⠁⣀⣀⠤⣤⣀⠀⠀⠀⠀⢀⡤⠶⠤⢌⡹⠿⠷⠻⢤⡀");
    info!("\u{1B}[34m⢰⠋⠈⠉⠘⠋⠁⠀⠀⠈⠙⠳⢄⣀⡴⠉⠀⠀⠀⠀⠙⠂⠀⠀⢀⡇");
    info!("\u{1B}[34m⢸⡠⡀⠀⠒⠂⠐⠢⠀⣀⠀⠀⠀⠀⠀⢀⠤⠚⠀⠀⢸⣔⢄⠀⢾⠀");
    info!("\u{1B}[34m⠀⠑⠸⢿⠀⠀⠀⠀⢈⡗⠭⣖⡒⠒⢊⣱⠀⠀⠀⠀⢨⠟⠂⠚⠋⠀");
    info!("\u{1B}[34m⠀⠀⠀⠘⠦⣄⣀⣠⠞⠀⠀⠀⠈⠉⠉⠀⠳⠤⠤⡤⠞⠀⠀⠀⠀⠀");
}

pub struct KernelMetadata {
    statistics: KernelStatistics,
}

#[allow(unused)]
impl KernelMetadata {
    pub fn new() -> Self {
        Self {
            statistics: KernelStatistics::new(),
        }
    }

    pub fn stat(&self) -> &KernelStatistics {
        &self.statistics
    }
}

fn display_current_time(timezone_offset: i64) -> TimeSpec {
    #[inline(always)]
    fn is_leap_year(year: i64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    #[inline(always)]
    fn days_in_month(year: i64, month: u8) -> u8 {
        const DAYS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        if month == 2 && is_leap_year(year) {
            29
        } else {
            DAYS[(month - 1) as usize]
        }
    }

    let time_spec = current_timespec();

    let mut total_seconds = time_spec.tv_sec + timezone_offset * 3600;

    let seconds = (total_seconds % 60) as u8;
    total_seconds /= 60;
    let minutes = (total_seconds % 60) as u8;
    total_seconds /= 60;
    let hours = (total_seconds % 24) as u8;
    total_seconds /= 24;

    let mut year = 1970;
    while total_seconds >= if is_leap_year(year) { 366 } else { 365 } {
        total_seconds -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }

    let mut month = 1;
    while total_seconds >= days_in_month(year, month) as i64 {
        total_seconds -= days_in_month(year, month) as i64;
        month += 1;
    }

    let day = (total_seconds + 1) as u8;

    log::info!(
        "Welcome, current time is: {:04}-{:02}-{:02} {:02}:{:02}:{:02}(UTC+{:02})",
        year,
        month,
        day,
        hours,
        minutes,
        seconds,
        timezone_offset
    );

    time_spec
}
