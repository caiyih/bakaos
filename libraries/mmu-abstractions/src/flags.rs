bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GenericMappingFlags: usize {
        const Readable = 1 << 0;
        const Writable = 1 << 1;
        const Executable = 1 << 2;
        const User = 1 << 3;
        const Kernel = 1 << 4;
        const Device = 1 << 5;
        const Uncached = 1 << 6;
    }
}
