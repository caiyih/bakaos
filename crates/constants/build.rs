use chrono::Local;
use source_generation::{
    ISourceGenerator, SourceGenerationContext, SourceGenerationDriver, SourceGenerationError,
    SymbolExportType,
};

fn main() {
    let context = SourceGenerationContext::new("src/generated".into(), false);

    let driver = SourceGenerationDriver::new(vec![Box::new(BuildInfoGenerator)]);

    driver.execute(context, false).unwrap();
}

struct BuildInfoGenerator;

impl ISourceGenerator for BuildInfoGenerator {
    fn execute(
        &mut self,
        context: &mut SourceGenerationContext,
    ) -> Result<(), SourceGenerationError> {
        let build_time = Local::now();
        let build_time = build_time.format("%a, %d %b %Y %H:%M:%S %z").to_string();

        let source_text = format!("pub const BUILD_TIME: &str = \"{}\";", build_time.trim());

        context.add_source("build_info.rs", &source_text, false, true)?;
        context.register_export_symbol(
            "build_info::BUILD_TIME",
            SymbolExportType::Use { as_name: None },
            true,
        )?;

        Ok(())
    }

    fn init(&mut self) {}

    fn name(&self) -> &'static str {
        "BuildInfoGenerator"
    }
}
