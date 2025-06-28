use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    handle_linker_script_change();
}

fn handle_linker_script_change() {
    let linker_script = env::var("ARCH")
        .ok()
        .and_then(|arch| match arch.as_str() {
            "riscv64" => Some("rv64.lds"),
            "loongarch64" => Some("la64.lds"),
            _ => None,
        })
        .or_else(|| {
            // get cargo target triple
            let target = env::var("TARGET").unwrap();
            let target = target.split('-').next().unwrap();

            match target {
                "riscv64gc" => Some("rv64.lds"),
                "loongarch64" => Some("la64.lds"),
                _ => None,
            }
        });

    // linker scripts are in kernel/lds/*.ld

    match linker_script {
        Some(script) => {
            println!("cargo:rerun-if-changed=lds/{script}");
        }
        None => {
            println!("cargo:rerun-if-changed=lds");

            let linkers_dir = fs::read_dir("lds").unwrap();
            for entry in linkers_dir {
                let entry = entry.unwrap();
                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
        }
    }
}
