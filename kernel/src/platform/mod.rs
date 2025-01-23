pub mod machine;
mod vf2;
mod virt;

#[allow(unreachable_code)]
pub fn get_machine_interface() -> &'static dyn machine::IMachine {
    // Virtual table is statitally allocated
    #[cfg(feature = "virt")]
    return &virt::VirtBoard;

    #[cfg(feature = "vf2")]
    return &vf2::VF2Machine;

    panic!("No machine driver is provided");
}
