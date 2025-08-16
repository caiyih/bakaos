use crate::{SyscallContext, SyscallResult};
use address::VirtualAddress;
use constants::ErrNo;

impl SyscallContext {
    pub fn sys_uname(&self, vaddr: VirtualAddress) -> SyscallResult {
        self.sys_uname_internal(vaddr, Self::release_utsname())
    }

    fn release_utsname() -> UtsName {
        UtsName::new(
            "Linux",
            "BakaOS",
            {
                let release = option_env!("BAKAOS_RELEASE_VERSION").unwrap_or("5.11.0");

                #[cfg(target_os = "none")]
                {
                    use alloc::format;

                    &format!("{release}-BakaOS-BareMetal")
                }
                #[cfg(not(target_os = "none"))]
                {
                    &format!("{release}-BakaOS-Test-Platform")
                }
            },
            constants::BUILD_VERSION,
            constants::TARGET_ARCH,
            "localdomain",
        )
    }

    fn sys_uname_internal(&self, vaddr: VirtualAddress, utsname: UtsName) -> SyscallResult {
        self.task
            .process()
            .mmu()
            .lock()
            .export(vaddr, utsname)
            .map(|_| 0)
            .map_err(|_| ErrNo::BadAddress)
    }
}

/// Structure returned by the sys_uname system call
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UtsName {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

impl Default for UtsName {
    fn default() -> Self {
        Self {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        }
    }
}

macro_rules! utsname_prop {
    ($field:ident) => {
        #[cfg(test)]
        fn $field(&self) -> &str {
            // This is more accurate than CStr::from_bytes_until_nul semantically
            // CStr::from_bytes_until_nul cannot handle it correctly if the written field occupies the entire buffer.
            core::str::from_utf8(&self.$field)
                .unwrap()
                .trim_end_matches('\0')
        }
    };
}

impl UtsName {
    pub fn new(
        sysname: &str,
        nodename: &str,
        release: &str,
        version: &str,
        machine: &str,
        domainname: &str,
    ) -> Self {
        let mut utsname = Self::default();

        macro_rules! write_field {
            ($field:ident) => {{
                let bytes = $field.as_bytes();
                utsname.$field[..bytes.len()].copy_from_slice(bytes);
            }};
        }

        write_field!(sysname);
        write_field!(nodename);
        write_field!(release);
        write_field!(version);
        write_field!(machine);
        write_field!(domainname);

        utsname
    }

    utsname_prop!(sysname);
    utsname_prop!(nodename);
    utsname_prop!(release);
    utsname_prop!(version);
    utsname_prop!(machine);
    utsname_prop!(domainname);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use abstractions::IUsizeAlias;
    use hermit_sync::SpinMutex;
    use memory_space_abstractions::MemorySpace;
    use mmu_abstractions::IMMU;
    use test_utilities::{
        allocation::segment::TestFrameAllocator, kernel::TestKernel, task::TestProcess,
    };

    use super::*;

    fn setup_test() -> (SyscallContext, Arc<SpinMutex<dyn IMMU>>, Box<UtsName>) {
        let kernel = TestKernel::new().build();
        let (alloc, mmu) = TestFrameAllocator::new_with_mmu();
        let (_, task) = TestProcess::new()
            .with_memory_space(Some(MemorySpace::new(mmu.clone(), alloc)))
            .build();

        let utsname = Box::new(UtsName::default());

        // Map the buffer to user space
        task.process().mmu().lock().register(utsname.as_ref(), true);

        (SyscallContext::new(task, kernel), mmu, utsname)
    }

    #[test]
    fn test_uname_fields() {
        let (context, _, utsname) = setup_test();
        let result = context.sys_uname(VirtualAddress::from_ref(utsname.as_ref()));
        assert_eq!(result, Ok(0));

        assert_eq!(utsname.sysname(), "Linux");
        assert_eq!(utsname.nodename(), "BakaOS");
        #[cfg(target_os = "none")]
        assert_eq!(utsname.release(), "5.11.0-BakaOS-BareMetal");
        #[cfg(not(target_os = "none"))]
        assert_eq!(utsname.release(), "5.11.0-BakaOS-Test-Platform");
        assert_eq!(utsname.version(), constants::BUILD_VERSION);
        assert_eq!(utsname.machine(), constants::TARGET_ARCH);
        assert_eq!(utsname.domainname(), "localdomain");

        log::info!("sysname: {}", utsname.sysname());
        log::info!("nodename: {}", utsname.nodename());
        log::info!("release: {}", utsname.release());
        log::info!("version: {}", utsname.version());
        log::info!("machine: {}", utsname.machine());
        log::info!("domainname: {}", utsname.domainname());
    }

    #[test]
    fn test_buffer_can_not_write() {
        let (context, mmu, _) = setup_test();

        let utsname = UtsName::default();

        mmu.lock().register(&utsname, false);

        let result = context.sys_uname(VirtualAddress::from_ref(&utsname));
        assert_eq!(result, Err(ErrNo::BadAddress));
    }

    #[test]
    fn test_invalid_buffer() {
        let (context, _, _) = setup_test();
        let invalid_addr = VirtualAddress::from_usize(0xdeadbeef);
        let result = context.sys_uname(invalid_addr);
        assert_eq!(result, Err(ErrNo::BadAddress));
    }

    #[test]
    fn test_small_buffer() {
        let (context, mmu, _) = setup_test();

        // Create a buffer smaller than UtsName size (6*65=390 bytes)
        let small_buffer = [0u8; 10];
        mmu.lock().register(&small_buffer, true);

        let result = context.sys_uname(small_buffer.as_ref().into());

        assert_eq!(result, Err(ErrNo::BadAddress));
    }
}
