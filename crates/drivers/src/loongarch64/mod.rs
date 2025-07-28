#[cfg(feature = "virt")]
pub mod virt;

cfg_if::cfg_if! {
    if #[cfg(feature = "virt")] {
        pub type MachineImpl = virt::VirtMachine;
    }  else {
        compile_error!("No valid machine feature enabled");
    }
}
