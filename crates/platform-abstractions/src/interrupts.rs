use alloc::boxed::Box;
use core::fmt::Debug;

pub enum UserInterrupt {
    Unknown(Box<dyn Debug + Send + Sync>),
    Breakpoint,
    Syscall,
    Timer,
    SupervisorExternal,
    StorePageFault(usize),
    StoreMisaligned(usize),
    LoadPageFault(usize),
    LoadMisaligned(usize),
    InstructionPageFault(usize),
    InstructionMisaligned(usize),
    IllegalInstruction(usize),
    AccessFault(usize),
}

impl Debug for UserInterrupt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unknown(payload) => payload.fmt(f),
            Self::Breakpoint => write!(f, "Breakpoint"),
            Self::Syscall => write!(f, "Syscall"),
            Self::Timer => write!(f, "Timer"),
            Self::SupervisorExternal => write!(f, "SupervisorExternal"),
            memory_interrupt => {
                let (type_str, stval) = match memory_interrupt {
                    UserInterrupt::StorePageFault(stval) => ("StorePageFault", stval),
                    UserInterrupt::StoreMisaligned(stval) => ("StoreMisaligned", stval),
                    UserInterrupt::LoadPageFault(stval) => ("LoadPageFault", stval),
                    UserInterrupt::LoadMisaligned(stval) => ("LoadMisaligned", stval),
                    UserInterrupt::InstructionPageFault(stval) => ("InstructionPageFault", stval),
                    UserInterrupt::InstructionMisaligned(stval) => ("InstructionMisaligned", stval),
                    UserInterrupt::IllegalInstruction(stval) => ("IllegalInstruction", stval),
                    UserInterrupt::AccessFault(stval) => ("AccessFault", stval),
                    _ => unreachable!(),
                };

                match () {
                    #[cfg(target_pointer_width = "64")]
                    () => write!(f, "{}({:#018x})", type_str, stval),

                    #[cfg(target_pointer_width = "32")]
                    () => write!(f, "{}({:#010x})", type_str, stval),
                }
            }
        }
    }
}
