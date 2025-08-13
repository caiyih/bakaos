#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct AuxVecKey(pub usize);

pub const AT_NULL: AuxVecKey = AuxVecKey(0); // end of vector
pub const AT_IGNORE: AuxVecKey = AuxVecKey(1); // entry should be ignored
pub const AT_EXECFD: AuxVecKey = AuxVecKey(2); // file descriptor of program
pub const AT_NOTELF: AuxVecKey = AuxVecKey(10); // program is not ELF
pub const AT_PLATFORM: AuxVecKey = AuxVecKey(15); // string identifying CPU for optimizations
pub const AT_BASE_PLATFORM: AuxVecKey = AuxVecKey(24); // string identifying real platform, may differ from AT_PLATFORM.
pub const AT_HWCAP2: AuxVecKey = AuxVecKey(26); // extension of AT_HWCAP
pub const AT_EXECFN: AuxVecKey = AuxVecKey(31); // filename of program
pub const AT_PHDR: AuxVecKey = AuxVecKey(3); // program headers for program
pub const AT_PHENT: AuxVecKey = AuxVecKey(4); // size of program header entry
pub const AT_PHNUM: AuxVecKey = AuxVecKey(5); // number of program headers
pub const AT_PAGESZ: AuxVecKey = AuxVecKey(6); // system page size
pub const AT_BASE: AuxVecKey = AuxVecKey(7); // base address of interpreter
pub const AT_FLAGS: AuxVecKey = AuxVecKey(8); // flags
pub const AT_ENTRY: AuxVecKey = AuxVecKey(9); // entry point of program
pub const AT_UID: AuxVecKey = AuxVecKey(11); // real uid
pub const AT_EUID: AuxVecKey = AuxVecKey(12); // effective uid
pub const AT_GID: AuxVecKey = AuxVecKey(13); // real gid
pub const AT_EGID: AuxVecKey = AuxVecKey(14); // effective gid
pub const AT_HWCAP: AuxVecKey = AuxVecKey(16); // arch dependent hints at CPU capabilities
pub const AT_CLKTCK: AuxVecKey = AuxVecKey(17); // frequency at which times() increments
pub const AT_SECURE: AuxVecKey = AuxVecKey(23); // secure mode boolean
pub const AT_RANDOM: AuxVecKey = AuxVecKey(25); // address of 16 random bytes

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
