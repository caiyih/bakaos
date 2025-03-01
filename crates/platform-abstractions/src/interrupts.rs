#[derive(Debug, Clone, Copy)]
pub enum UserInterrupt {
    Unknown,
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
