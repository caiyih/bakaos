use std::{
    collections::BTreeMap,
    fmt::Display,
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use source_generation::{
    ISourceGenerator, SourceGenerationContext, SourceGenerationDriver, SourceGenerationError,
};

static PLATFORM_VALIDATIONS: LazyLock<Vec<Platform>> = LazyLock::new(|| {
    vec![
        Platform {
            feature: "virt",
            arch: vec![TargetArch::RISCV64, TargetArch::LoongArch64],
        },
        Platform {
            feature: "2k1000",
            arch: vec![TargetArch::LoongArch64],
        },
        Platform {
            feature: "vf2",
            arch: vec![TargetArch::RISCV64],
        },
    ]
});

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TargetArch {
    RISCV64,
    LoongArch64,
    X86_64,
    NotSupported(String),
}

impl Display for TargetArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetArch::RISCV64 => write!(f, "riscv64"),
            TargetArch::LoongArch64 => write!(f, "loongarch64"),
            TargetArch::X86_64 => write!(f, "x86_64"),
            TargetArch::NotSupported(s) => write!(f, "NotSupported({})", s),
        }
    }
}

impl From<&str> for TargetArch {
    fn from(value: &str) -> Self {
        match value {
            "riscv64" => TargetArch::RISCV64,
            "loongarch64" => TargetArch::LoongArch64,
            "x86_64" => TargetArch::X86_64,
            _ => TargetArch::NotSupported(value.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Platform {
    feature: &'static str,
    arch: Vec<TargetArch>,
}

fn main() {
    const GENERATED_FOLDER: &str = "src/generated";

    // Force rebuild
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    println!("cargo:rustc-env=FORCE_REBUILD_TS={}", now.as_nanos());

    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");

    for feature in PLATFORM_VALIDATIONS.iter().map(|p| p.feature) {
        println!("cargo:rerun-if-env-changed={}", get_feature_env(feature));
    }

    // Currently, platform info were hardcoded into the source code
    // When it was changed to be generated, we should add a rerun trigger to the configuration files
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed={}", GENERATED_FOLDER);

    let context = SourceGenerationContext::new(GENERATED_FOLDER.into(), true);

    let driver = SourceGenerationDriver::new(vec![Box::new(FeatureCheckGenerator)]);

    driver.execute(context, false).unwrap();
}

fn get_feature_env(feature: &str) -> String {
    format!("CARGO_FEATURE_{}", feature.to_uppercase())
}

struct SourceText {
    text: String,
}

impl SourceText {
    fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn append_line(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
    }

    pub fn generate_error(&mut self, error_message: &str) {
        self.append_line(&format!(
            "compile_error!(\"{}\");",
            error_message.replace("\"", "\\\"")
        ));
        self.text.push('\n');
    }
}

fn check_target_arch(source_text: &mut SourceText) {
    let target_arch =
        std::env::var("CARGO_CFG_TARGET_ARCH").expect("Failed to get CARGO_CFG_TARGET_ARCH");

    let target_arch = target_arch.as_str().into();

    if let TargetArch::NotSupported(target_arch) = target_arch {
        source_text.generate_error(&format!("Target arch {} is not supported", target_arch));
    }
}

fn check_feature_selected(source_text: &mut SourceText) {
    let arch_to_features = PLATFORM_VALIDATIONS
        .iter()
        .flat_map(|p| p.arch.iter().map(move |arch| (arch, p.feature)))
        .fold(BTreeMap::new(), |mut map, (arch, feature)| {
            map.entry(arch).or_insert_with(|| vec![feature]);
            map
        });

    for (arch, features_list) in arch_to_features
        .iter()
        .filter(|(_, features_list)| !features_list.is_empty())
    {
        let arch = arch.to_string();

        let comment_prefix = format!("// \"{}\" Rule: ", arch);

        source_text.append_line(&format!(
            "{}Must enable one of the platform features: {}",
            comment_prefix,
            features_list.join(", ")
        ));
        source_text.append_line(&format!(
            "#[cfg(all(not(any({})), target_arch = \"{}\"))]",
            features_list
                .iter()
                .map(|f| format!("feature = \"{}\"", f))
                .collect::<Vec<_>>()
                .join(", "),
            arch,
        ));
        source_text.generate_error("No platform feature enabled, please enable one");

        for i in 0..(features_list.len() - 1) {
            for j in (i + 1)..features_list.len() {
                let (f1, f2) = (features_list[i], features_list[j]);

                source_text.append_line(&format!(
                    "{}Only one platform feature can be enabled",
                    comment_prefix
                ));
                source_text.append_line(&format!(
                    "#[cfg(all(all(feature = \"{}\", feature = \"{}\"), target_arch = \"{}\"))]",
                    f1, f2, arch,
                ));
                source_text.generate_error(&format!(
                    "Only one platform feature can be enabled. You can not enable \"{}\" and \"{}\" at the same time, please enable only one",
                    f1, f2
                ));
            }
        }
    }
}

fn check_platform_compatibility(source_text: &mut SourceText) {
    for platform in PLATFORM_VALIDATIONS.iter() {
        let arch = &platform.arch;

        match arch.len() {
            0 => {
                source_text.append_line(&format!("// The featue \"{}\" can not be enabled because there is no supported target architecture", platform.feature));
                source_text.append_line(&format!("#cfg(feature = \"{}\")", platform.feature));
                source_text.generate_error("No supported architectures found for this platform");
            }
            1 => {
                let single = arch.first().unwrap();

                source_text.append_line(&format!(
                    "// The feature \"{}\" can only be used for target arch: {}",
                    platform.feature, single
                ));
                source_text.append_line(&format!(
                    "#[cfg(all(not(target_arch = \"{}\"), feature = \"{}\"))]",
                    single, platform.feature
                ));
                source_text.generate_error(&format!(
                    "Only \"{}\" is supported for platform \"{}\"",
                    single, platform.feature
                ));
            }
            _ => {
                let supported_architectures = arch
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                source_text.append_line(&format!(
                    "// The feature \"{}\" can only be used for the following architectures: {}",
                    platform.feature, supported_architectures
                ));
                source_text.append_line(&format!(
                    "#[cfg(all(not(any({})), feature = \"{}\"))]",
                    arch.iter()
                        .map(|a| format!("target_arch = \"{}\"", a))
                        .collect::<Vec<_>>()
                        .join(", "),
                    platform.feature
                ));
                source_text.generate_error(&format!(
                    "Platform \"{}\" supports the following architectures: [{}], but you selected an unsupported one.",
                    platform.feature,
                    supported_architectures
                ));
            }
        }
    }
}

struct FeatureCheckGenerator;

impl ISourceGenerator for FeatureCheckGenerator {
    fn execute(&mut self, ctx: &mut SourceGenerationContext) -> Result<(), SourceGenerationError> {
        let mut source_text = SourceText::new();

        check_target_arch(&mut source_text);

        check_feature_selected(&mut source_text);

        check_platform_compatibility(&mut source_text);

        ctx.add_source("_compatibility_rules.rs", &source_text.text, false, true)
    }

    fn init(&mut self) {}

    fn name(&self) -> &'static str {
        "FeatureCheckGenerator"
    }
}
