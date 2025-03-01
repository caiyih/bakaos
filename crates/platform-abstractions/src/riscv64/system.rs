use ::sbi_rt::{system_reset, NoReason, Shutdown, SystemFailure};
use platform_specific::legacy_println;

pub fn machine_shutdown(failure: bool) -> ! {
    match failure {
        true => system_reset(Shutdown, SystemFailure),
        false => system_reset(Shutdown, NoReason),
    };

    loop {}
}

pub fn print_bootloader_info() {
    use sbi_spec::base::impl_id;

    legacy_println!("Platform: RISC-V64");

    legacy_println!("SBI specification version: {0}", sbi_rt::get_spec_version());

    let sbi_impl = sbi_rt::get_sbi_impl_id();
    let sbi_impl = match sbi_impl {
        impl_id::BBL => "Berkley Bootloader",
        impl_id::OPEN_SBI => "OpenSBI",
        impl_id::XVISOR => "Xvisor",
        impl_id::KVM => "Kvm",
        impl_id::RUST_SBI => "RustSBI",
        impl_id::DIOSIX => "Diosix",
        impl_id::COFFER => "Coffer",
        _ => "Unknown",
    };

    legacy_println!("SBI implementation: {0}", sbi_impl);
}
