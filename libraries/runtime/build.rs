pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={}", feature_env("std"));
    println!("cargo:rerun-if-env-changed={}", test_env());

    // see https://doc.rust-lang.org/nightly/rustc/check-cfg/cargo-specifics.html#cargorustc-check-cfg-for-buildrsbuild-script
    println!("cargo:rustc-check-cfg=cfg(runtime_std)");

    if check_env(&test_env()) || check_env(&feature_env("std")) {
        println!("cargo:rustc-cfg=runtime_std");
    }
}

fn feature_env(feature: &str) -> String {
    format!("CARGO_FEATURE_{}", feature.to_uppercase()).replace('-', "_")
}

fn test_env() -> String {
    String::from("CARGO_CFG_TEST")
}

fn check_env(var: &str) -> bool {
    std::env::var(var).is_ok()
}
