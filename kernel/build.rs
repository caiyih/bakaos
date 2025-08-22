use std::{fs, path::Path};

const LD_FILE_NAME: &str = "linker.ld";
const LDS_DIR: &str = "lds";

const PLATFORM_FEATURES: &[&str] = &["virt", "vf2", "2k1000"];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", LDS_DIR);
    println!("cargo:rerun-if-changed={}", LD_FILE_NAME);

    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH"); // target arch

    let target_arch =
        std::env::var("CARGO_CFG_TARGET_ARCH").expect("Failed to get CARGO_CFG_TARGET_ARCH");

    handle_platform_feature_change(&target_arch);

    if let Err(msg) = prepare_linker_script(&target_arch) {
        eprintln!("\x1b[31mError: {}\x1b[0m", msg);
        std::process::exit(1);
    }
}

fn handle_platform_feature_change(target_arch: &str) {
    // linker scripts are in kernel/lds/*.ld

    println!("cargo:rerun-if-changed=lds");

    let linkers_dir = fs::read_dir(LDS_DIR).unwrap();
    for entry in linkers_dir.filter_map(|e| e.ok()) {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if !file_name.ends_with(".ld") {
            continue;
        }

        let file_name = file_name.trim_end_matches(".ld");

        if !file_name.ends_with(target_arch) {
            continue;
        }

        let file_name = file_name.trim_end_matches(target_arch);

        if !file_name.ends_with('-') {
            continue;
        }

        let platform_feature = file_name.trim_end_matches('-');

        println!(
            "cargo:rerun-if-env-changed={}",
            get_feature_env(platform_feature)
        );
    }
}

fn get_feature_env(feature: &str) -> String {
    format!("CARGO_FEATURE_{}", feature.to_uppercase())
}

fn is_feature_enabled(feature: &str) -> bool {
    std::env::var(get_feature_env(feature)).is_ok_and(|v| v == "1")
}

fn prepare_linker_script(target_arch: &str) -> Result<(), String> {
    match PLATFORM_FEATURES.iter().find(|f| is_feature_enabled(f)) {
        None => Err(String::from(
            "No platform feature enabled, please enable one",
        )),
        Some(active_feature) => {
            let linker_script_name = format!("{}-{}.ld", active_feature, target_arch);
            let linker_script_path = Path::new(LDS_DIR).join(linker_script_name);

            if !linker_script_path.exists() {
                return Err(format!(
                    "Linker script not found at {}",
                    linker_script_path.display()
                ));
            }

            // copy the file to `LD_FILE_NAME`

            std::fs::copy(linker_script_path, LD_FILE_NAME)
                .map_err(|e| format!("Failed to copy linker script to {}: {}", LD_FILE_NAME, e))?;

            Ok(())
        }
    }
}
