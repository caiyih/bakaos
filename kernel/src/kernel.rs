use log::{debug, info};

use crate::{
    platform::{self, machine},
    statistics::KernelStatistics,
    timing::current_timespec,
};

static mut KERNEL: Option<Kernel> = None;

#[allow(unused)]
pub fn get() -> &'static Kernel {
    #[allow(static_mut_refs)]
    unsafe {
        KERNEL.as_ref().unwrap()
    }
}

#[allow(static_mut_refs)]
pub fn init() {
    unsafe {
        if KERNEL.is_none() {
            KERNEL = Some(Kernel::new());

            print_banner();
            debug!("Initializing kernel");

            let kernel = get();

            debug!("Kernel initialized successfully");
            debug!("  Machine    : {}", kernel.machine().name());
            debug!("  Frequency  : {} Hz", kernel.machine().clock_freq());
            debug!("  Memory End : {:#010x}", kernel.machine().memory_end());

            for (idx, (start, len)) in kernel.machine().mmio().iter().enumerate() {
                debug!(
                    "  MMIO[{}]    : {:#010x} - {:#010x}",
                    idx,
                    start,
                    start + len
                );
            }

            debug!(
                "  Uptime     : {:.3} ms",
                current_timespec().total_seconds() * 1000.0
            );
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

pub struct Kernel {
    machine: &'static dyn machine::IMachine,
    statistics: KernelStatistics,
}

#[allow(unused)]
impl Kernel {
    pub fn new() -> Self {
        let machine = platform::get_machine_interface();

        Self {
            machine,
            statistics: KernelStatistics::new(),
        }
    }

    pub fn machine(&self) -> &dyn machine::IMachine {
        self.machine
    }

    pub fn stat(&self) -> &KernelStatistics {
        &self.statistics
    }

    pub fn up_time(&self) -> u64 {
        self.machine.machine_uptime()
    }
}
