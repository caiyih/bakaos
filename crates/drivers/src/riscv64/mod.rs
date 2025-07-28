#[cfg(feature = "virt")]
pub mod virt;

#[cfg(feature = "vf2")]
pub mod vf2;

cfg_if::cfg_if! {
    if #[cfg(feature = "virt")] {
        pub type MachineImpl = virt::VirtMachine;
    } else if #[cfg(feature = "vf2")] {
        pub type MachineImpl = vf2::VF2Machine;
    } else {
        compile_error!("No valid machine feature enabled");
    }
}
