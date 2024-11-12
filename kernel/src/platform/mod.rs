pub mod machine;
mod virt;

use virt::VirtBoard;

pub fn get_machine_interface() -> &'static dyn machine::IMachine {
    // Virtual table is statitally allocated
    &VirtBoard
}
