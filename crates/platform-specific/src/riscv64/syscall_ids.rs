pub const SYSCALL_ID_SHUTDOWN: usize = 0;
pub const SYSCALL_ID_GETCWD: usize = 17;
pub const SYSCALL_ID_DUP: usize = 23;
pub const SYSCALL_ID_DUP3: usize = 24;
pub const SYSCALL_ID_FCNTL64: usize = 25;
pub const SYSCALL_ID_IOCTL: usize = 29;
pub const SYSCALL_ID_MKDIRAT: usize = 34;
pub const SYSCALL_ID_UNLINKAT: usize = 35;
pub const SYSCALL_ID_SYMLINKAT: usize = 36;
pub const SYSCALL_ID_LINKAT: usize = 37;
pub const SYSCALL_ID_UMOUNT: usize = 39;
pub const SYSCALL_ID_MOUNT: usize = 40;
pub const SYSCALL_ID_FTRUNCATE64: usize = 46;
pub const SYSCALL_ID_CHDIR: usize = 49;
pub const SYSCALL_ID_OPENAT: usize = 56;
pub const SYSCALL_ID_CLOSE: usize = 57;
pub const SYSCALL_ID_PIPE2: usize = 59;
pub const SYSCALL_ID_GETDENTS64: usize = 61;
pub const SYSCALL_ID_LSEEK: usize = 62;
pub const SYSCALL_ID_READ: usize = 63;
pub const SYSCALL_ID_WRITE: usize = 64;
pub const SYSCALL_ID_READV: usize = 65;
pub const SYSCALL_ID_WRITEV: usize = 66;
pub const SYSCALL_ID_PREAD: usize = 67;
pub const SYSCALL_ID_PWRITE: usize = 68;
pub const SYSCALL_ID_SENDFILE: usize = 71;
pub const SYSCALL_ID_PSELECT6: usize = 72;
pub const SYSCALL_ID_PPOLL: usize = 73;
pub const SYSCALL_ID_SPLICE: usize = 76;
pub const SYSCALL_ID_READLINKAT: usize = 78;
pub const SYSCALL_ID_NEWFSTATAT: usize = 79;
pub const SYSCALL_ID_NEWFSTAT: usize = 80;
pub const SYSCALL_ID_EXIT: usize = 93;
pub const SYSCALL_ID_EXIT_GROUP: usize = 94;
pub const SYSCALL_ID_SET_TID_ADDRESS: usize = 96;
pub const SYSCALL_ID_FUTEX: usize = 98;
pub const SYSCALL_ID_NANOSLEEP: usize = 101;
pub const SYSCALL_ID_SYSLOG: usize = 116;
pub const SYSCALL_ID_SCHED_YIELD: usize = 124;
pub const SYSCALL_ID_TIMES: usize = 153;
pub const SYSCALL_ID_UNAME: usize = 160;
pub const SYSCALL_ID_GETRUSAGE: usize = 165;
pub const SYSCALL_ID_GETTIMEOFDAY: usize = 169;
pub const SYSCALL_ID_GETPID: usize = 172;
pub const SYSCALL_ID_GETPPID: usize = 173;
pub const SYSCALL_ID_GETUID: usize = 174;
pub const SYSCALL_ID_GETEUID: usize = 175;
pub const SYSCALL_ID_GETTID: usize = 178;
pub const SYSCALL_ID_SYSINFO: usize = 179;
pub const SYSCALL_ID_SHMGET: usize = 194;
pub const SYSCALL_ID_SHMAT: usize = 196;
pub const SYSCALL_ID_SOCKET: usize = 198;
pub const SYSCALL_ID_BRK: usize = 214;
pub const SYSCALL_ID_MUNMAP: usize = 215;
pub const SYSCALL_ID_CLONE: usize = 220;
pub const SYSCALL_ID_EXECVE: usize = 221;
pub const SYSCALL_ID_MMAP: usize = 222;
pub const SYSCALL_ID_MPROTECT: usize = 226;
pub const SYSCALL_ID_WAIT4: usize = 260;
pub const SYSCALL_ID_PRLIMIT64: usize = 261;
pub const SYSCALL_ID_RENAMEAT2: usize = 276;
pub const SYSCALL_ID_GETRANDOM: usize = 278;
pub const SYSCALL_ID_COPY_FILE_RANGE: usize = 285;
pub const SYSCALL_ID_STATX: usize = 291;
pub const SYSCALL_ID_CLOCK_GETTIME: usize = 113;
