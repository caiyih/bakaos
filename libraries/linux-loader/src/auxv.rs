#[repr(usize)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum AuxVecKey {
    /// End of vector
    AT_NULL = 0,
    /// Entry should be ignored
    AT_IGNORE = 1,
    /// File descriptor of program
    AT_EXECFD = 2,
    /// Program headers for program
    AT_PHDR = 3,
    /// Size of program header entry
    AT_PHENT = 4,
    /// Number of program headers
    AT_PHNUM = 5,
    /// System page size
    AT_PAGESZ = 6,
    /// Base address of interpreter
    AT_BASE = 7,
    /// Flags
    AT_FLAGS = 8,
    /// Entry point of program
    AT_ENTRY = 9,
    /// Program is not ELF
    AT_NOTELF = 10,
    /// Real uid
    AT_UID = 11,
    /// Effective uid
    AT_EUID = 12,
    /// Real gid
    AT_GID = 13,
    /// Effective gid
    AT_EGID = 14,
    /// String identifying CPU for optimizations
    AT_PLATFORM = 15,
    /// Arch dependent hints at CPU capabilities
    AT_HWCAP = 16,
    /// Frequency at which times() increments
    AT_CLKTCK = 17,
    /// Secure mode boolean
    AT_SECURE = 23,
    /// String identifying real platform, may differ from AT_PLATFORM.
    AT_BASE_PLATFORM = 24,
    /// Address of 16 random bytes
    AT_RANDOM = 25,
    /// Extension of AT_HWCAP
    AT_HWCAP2 = 26,
    /// Filename of program
    AT_EXECFN = 31,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AuxVecEntry {
    pub key: AuxVecKey,
    pub value: usize,
}

impl AuxVecEntry {
    pub const fn new(key: AuxVecKey, val: usize) -> Self {
        AuxVecEntry { key, value: val }
    }
}
