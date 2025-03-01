use alloc::boxed::Box;
use core::fmt::Debug;

#[derive(Debug)]
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
