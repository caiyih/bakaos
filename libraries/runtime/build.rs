use serde_json::Value;

pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={}", feature_env("std"));
    println!("cargo:rerun-if-env-changed={}", test_env());

    // see https://doc.rust-lang.org/nightly/rustc/check-cfg/cargo-specifics.html#cargorustc-check-cfg-for-buildrsbuild-script
    println!("cargo:rustc-check-cfg=cfg(runtime_std)");

    if (is_target_support_std() || check_env(&test_env())) && !check_env(&feature_env("no_std")) {
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

fn get_target() -> String {
    std::env::var("TARGET").unwrap_or_else(|_| String::from("unknown"))
}

fn is_target_support_std() -> bool {
    let target = get_target();

    let output = std::process::Command::new("rustc")
        .args(&[
            "--print",
            "target-spec-json",
            "--target",
            &target,
            "-Z",
            "unstable-options",
        ])
        .output()
        .expect("failed to execute process");

    let output = String::from_utf8_lossy(&output.stdout);

    let json: Value = serde_json::from_str(&output).unwrap();

    json["metadata"]["std"].as_bool().unwrap()
}
