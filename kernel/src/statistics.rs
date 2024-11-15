use core::sync::atomic::AtomicUsize;

pub struct KernelStatistics {
    external_interrupts: AtomicUsize,
    timer_interrupts: AtomicUsize,
    software_interrupts: AtomicUsize,
    kernel_exceptions: AtomicUsize,
    user_exceptions: AtomicUsize,
    // TODO: Should implemented in hashmap
    //       but we have to use a mutex then
    syscall_count: AtomicUsize,
}

#[allow(unused)]
impl KernelStatistics {
    pub fn new() -> Self {
        Self {
            external_interrupts: AtomicUsize::new(0),
            timer_interrupts: AtomicUsize::new(0),
            software_interrupts: AtomicUsize::new(0),
            kernel_exceptions: AtomicUsize::new(0),
            user_exceptions: AtomicUsize::new(0),
            syscall_count: AtomicUsize::new(0),
        }
    }

    pub fn on_external_interrupt(&mut self) -> usize {
        self.external_interrupts
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    pub fn on_timer_interrupt(&mut self) -> usize {
        self.timer_interrupts
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    pub fn on_software_interrupt(&mut self) -> usize {
        self.software_interrupts
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    pub fn on_kernel_exception(&mut self) -> usize {
        self.kernel_exceptions
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    pub fn on_user_exception(&mut self) -> usize {
        self.user_exceptions
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    pub fn on_syscall(&mut self) -> usize {
        self.syscall_count
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    pub fn external_interrupts(&self) -> usize {
        self.external_interrupts
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn timer_interrupts(&self) -> usize {
        self.timer_interrupts
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn software_interrupts(&self) -> usize {
        self.software_interrupts
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn kernel_exceptions(&self) -> usize {
        self.kernel_exceptions
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn user_exceptions(&self) -> usize {
        self.user_exceptions
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn syscall_count(&self) -> usize {
        self.syscall_count
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn total_interrupts(&self) -> usize {
        self.external_interrupts() + self.timer_interrupts() + self.software_interrupts()
    }

    pub fn total_exceptions(&self) -> usize {
        self.kernel_exceptions() + self.user_exceptions()
    }

    pub fn total_events(&self) -> usize {
        self.total_interrupts() + self.total_exceptions()
    }
}
