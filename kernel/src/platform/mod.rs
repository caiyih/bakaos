pub mod machine;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[allow(unreachable_code)]
pub fn get_machine_interface() -> &'static dyn machine::IMachine {
    // Virtual table is statitally allocated

    #[cfg(target_arch = "riscv64")]
    {
        #[cfg(feature = "virt")]
        return &riscv64::VirtBoard;

        #[cfg(feature = "vf2")]
        return &riscv64::VF2Machine;
    }

    panic!("No machine driver is provided");
}
