pub struct KernelStatistics {
    external_interrupts: u64,
    timer_interrupts: u64,
    software_interrupts: u64,
    kernel_exceptions: u64,
    user_exceptions: u64,
    // TODO: Should implemented in hashmap
    syscall_count: u64,
}

#[allow(unused)]
impl KernelStatistics {
    pub fn new() -> Self {
        Self {
            external_interrupts: 0,
            timer_interrupts: 0,
            software_interrupts: 0,
            kernel_exceptions: 0,
            user_exceptions: 0,
            syscall_count: 0,
        }
    }

    pub fn on_external_interrupt(&mut self) {
        self.external_interrupts += 1;
    }

    pub fn on_timer_interrupt(&mut self) {
        self.timer_interrupts += 1;
    }

    pub fn on_software_interrupt(&mut self) {
        self.software_interrupts += 1;
    }

    pub fn on_kernel_exception(&mut self) {
        self.kernel_exceptions += 1;
    }

    pub fn on_user_exception(&mut self) {
        self.user_exceptions += 1;
    }

    pub fn on_syscall(&mut self) {
        self.syscall_count += 1;
    }

    pub fn external_interrupts(&self) -> u64 {
        self.external_interrupts
    }

    pub fn timer_interrupts(&self) -> u64 {
        self.timer_interrupts
    }

    pub fn software_interrupts(&self) -> u64 {
        self.software_interrupts
    }

    pub fn kernel_exceptions(&self) -> u64 {
        self.kernel_exceptions
    }

    pub fn user_exceptions(&self) -> u64 {
        self.user_exceptions
    }

    pub fn syscall_count(&self) -> u64 {
        self.syscall_count
    }

    pub fn total_interrupts(&self) -> u64 {
        self.external_interrupts + self.timer_interrupts + self.software_interrupts
    }

    pub fn total_exceptions(&self) -> u64 {
        self.kernel_exceptions + self.user_exceptions
    }

    pub fn total_events(&self) -> u64 {
        self.total_interrupts() + self.total_exceptions()
    }
}
