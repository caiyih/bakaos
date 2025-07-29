use core::panic;
use source_generation::{
    ISourceGenerator, SourceGenerationContext, SourceGenerationDriver, SourceGenerationError,
    SymbolExportType,
};
use std::string::String;

const PLATFORM_FEATURES: &[&str] = &["virt", "vf2", "2k1000"];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/generated");

    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH"); // target arch

    let mut enabled_feature = None;

    for feature in PLATFORM_FEATURES {
        println!("cargo:rerun-if-env-changed={}", get_feature_env(feature));

        if is_feature_enabled(feature) {
            enabled_feature = Some(*feature);
        }
    }

    match enabled_feature {
        None => panic!("No valid platform feature enabled"),
        Some(platform) => {
            let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH")
                .expect("Failed to get CARGO_CFG_TARGET_ARCH");

            let context = SourceGenerationContext::new("src/generated".into(), true);

            let driver = SourceGenerationDriver::new(vec![Box::new(MachineImplGenerator {
                target_arch,
                platform: String::from(platform),
            })]);

            driver.execute(context, false).unwrap();
        }
    }
}

fn get_feature_env(feature: &str) -> String {
    format!("CARGO_FEATURE_{}", feature.to_uppercase())
}

fn is_feature_enabled(feature: &str) -> bool {
    std::env::var(get_feature_env(feature)).is_ok_and(|v| v == "1")
}

struct MachineImplGenerator {
    target_arch: String,
    platform: String,
}

impl ISourceGenerator for MachineImplGenerator {
    fn execute(
        &mut self,
        context: &mut SourceGenerationContext,
    ) -> Result<(), SourceGenerationError> {
        let module_name = get_platform_module_name(&self.platform);

        let source_text = format!(
            "pub use crate::{}::{}::machine_{} as machine;",
            self.target_arch, module_name, self.platform
        );

        context.add_source("_machine_export.rs", &source_text, false, true)?;
        context.register_export_symbol(
            "_machine_export::machine",
            SymbolExportType::Use { as_name: None },
            true,
        )?;

        Ok(())
    }

    fn init(&mut self) {}

    fn name(&self) -> &'static str {
        "MachineImplGenerator"
    }
}

fn get_platform_module_name(platform: &str) -> String {
    if let Some(first_char) = platform.chars().next() {
        if first_char.is_numeric() {
            return format!("_{}", platform);
        }
    }

    String::from(platform)
}
